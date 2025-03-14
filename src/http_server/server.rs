use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use once_cell::sync::Lazy;
use regex::Regex;
use strum::IntoEnumIterator;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, MutexGuard};
use url::Url;

use crate::http_server::util::extract_nth_segment_from_url;
use crate::http_server::{ONE_KB, ONE_MB};

use super::util::normalise_path;

use super::constants::HttpMethod;
use super::request::Request;
use super::response::Response;
use super::util::glob_to_regex;

/**  Async function that returns T (and can be used in multithreading env (send)).
Rust can't statically define types that return traits yet, since traits are implemented differently and have different sizes
so we must dynamically define a Future type with Box<dyn Future...>  **/
pub type AsyncFuncReturn<RetType> = Pin<Box<dyn Future<Output = RetType> + Send>>;

pub type RequestParam<'a> = MutexGuard<'a, Request>;
pub type ResponseParam<'a> = MutexGuard<'a, Response>;

// Each function is an Arc, since they must live as long as someone owns one. No need for mutex since they aren't mutable
// If an async function borrows something, that thing must live as long as the function, so for Arc that must be static or Arc.
// Request and response are scoped to the tokio::task, which will die when it dies, so we must wrap in an Arc. We mutate them, so mutex (we lock beforehand hence mutexguard).
pub type MiddlewareFunc = Arc<fn(Arc<Mutex<Request>>, Arc<Mutex<Response>>) -> AsyncFuncReturn<()>>;
type Middlewares = Vec<MiddlewareFunc>;

pub type RouteHandlerFunc = fn(Arc<Mutex<Request>>, Arc<Mutex<Response>>) -> AsyncFuncReturn<()>;
type RouteHandlers = HashMap<HttpMethod, Vec<RouteAndHandler>>;

// Lazily inits static value
static URI_PARAM_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r":[\w-]+").unwrap());

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct RouteParam {
    num_slashes_before: usize,
    name: String,
}

#[derive(Hash, PartialEq, Eq, Clone, Default, Debug)]
pub struct Route {
    method: HttpMethod,
    path: String,
    params: Vec<RouteParam>,
}

#[derive(Clone, Debug)]
struct RouteAndHandler {
    route: Route,
    handler: Arc<RouteHandlerFunc>,
}
pub struct Server {
    pub port: u32,
    pub middlewares: Middlewares,
    handlers: RouteHandlers,
}

impl Server {
    pub fn new(port: u32) -> Self {
        let mut handlers = HashMap::new();
        for method in HttpMethod::iter() {
            handlers.insert(method, Vec::new());
        }

        Server {
            port,
            middlewares: Vec::new(),
            handlers,
        }
    }

    pub async fn route(&mut self, method: HttpMethod, path: &str, handler: RouteHandlerFunc) {
        let mut norm_path = normalise_path(path);
        let mut handlers_for_method = self.handlers.get_mut(&method).unwrap();

        // Extract request params if they exist
        let mut params = Vec::new();
        for m in URI_PARAM_REGEX.find_iter(&norm_path) {
            let start = m.start();
            let before = &norm_path[..start].to_owned();
            let num_slashes_before = before.matches("/").count();
            let name = m.as_str()[1..].to_owned();
            params.push({
                RouteParam {
                    name,
                    num_slashes_before,
                }
            })
        }
        // Replace :param syntax after extraction
        norm_path = URI_PARAM_REGEX.replace_all(&norm_path, "*").to_string();
        norm_path = glob_to_regex(&norm_path);

        handlers_for_method.push(RouteAndHandler {
            route: Route {
                method,
                path: norm_path,
                params,
            },
            handler: Arc::new(handler),
        });

        // Order paths descending so more appropriate url matches match first
        handlers_for_method.sort_by(|a, b| {
            let a_num_slashes = a.route.path.matches("/").count();
            let b_num_slashes = b.route.path.matches("/").count();
            let comparison = a_num_slashes.cmp(&b_num_slashes).reverse();
            if comparison == Ordering::Equal {
                let a_true_len = a.route.path.len()
                    - (a.route.path.matches("[^/]+").count() * 5)
                    - (a.route.path.matches(".+").count() * 3);
                let b_true_len = b.route.path.len()
                    - (b.route.path.matches("[^/]+").count() * 5)
                    - (b.route.path.matches(".+").count() * 3);
                let comparison = a_true_len.cmp(&b_true_len);
                if comparison == Ordering::Equal {
                    return a.route.params.len().cmp(&b.route.params.len()).reverse();
                }
                return comparison;
            }
            comparison
        });
    }

    pub async fn add_middleware(&mut self, handler: MiddlewareFunc) {
        self.middlewares.push(handler);
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let address = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&address)
            .await
            .unwrap_or_else(|_| panic!("Could not bind TCP listener to: {}", address));

