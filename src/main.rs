mod http;
use std::{collections::HashMap, sync::Arc};

use http::core::*;

route!(root_handler, { Propogation::Stop });

#[tokio::main]
async fn main() {
    let server = Server {
        port: 8080,
        middleware: Arc::new(Vec::new()),
        handlers: HashMap::new(),
    };

    server.route(HttpMethod::GET, "/", root_handler);

    server.start().await;
}
