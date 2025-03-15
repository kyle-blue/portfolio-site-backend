use std::collections::HashMap;

use serde::Deserialize;

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
        let mut string = String::from_utf8(self.body.clone().unwrap_or_default()).unwrap();
        // Remove trailing NULL character caused by reading string from a buffer
        string.trim_matches(char::from(0)).to_string()
    }

    pub fn get_body_as_json<T: for<'a> Deserialize<'a>>(&self) -> Option<T> {
        let body_result = serde_json::from_str::<T>(self.get_body_as_string().as_str());
        if let Ok(json_body) = body_result {
            return Some(json_body);
        } else if let Err(e) = body_result {
            println!("{:?}", e);
        }
        None
    }
}
