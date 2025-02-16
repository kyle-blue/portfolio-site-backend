mod http;

use std::error::Error;

use http::core::*;

route!(root_handler, { None });

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::new(8080);
    server.route(HttpMethod::GET, "/", root_handler);
    server.start().await?;
    Ok(())
}
