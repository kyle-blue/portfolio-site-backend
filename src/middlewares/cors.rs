use strum::IntoEnumIterator;

use crate::{
    http_server::{HttpMethod, RequestParam, ResponseParam},
    route,
};
use std::env;

route!(
    cors_middleware,
    async move |request: RequestParam, mut response: ResponseParam| {
        let default_origin = "https://www.kblue-dev.ido".to_string();
        let origin = request.headers.get("origin").unwrap_or(&default_origin);

        let environment = env::var("ENVIRONMENT").unwrap_or("prod".to_string());
        let allowed_origins = env::var("ALLOWED_ORIGINS").unwrap_or("".to_string());

        // Origin & creds header needed on pre-flight & actual requests
        if &environment == "dev" {
            response.add_header("Access-Control-Allow-Origin", origin);
        } else if allowed_origins
            .split(", ")
            .collect::<Vec<&str>>()
            .contains(&origin.as_str())
        {
            response.add_header("Access-Control-Allow-Origin", origin);
        }
        response.add_header("Access-Control-Allow-Credentials", "true");

        if request.method != HttpMethod::OPTIONS {
            return;
        }

        // Other pre flight cors headers

        let mut methods_to_allow = Vec::new();
        for method in HttpMethod::iter() {
            methods_to_allow.push(method.to_string());
        }

        response.add_header(
            "Access-Control-Allow-Methods",
            methods_to_allow.join(", ").as_str(),
        );
        response.add_header(
            "Access-Control-Allow-Headers",
            "Authorization, Content-Type",
        );

        response.send();
    }
);
