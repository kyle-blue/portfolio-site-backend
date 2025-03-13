mod api;
mod http_server;

use http_server::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let mut server = Server::new(8080);
    server.route(
        HttpMethod::POST,
        "/api/v1/send_email",
        api::v1::send_email_handler,
    );
    server.start().await?;

    Ok(())
}
