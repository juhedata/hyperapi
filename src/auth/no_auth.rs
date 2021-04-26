use super::{AuthProvider, AuthResult};
use hyper::http::request::Parts;


#[derive(Debug)]
pub struct NoAuthProvider {}

impl AuthProvider for NoAuthProvider {
    fn update_config(&mut self, _update: crate::config::ConfigUpdate) {}

    fn identify_client(&self, head: Parts, _service_id: &str) -> (Parts, Result<AuthResult, String>) {
        let result = AuthResult {
            client_id: String::from(""),
            sla: String::from(""),
        };
        (head, Ok(result))
    }
}


impl NoAuthProvider {
    pub fn new() -> Self {
        NoAuthProvider {}
    }
}
