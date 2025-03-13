#[macro_export]
macro_rules! route {
    ($function_name:ident, $handler_block:expr) => {
        #[allow(unused_variables)]
        pub fn $function_name(
            request: crate::http_server::Request,
        ) -> crate::http_server::RouteHandlerReturn {
            return Box::pin($handler_block(request));
        }
    };
}
