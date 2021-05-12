use super::{AuthProvider, AuthResult, authenticator::GatewayAuthError};
use hyper::http::request::Parts;


#[derive(Debug)]
pub struct NoAuthProvider {}

impl AuthProvider for NoAuthProvider {
    fn update_config(&mut self, _update: crate::config::ConfigUpdate) {}

    fn identify_client(&self, head: Parts, _service_id: &str) -> Result<(Parts, AuthResult), GatewayAuthError> {
        let result = AuthResult {
            client_id: String::from(""),
            sla: String::from(""),
        };
        Ok((head, result))
    }
}


impl NoAuthProvider {
    pub fn new() -> Self {
        NoAuthProvider {}
    }
}