        println!("Accepting incoming connections on {}", address);

        loop {
            let (mut stream, incoming) = listener
                .accept()
                .await
                .expect("Could not accept connection");

            println!("Incoming request from {}", incoming.ip());

            let handlers = self.handlers.clone();
            let middlewares = self.middlewares.clone();
            tokio::spawn(async move {
                let request: Arc<Mutex<Request>>;
                let response = Arc::new(Mutex::new(Response::new()));

                let mut all_stream_data = Vec::new();
                loop {
                    let mut buffer: [u8; ONE_KB * 8] = [0; ONE_KB * 8];
                    let num_bytes = stream.read(&mut buffer).await;
                    all_stream_data.extend(&buffer);

                    if num_bytes.unwrap() == 0 {
                        println!("Error: End of TCP stream, probably wasn't a valid HTTP request");
                        return Err(());
                    }
                    if all_stream_data.len() > ONE_MB {
                        println!("Error: Request bigger than 1MB");
                        return Err(());
                    }

                    let request_match = Server::parse_request(&all_stream_data);
                    match request_match {
                        Ok(req) => {
                            request = Arc::new(Mutex::new(req));
                            break;
                        }
                        _ => continue, // Incomplete request
                    };
                }

                let request_method: HttpMethod;
                let request_path: String;
                {
                    let locked_request = request.lock().await;
                    request_method = locked_request.method.clone();
                    request_path = locked_request.path.clone();
                    println!(
                        "Method: {:?} --- {}",
                        locked_request.method, locked_request.path
                    );
                }

                for handler in handlers.get(&request_method).unwrap_or(&Vec::new()).iter() {
                    let pattern = Regex::new(&handler.route.path).unwrap();
                    let is_match = pattern.is_match(&request_path);
                    if is_match {
                        // Param extraction from request
                        if !handler.route.params.is_empty() {
                            for param in handler.route.params.iter() {
                                let maybe_param_value = extract_nth_segment_from_url(
                                    &request_path,
                                    param.num_slashes_before,
                                );

                                let mut locked_request = request.lock().await;
                                if let Some(param_value) = maybe_param_value {
                                    locked_request
                                        .params
                                        .insert(param.name.to_string(), param_value);
                                }
                            }
                        }

                        // Loop middlewares
                        for middleware in middlewares.iter() {
                            let maybe_response =
                                middleware(request.clone(), response.clone()).await;
                            let locked_response = response.lock().await;
                            if locked_response.should_respond() {
                                Server::return_response(locked_response, &mut stream).await;
                                return Ok(());
                            }
                        }

                        // Send response
                        let handler_func: &Arc<
                            fn(
                                Arc<Mutex<Request>>,
                                Arc<Mutex<Response>>,
                            )
                                -> Pin<Box<dyn Future<Output = ()> + Send>>,
                        > = &handler.handler;
                        let maybe_response = handler_func(request.clone(), response.clone()).await;
                        let locked_response = response.lock().await;
                        if locked_response.should_respond() {
                            Server::return_response(locked_response, &mut stream).await;
                            return Ok(());
                        }
                    }
                }
                Ok(())
            });
        }
    }

    async fn return_response(locked_response: MutexGuard<'_, Response>, stream: &mut TcpStream) {
        let response_string = format!(
            "HTTP/1.1 {} {}\r\n{}\r\n\r\n{}",
            locked_response.status_code,
            locked_response.status_text,
            locked_response
                .headers
                .iter()
                .map(|(key, value)| format!("{}: {}", key, value))
                .collect::<Vec<_>>()
                .join("\r\n"),
            locked_response.get_body_as_string(),
        );

        let _ = stream.write(response_string.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }

    fn parse_request(buffer: &[u8]) -> Result<Request, Box<dyn std::error::Error>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let res = match req.parse(buffer)? {
            httparse::Status::Complete(amt) => amt,
            httparse::Status::Partial => {
                return Err("Request is incomplete".into());
            }
        };

        let method = HttpMethod::from_str(req.method.ok_or("Method not found")?);
        let url_str = req.path.ok_or("URI not found")?.to_string();
        let version = req.version.ok_or("Version not found")?.to_string();

        let mut headers_map = HashMap::new();
        for header in req.headers.iter() {
            let name = header.name.to_string();
            let value = std::str::from_utf8(header.value)?.to_string();
            headers_map.insert(name, value);
        }

        let body = if res < buffer.len() {
            Some(buffer[res..].to_vec())
        } else {
            None
        };

        let mut url =
            Url::parse(format!("https://a.b{}", url_str).as_str()).expect("Failed to parse URL");
        let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
        url.set_query(None);

        let path = normalise_path(url.path());
        Ok(Request {
            path,
            version,
            body,
            headers: headers_map,
            method,
            params: HashMap::new(),
            query,
        })
    }
}
