use std::env;

use crate::http_server::{RequestParam, ResponseParam};
use crate::route;

use mail_send::mail_builder::MessageBuilder;
use mail_send::{SmtpClient, SmtpClientBuilder};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

#[derive(Debug, Serialize, Deserialize)]
struct EmailInfo {
    name: String,
    email: String,
    message: String,
}

fn get_client_email_message<'a>(
    name: &'a str,
    message: &'a str,
    email_address: &'a str,
) -> MessageBuilder<'a> {
    let body =  format!("
        <style>
            body {{margin: 0}};
        </style>
        <div style=\"margin: 0; padding: 0.2rem 1rem; width: 100%; background: rgb(138, 121, 173); background: linear-gradient(180deg, rgba(138, 121, 173, 1) 0%, rgba(174, 130, 181, 1) 100%);\">
            <h1 style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; color: #ffffff\">Hello {}!</h1>
        </div>
        <div style=\"padding: 0.2rem 1rem\">
            <h2 style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">I have recieved your message:</h2>
            <p style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; text-indent: 1rem; white-space: pre-wrap; font-style: italic;\">{}</p>
            <p style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">I will reply at my earliest convenience through my personal email address (kyle.blue.nuttall@gmail.com) to the email address you provided ({}).</p>
            <p style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">Thanks!</p>
            <h3 style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">Kyle Doidge - kblue.io</h3>
        </div>
    ", name, message, email_address);

    MessageBuilder::new()
        .to(vec![("", email_address)])
        .subject("Thank you for your message! - kblue.io")
        .html_body(body)
}

fn get_my_email_message<'a>(
    name: &'a str,
    message: &'a str,
    email_address: &'a str,
) -> MessageBuilder<'a> {
    let body =  format!("
        <style>
            body {{margin: 0}};
        </style>
        <div style=\"margin: 0; padding: 0.2rem 1rem; width: 100%; background: rgb(138, 121, 173); background: linear-gradient(180deg, rgba(138, 121, 173, 1) 0%, rgba(174, 130, 181, 1) 100%);\">
            <h1 style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; color: #ffffff\">You have a message from {}!</h1>
        </div>
        <div style=\"padding: 0.2rem 1rem\">
            <h2 style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">He says:</h2>
            <p style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; text-indent: 1rem; white-space: pre-wrap; font-style: italic;\">{}</p>
            <p style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">Reply to his email here: {}</p>
            <p style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">Thanks!</p>
            <h3 style=\"font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\">kblue bot</h3>
        </div>
    ", name, message, email_address);

    MessageBuilder::new()
        .to(vec![("", "kyle.blue.nuttall@gmail.com")])
        .subject(format!(
            "{} - {} sent you a message on kblue.io!",
            name, email_address
        ))
        .html_body(body)
}

async fn create_smtp_client() -> SmtpClient<TlsStream<TcpStream>> {
    let email_password = env::var("EMAIL_PASSWORD").unwrap();

    SmtpClientBuilder::new("smtp.gmail.com", 587)
        .implicit_tls(false)
        .credentials(("kyle.blue.doidge.bot@gmail.com", email_password.as_str()))
        .connect()
        .await
        .unwrap()
}

route!(
    send_email_handler,
    async move |request: RequestParam, mut response: ResponseParam| {
        let maybe_email_info: Option<EmailInfo> = request.get_body_as_json();
        if let Some(email_info) = maybe_email_info {
            let bot_email = env::var("EMAIL_ADDRESS").unwrap();
            let mut smtp_client = create_smtp_client().await;
            let message =
                get_client_email_message(&email_info.name, &email_info.message, &email_info.email)
                    .from(("Kyle Doidge", bot_email.as_str()));
            let result1 = smtp_client.send(message).await;
            let message =
                get_my_email_message(&email_info.name, &email_info.message, &email_info.email)
                    .from(("KBlue Bot", bot_email.as_str()));

            let result2 = smtp_client.send(message).await;

            if result1.is_ok() && result2.is_ok() {
                response.set_body_str("{\"message\": \"success\"}");
            } else {
                response.set_body_str("{\"message\": \"could not successfully send emails\"}");
                response.set_status_code(500);
            }
        } else {
            response.set_body_str("{\"message\": \"could not deserialise json body\"}");
            response.set_status_code(400);
        }
        response.send();
    }
);
