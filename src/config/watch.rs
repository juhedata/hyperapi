use hyper::{Body, Uri};
use tokio::sync::broadcast;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use super::config::{GatewayConfig, ClientInfo, ServiceInfo};


#[derive(Serialize, Deserialize, Debug)]
struct WebConfigSource {
    pub apihub: ServiceConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServiceConfig {
    pub version: i32,
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Debug, Clone)]
pub enum ConfigUpdate {
    ServiceUpdate(ServiceInfo),
    ServiceRemove(String),
    ClientUpdate(ClientInfo),
    ClientRemove(String),
}


pub struct ConfigSource {

}


impl ConfigSource {

    pub fn new(config: GatewayConfig) -> Self {
        todo!()
    }

    pub async fn watch(&mut self, updates: broadcast::Sender<ConfigUpdate>) {
        todo!()
    }
}

