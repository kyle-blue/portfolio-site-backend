use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::format::strftime::StrftimeItems;
use chrono::Utc;
use glob::Pattern;
use once_cell::sync::Lazy;
use regex::Regex;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use super::http::get_status_text;

/**
 * Make a server with:
 *    port, handlers (which allow middleware), overall middleware,
 *
 * Define routes
 *
 * Defined methods
 *
 * Defined error codes
 *
 * Main server run function
 *
 * Async thread handle function
 */

/**  Async function that returns T (and can be used in multithreading env (send)).
Rust can't statically define types that return traits yet, since traits are implemented differently and have different sizes
so we must dynamically define a Future type with Box<dyn Future...>  **/
type AsyncFuncReturn<RetType> = Pin<Box<dyn Future<Output = RetType> + Send>>;
type AsyncFunc<Args, RetType> = fn(Args) -> AsyncFuncReturn<RetType>;
type MiddlewareFunc = AsyncFunc<(Request,), Option<Response>>;
type Middleware = Arc<Vec<MiddlewareFunc>>;

pub type RouteHandlerReturn = AsyncFuncReturn<Option<Response>>;
pub type RouteHandlerFunc = AsyncFunc<Request, Option<Response>>;
type RouteHandler = Arc<RouteHandlerFunc>;

// Lazily inits static value
static URI_PARAM_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r":[\w-]+").unwrap());

#[derive(Clone)]
pub struct Request {
    method: HttpMethod,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
}

pub struct Response {
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub status_code: u16,
    status_text: String,
}
impl Response {
    pub fn new() -> Self {
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("content-type".to_owned(), "application/json".to_owned());

        let now = Utc::now();
        let format = StrftimeItems::new("%a, %d %b %Y %H:%M:%S GMT");
        headers.insert("Date".to_owned(), now.format_with_items(format).to_string());

        return Self {
            headers,
            body: None,
            status_code: 200,
            status_text: get_status_text(200).to_owned(),
        };
    }
    pub fn get_body_as_string(&self) -> String {
        return String::from_utf8(self.body.clone().unwrap_or_default()).unwrap();
    }
    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = code;
        self.status_text = get_status_text(code).to_owned();
    }
    pub fn set_body(&mut self, data: Vec<u8>) {
        self.body = Some(data);
    }
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers
            .insert(key.to_string().to_lowercase(), value.to_string());
    }
}

impl Request {
    pub fn get_body_as_string(&self) -> String {
        return String::from_utf8(self.body.clone().unwrap_or_default()).unwrap();
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct RouteParam {
    num_slashes_before: usize,
    name: String,
}

#[derive(Hash, PartialEq, Eq, Clone, Default)]
pub struct Route {
    method: HttpMethod,
    path: String,
    params: Vec<RouteParam>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, EnumIter)]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    PATCH,
    OTHER(String),
}

impl HttpMethod {
    fn from_str(s: &str) -> Self {
        match s {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            "CONNECT" => HttpMethod::CONNECT,
            "TRACE" => HttpMethod::TRACE,
            "PATCH" => HttpMethod::PATCH,
            _ => HttpMethod::OTHER(s.to_string()),
        }
    }
}

#[macro_export]
macro_rules! route {
    ($function_name:ident, $handler_block:block) => {
        #[allow(unused_variables)]
        fn $function_name(request: Request) -> RouteHandlerReturn {
            return Box::pin(async move $handler_block);
        }
    };
}

#[derive(Clone)]
struct RouteAndHandler {
    route: Route,
    handler: RouteHandler,
}
pub struct Server {
    pub port: u32,
    pub handlers: HashMap<HttpMethod, Vec<RouteAndHandler>>,
    pub middleware: Middleware,
}

impl Server {
    pub fn new(port: u32) -> Self {
        let mut handlers = HashMap::new();
        for method in HttpMethod::iter() {
            handlers.insert(method, Vec::new());
        }

        Server {
            port,
            middleware: Arc::new(Vec::new()),
            handlers,
        }
    }

