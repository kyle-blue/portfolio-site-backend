use std::collections::HashMap;

use chrono::format::strftime::StrftimeItems;
use chrono::Utc;

use super::constants::get_status_text;

pub struct Response {
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub status_code: u16,
    pub status_text: String,
    _should_respond: bool,
}
impl Response {
    // PUBLIC
    pub fn new() -> Self {
        Self {
            headers: Response::get_default_headers(),
            body: None,
            status_code: 200,
            status_text: get_status_text(200).to_owned(),
            _should_respond: false,
        }
    }
    pub fn get_body_as_string(&self) -> String {
        String::from_utf8(self.body.clone().unwrap_or_default()).unwrap()
    }
    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = code;
        self.status_text = get_status_text(code).to_owned();
    }
    pub fn set_body(&mut self, data: Vec<u8>) {
        self.body = Some(data);
    }
    pub fn set_body_string(&mut self, data: String) {
        self.body = Some(data.into_bytes());
    }
    pub fn set_body_str(&mut self, data: &str) {
        self.body = Some(data.as_bytes().to_vec());
    }
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers
            .insert(key.to_string().to_lowercase(), value.to_string());
    }
    // Send the response and stop propogating routes / middleware
    pub fn send(&mut self) {
        self._should_respond = true;
    }
    pub fn should_respond(&self) -> bool {
        return self._should_respond;
    }

    // PRIVATE
    fn get_default_headers() -> HashMap<String, String> {
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("content-type".to_owned(), "application/json".to_owned());

        let now = Utc::now();
        let format = StrftimeItems::new("%a, %d %b %Y %H:%M:%S GMT");
        headers.insert("Date".to_owned(), now.format_with_items(format).to_string());
        headers
    }
}
