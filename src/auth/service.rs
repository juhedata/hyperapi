use std::collections::HashMap;
use crate::config::{ConfigUpdate, FilterSetting, AuthSetting};
use tokio::sync::{mpsc, broadcast};
use tracing::{event, Level};
use crate::auth::{ServiceAuthInfo, AuthProvider, AuthRequest, AppKeyAuthProvider, JWTAuthProvider, NoAuthProvider};
use super::authenticator::{AuthResult, AuthResponse};


pub struct AuthService {
    conf_receiver: broadcast::Receiver<ConfigUpdate>,
    auth_receiver: mpsc::Receiver<AuthRequest>,

    services: HashMap<String, ServiceAuthInfo>,
    service_path: HashMap<String, String>,
    authenticators: HashMap<String, Box<dyn AuthProvider + Send + 'static>>,
}


impl AuthService {

    pub fn new(conf_receiver: broadcast::Receiver<ConfigUpdate>, auth_receiver: mpsc::Receiver<AuthRequest>) -> Self {
        AuthService {
            conf_receiver,
            auth_receiver,
            services: HashMap::new(),
            service_path: HashMap::new(),
            authenticators: HashMap::new(),
        }
    }

    pub async fn start(&mut self) {
        self.authenticators.insert(String::from("appkey"), Box::new(AppKeyAuthProvider::new()));
        self.authenticators.insert(String::from("jwt"), Box::new(JWTAuthProvider::new()));
        self.authenticators.insert(String::from("noauth"), Box::new(NoAuthProvider::new()));

        event!(Level::INFO, "auth service started");
        loop {
            tokio::select! {
                conf_update = self.conf_receiver.recv() => {
                    if let Ok(config) = conf_update {
                        self.update_config(config);
                    } else {
                        event!(Level::WARN, "failed to receive config update");
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

    pub fn update_config(&mut self, update: ConfigUpdate) {
        for (_type, provider) in self.authenticators.iter_mut() {
            provider.update_config(update.clone())
        }

        match update {
            ConfigUpdate::ServiceUpdate(s) => {
                let mut slas = HashMap::new();
                for sla in s.sla.iter() {
                    slas.insert(sla.name.clone(), sla.filters.clone());
                }
                let service = ServiceAuthInfo {
                    service_id: s.service_id.clone(),
                    auth: s.auth.clone(),
                    filters: s.filters.clone(),
                    slas: slas,
                };
                self.services.insert(s.service_id.clone(), service);
                self.service_path.insert(s.path.clone(), s.service_id.clone());
            },
            ConfigUpdate::ServiceRemove(sid) => {
                self.services.remove(&sid);
            },
            _ => {},
        }
    }

    pub async fn auth_handler(&mut self, task: AuthRequest) {
        let (head, result_channel) = task.into_parts();
        let service_path = Self::extract_service_path(&head.uri.path());
        let mut error = AuthResponse { 
            success: false, 
            error: "".into(),
            client_id: "".into(),
            service_id: "".into(),
            sla: "".into(),
            service_filters: vec![],
            client_filters: vec![],
        };
        if let Some(service_id) = self.service_path.get(&service_path) {
            if let Some(service) = self.services.get(service_id) {
                let provider = match service.auth {
                    AuthSetting::AppKey(_) => self.authenticators.get("appkey").unwrap(),
                    AuthSetting::JWT(_) => self.authenticators.get("jwt").unwrap(),
                    AuthSetting::None(_) => self.authenticators.get("noauth").unwrap(),
                };

                let (head, ar) = provider.identify_client(head, service_id);
                if let Ok(auth_result) = ar {
                    if let Some((sf, cf)) = Self::get_filters(&auth_result, service) {
                        let resp = AuthResponse {
                            success: true,
                            error: "".into(),
                            client_id: auth_result.client_id.clone(),
                            service_id: service_id.clone(),
                            sla: auth_result.sla.clone(),
                            service_filters: sf,
                            client_filters: cf,
                        };

                        let _ = result_channel.send((head, resp));
                        return
                    } else {
                        error.error = "Invalid SLA".into();
                        let _ = result_channel.send((head, error));
                        return
                    }
                } else {
                    error.error = ar.unwrap_err();
                    let _ = result_channel.send((head, error));
                    return
                }
            } else {
                error.error = format!("No service matched for service_id {}", service_id);
                let _ = result_channel.send((head, error));
                return
            }
        } else {
            error.error = format!("No service matched for path {}", service_path);
            let _ = result_channel.send((head, error));
            return
        }
    }

    fn get_filters(client: &AuthResult, service: &ServiceAuthInfo) -> Option<(Vec<FilterSetting>, Vec<FilterSetting>)> {
        if client.client_id.eq("") {  // NoAuth
            return Some((service.filters.clone(), vec![]))
        }

        if let Some(sla_filters) = service.slas.get(&client.sla) {
            Some((service.filters.clone(), sla_filters.clone()))
        } else {
            None
        }
    }

    fn extract_service_path(path: &str) -> String {
        let path = path.strip_prefix("/").unwrap_or(path);
        let (service_path, _path) = match path.find("/") {
            Some(pos) => {
                path.split_at(pos)
            },
            None => {
                (path, "")
            }
        };
        format!("/{}", service_path)
    }
   
}

