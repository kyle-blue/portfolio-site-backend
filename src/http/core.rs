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
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

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
    uri: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
}

pub struct Response {
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}
impl Response {
    fn new() -> Self {
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".to_owned(), "application/json".to_owned());

        let now = Utc::now();
        let format = StrftimeItems::new("%a, %d %b %Y %H:%M:%S GMT");
        headers.insert("Date".to_owned(), now.format_with_items(format).to_string());

        return Self {
            headers,
            body: None,
        };
    }
}

impl Request {
    fn get_body_as_string(&self) -> String {
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
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
        Server {
            port,
            middleware: Arc::new(Vec::new()),
            handlers: HashMap::new(),
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

        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("Could not accept connection");

            let handlers = self.handlers.clone();

            tokio::spawn(async move {
                let mut request: Request;

                let mut all_stream_data = Vec::new();
                loop {
                    let mut buffer: [u8; 8192] = [0; 8192]; // 8kb
                    let _ = stream.read(&mut buffer);
                    all_stream_data.extend(&buffer);

                    let request_match: Result<Request, ()> = parse_request(&all_stream_data);

                    request = match request_match {
                        Ok(req) => req,
                        Err(()) => continue, // Incomplete request
                    };
                    break;
                }

                // blah/*   blah/:id/

                for val in handlers.get(&request.method).unwrap_or(&Vec::new()).iter() {
                    let pattern = Pattern::new(&val.route.path).unwrap();
                    let is_match = pattern.matches(&request.uri);
                    if is_match {
                        if val.route.params.len() != 0 {
                            fn extract_nth_segment(url: &str, n: usize) -> Option<String> {
                                let pattern = format!(r"^(?:[^/]*/){{{}}}(\w+)", n); // Replace {N} dynamically
                                let regex = Regex::new(&pattern).unwrap();

                                regex.captures(url).map(|cap| cap[1].to_string())
                                // Extract match
                            }
                            for param in val.route.params.iter() {
                                extract_nth_segment(&request.uri, param.num_slashes_before);
                            }
                        }

                        let maybe_response = (val.handler)(request.clone()).await;
                        match maybe_response {
                            Some(response) => {
                                return_response(response);
                                break;
                            }
                            None => {}
                        }
                    }
                }
            });
        }

        fn return_response(response: Response) {}

        fn parse_request(buffer: &Vec<u8>) -> Result<Request, ()> {
            return Err(());
        }

        // Create TCP listener
        // On call spawn tokio task
        //   tokio task parses tcp to html, calls all middleware and the correct handler
    }
}
