use std::collections::HashMap;
use crate::config::{ClientInfo, ConfigUpdate, FilterSetting, ServiceInfo};
use tokio::sync::{mpsc, broadcast, oneshot};
use hyper::http::request::Parts;


pub struct AuthResponse {
    pub head: Parts,
    pub client_id: String,
    pub service_id: String,
    pub filters: Vec<FilterSetting>,
}


pub struct AuthRequest {
    pub head: Parts,
    pub result: oneshot::Sender<AuthResponse>,
}


pub struct AuthService {
    conf_receiver: broadcast::Receiver<ConfigUpdate>,
    auth_receiver: mpsc::Receiver<AuthRequest>,
    services: HashMap<String, ServiceInfo>,
    apps: HashMap<String, ClientInfo>,
}


impl AuthService {

    pub fn new(conf_receiver: broadcast::Receiver<ConfigUpdate>, auth_receiver: mpsc::Receiver<AuthRequest>) -> Self {
        AuthService {
            conf_receiver,
            auth_receiver,
            services: HashMap::new(),
            apps: HashMap::new(),
        }
    }

    pub fn update_config(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(service) => {
                
            },
            ConfigUpdate::ServiceRemove(sid) => {

            },
            ConfigUpdate::ClientUpdate(client) => {

            },
            ConfigUpdate::ClientRemove(cid) => {
                
            },
        }
    }

    pub async fn auth_handler(&mut self, task: AuthRequest) {
        todo!()
    }

    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                conf_update = self.conf_receiver.recv() => {
                    if let Ok(config) = conf_update {
                        self.update_config(config);
                    }
                },
                auth_request = self.auth_receiver.recv() => {
                    if let Some(req) = auth_request {
                        self.auth_handler(req).await;
                    }
                },
            }
        }
    }
}

