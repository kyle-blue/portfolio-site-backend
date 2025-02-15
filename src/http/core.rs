use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
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
type MiddlewareFunc = AsyncFunc<(Request,), Propogation>;
type Middleware = Arc<Vec<MiddlewareFunc>>;

pub type RouteHandlerReturn = AsyncFuncReturn<Propogation>;
pub type RouteHandlerFunc = AsyncFunc<Request, Propogation>;
type RouteHandler = Arc<RouteHandlerFunc>;

pub enum Propogation {
    Stop,
    Continue,
}

pub struct Request {
    method: HttpMethod,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl Request {
    fn get_body_as_string(&self) -> String {
        return String::from_utf8(self.body.clone().unwrap_or_default()).unwrap();
    }
}

#[derive(Hash, PartialEq, Eq)]
pub struct Route {
    method: HttpMethod,
    path: String,
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
pub struct Server {
    pub port: u32,
    pub handlers: HashMap<Route, RouteHandler>,
    pub middleware: Middleware,
}

impl Server {
    pub fn route(&self, method: HttpMethod, path: &str, handler: RouteHandlerFunc) {}
    pub async fn start(&self) {}
}
