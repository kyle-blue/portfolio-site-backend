mod http_server;

use http_server::*;
use std::error::Error;

route!(root_handler, |request: Request| {
    let mut response = Response::new();
    let body = String::from("Hello bossman");
    response.set_body(body.into_bytes());
    Some(response)
});

route!(id_handler, |request: Request| {
    let mut response = Response::new();
    let id = request.params.get("id").unwrap();
    let body = format!("{{\"id\": \"{id}\"}}");
    response.set_body(body.into_bytes());
    Some(response)
});

route!(catch_all_other_handler, |request: Request| {
    let mut response = Response::new();
    let body = format!("{}", request.path);
    response.set_body(body.into_bytes());
    Some(response)
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::new(8080);
    server.route(HttpMethod::GET, "/", root_handler);
    server.route(HttpMethod::GET, "/:id", id_handler);
    server.route(HttpMethod::GET, "/**", catch_all_other_handler);
    server.start().await?;
    Ok(())
}
