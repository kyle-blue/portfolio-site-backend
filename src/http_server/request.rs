use std::collections::HashMap;

use super::constants::HttpMethod;

#[derive(Clone)]
pub struct Request {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub version: String,
}

impl Request {
    pub fn get_body_as_string(&self) -> String {
        return String::from_utf8(self.body.clone().unwrap_or_default()).unwrap();
    }
}
