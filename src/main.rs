mod http;

use std::error::Error;

use http::core::*;

route!(root_handler, {
    let mut response = Response::new();
    let body = String::from("Hello bossman");
    response.set_body(body.into_bytes());
    Some(response)
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::new(8080);
    server.route(HttpMethod::GET, "/", root_handler);
    server.start().await?;
    Ok(())
}
