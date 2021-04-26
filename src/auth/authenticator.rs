use std::collections::HashMap;
use hyper::http::request::Parts;
use tokio::sync::oneshot;
use crate::config::{ConfigUpdate, FilterSetting, AuthSetting};


#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub success: bool,
    pub error: String,
    pub client_id: String,
    pub service_id: String,
    pub sla: String,
    pub service_filters: Vec<FilterSetting>,
    pub client_filters: Vec<FilterSetting>,
}


pub struct AuthRequest {
    pub head: Parts,
    pub result: oneshot::Sender<(Parts, AuthResponse)>,
}


impl AuthRequest {
    pub fn into_parts(self) -> (Parts, oneshot::Sender<(Parts, AuthResponse)>) {
        (self.head, self.result)
    }
}


pub struct ServiceAuthInfo {
    pub service_id: String,
    pub auth: AuthSetting,
    pub filters: Vec<FilterSetting>,
    pub slas: HashMap<String, Vec<FilterSetting>>
}


#[derive(Debug, Clone)]
pub struct AuthResult {
    pub client_id: String,
    pub sla: String,
}


pub trait AuthProvider {

    fn update_config(&mut self, update: ConfigUpdate);

    fn identify_client(&self, client: Parts, service_id: &str) -> (Parts, Result<AuthResult, String>);
}

