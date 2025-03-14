use super::AsyncFuncReturn;

#[macro_export]
macro_rules! route {
    ($function_name:ident, $handler_block:expr) => {
        #[allow(unused_variables)]
        pub fn $function_name(
            req: std::sync::Arc<tokio::sync::Mutex<crate::http_server::Request>>,
            res: std::sync::Arc<tokio::sync::Mutex<crate::http_server::Response>>,
        ) -> crate::http_server::AsyncFuncReturn<()> {
            return Box::pin(async move {
                let locked_request = req.lock().await;
                let locked_response = res.lock().await;
                $handler_block(locked_request, locked_response).await
            });
        }
    };
}

#[macro_export]
macro_rules! middleware {
    ($function_name:ident, $handler_block:expr) => {
        #[allow(unused_variables)]
        pub fn $function_name(
            req: std::sync::Arc<tokio::sync::Mutex<crate::http_server::Request>>,
            res: std::sync::Arc<tokio::sync::Mutex<crate::http_server::Response>>,
        ): crate::http_server::AsyncFuncReturn<()> {
            return Box::pin(async move {
                let locked_request = req.lock().await;
                let locked_response = res.lock().await;
                $handler_block(locked_request, locked_response).await
            });
        }
    };
}
