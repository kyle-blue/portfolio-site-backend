mod api;
mod http_server;
mod middlewares;

use http_server::*;
use middlewares::cors_middleware;
use std::env;
use std::error::Error;

fn env_var_check() {
    let required_envs = [
        "ENVIRONMENT",
        "EMAIL_ADDRESS",
        "EMAIL_PASSWORD",
        "ALLOWED_ORIGINS",
    ];
    let mut missing_envs = Vec::new();

    for env_str in required_envs {
        let env_var = env::var(env_str);
        if env_var.is_err() {
            missing_envs.push(env_str);
        }
    }
    if !missing_envs.is_empty() {
        panic!("Missing environment variables: {:?}", missing_envs);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_var_check();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let mut server = Server::new(8080);
    server.add_middleware(cors_middleware);
    server.route(
        HttpMethod::POST,
        "/api/v1/send_email",
        api::v1::send_email_handler,
    );

    server.start().await?;

    Ok(())
}