    pub fn route(&mut self, method: HttpMethod, path: &str, handler: RouteHandlerFunc) {
        let route_handlers = self.handlers.get_mut(&method).unwrap();

        // Extract params if they exist
        let mut params = Vec::new();
        for m in URI_PARAM_REGEX.find_iter(&path) {
            let start = m.start();
            let before = &path[..start].to_owned();
            let num_slashes_before = before.matches("/").count();
            let name = m.as_str()[1..].to_owned();
            params.push({
                RouteParam {
                    name,
                    num_slashes_before,
                }
            })
        }
        URI_PARAM_REGEX.replace_all(path, "*");

        route_handlers.push(RouteAndHandler {
            route: Route {
                method,
                path: path.to_owned(),
                params,
            },
            handler: Arc::new(handler),
        });

        // Order paths by num /s, str len, and params
        route_handlers.sort_by(|a, b| {
            let a_num_slashes = a.route.path.matches("/").count();
            let b_num_slashes = b.route.path.matches("/").count();
            let comparison = a_num_slashes.cmp(&b_num_slashes).reverse();
            if comparison == Ordering::Equal {
                let comparison = a.route.params.len().cmp(&b.route.params.len()).reverse();
                if comparison == Ordering::Equal {
                    return a.route.params.len().cmp(&b.route.params.len()).reverse();
                }
                return comparison;
            }
            comparison
        });
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let address = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&address)
            .await
            .expect(format!("Could not bind TCP listener to: {}", address).as_str());

        println!("Accepting incoming connections on {}", address);

        loop {
            let (mut stream, incoming) = listener
                .accept()
                .await
                .expect("Could not accept connection");

            println!("Incoming request from {}", incoming.ip().to_string());

            let handlers = self.handlers.clone();

            tokio::spawn(async move {
                let mut request: Request;

                let mut all_stream_data = Vec::new();
                loop {
                    let mut buffer: [u8; 8192] = [0; 8192]; // 8kb
                    let num_bytes = stream.read(&mut buffer).await;
                    all_stream_data.extend(&buffer);

                    if num_bytes.unwrap() == 0 {
                        // End of request
                        return;
                    }

                    let request_match = parse_request(&all_stream_data);

                    request = match request_match {
                        Ok(req) => req,
                        _ => continue, // Incomplete request
                    };
                    break;
                }

                for val in handlers.get(&request.method).unwrap_or(&Vec::new()).iter() {
                    let pattern = Pattern::new(&val.route.path).unwrap();
                    let is_match = pattern.matches(&request.path);
                    if is_match {
                        if val.route.params.len() != 0 {
                            fn extract_nth_segment(url_path: &str, n: usize) -> Option<String> {
                                let pattern = format!(r"^(?:[^/]*/){{{}}}(\w+)", n); // Replace {N} dynamically
                                let regex = Regex::new(&pattern).unwrap();

                                regex.captures(url_path).map(|cap| cap[1].to_string())
                            }
                            for param in val.route.params.iter() {
                                let maybe_param_value =
                                    extract_nth_segment(&request.path, param.num_slashes_before);
                                if let Some(param_value) = maybe_param_value {
                                    request.params.insert(param.name.to_string(), param_value);
                                }
                            }
                        }

                        let maybe_response = (val.handler)(request.clone()).await;
                        match maybe_response {
                            Some(response) => {
                                return_response(response, &mut stream).await;
                                break;
                            }
                            None => {}
                        }
                    }
                }
            });
        }

        async fn return_response(response: Response, stream: &mut TcpStream) {
            let response_string = format!(
                "HTTP/1.1 {} {}\r\n{}\r\n\r\n{}",
                response.status_code,
                response.status_text,
                response
                    .headers
                    .iter()
                    .map(|(key, value)| format!("{}: {}", key, value))
                    .collect::<Vec<_>>()
                    .join("\r\n"),
                response.get_body_as_string(),
            );

            stream.write(response_string.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        }

        fn parse_request(buffer: &Vec<u8>) -> Result<Request, Box<dyn std::error::Error>> {
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);

            let res = match req.parse(&buffer)? {
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

            let mut url = Url::parse(format!("https://a.b{}", url_str).as_str())
                .expect("Failed to parse URL");
            let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
            url.set_query(None);

            Ok(Request {
                path: url.path().to_string(),
                version,
                body,
                headers: headers_map,
                method,
                params: HashMap::new(),
                query,
            })
        }
        // Create TCP listener
        // On call spawn tokio task
        //   tokio task parses tcp to html, calls all middleware and the correct handler
    }
}
