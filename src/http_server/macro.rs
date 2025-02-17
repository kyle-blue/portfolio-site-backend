#[macro_export]
macro_rules! route {
    ($function_name:ident, $handler_block:expr) => {
        #[allow(unused_variables)]
        fn $function_name(request: Request) -> RouteHandlerReturn {
            return Box::pin(async move { $handler_block(request) });
        }
    };
}
