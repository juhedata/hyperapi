use std::collections::HashMap;
use hyper::http::request::Parts;
use tokio::sync::oneshot;
use crate::config::{ConfigUpdate, FilterSetting, AuthSetting};
use thiserror::Error;


#[derive(Error, Debug, Clone)]
pub enum GatewayAuthError { 
    #[error("Unknown Service")]
    UnknownService,

    #[error("Unknown Client")]
    UnknownClient,

    #[error("Invalid SLA")]
    InvalidSLA,

    #[error("Invalid auth token")]
    InvalidToken,

    #[error("Auth token not found")]
    TokenNotFound,

    #[error("Unknown auth error")]
    Unknown,
}


#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub client_id: String,
    pub service_id: String,
    pub sla: String,
    pub service_filters: Vec<FilterSetting>,
    pub client_filters: Vec<FilterSetting>,
}


pub struct AuthRequest {
    pub head: Parts,
    pub result: oneshot::Sender<Result<(Parts, AuthResponse), GatewayAuthError>>,
}

impl AuthRequest {
    pub fn into_parts(self) -> (Parts, oneshot::Sender<Result<(Parts, AuthResponse), GatewayAuthError>>) {
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

    fn identify_client(&self, client: Parts, service_id: &str) -> Result<(Parts, AuthResult), GatewayAuthError>;
}

