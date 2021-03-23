use std::{collections::HashMap, str::FromStr};
use crate::config::{ClientInfo, ConfigUpdate, FilterSetting, AuthSetting};
use tokio::sync::{mpsc, broadcast, oneshot};
use hyper::http::request::Parts;
use jsonwebtoken as jwt;
use std::time::SystemTime;
use regex::Regex;
use serde::{Serialize, Deserialize};
use serde_urlencoded;


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


struct ServiceAuthInfo {
    pub service_id: String,
    pub auth: AuthSetting,
    pub filters: Vec<FilterSetting>,
    pub slas: HashMap<String, Vec<FilterSetting>>
}


pub struct AuthService {
    conf_receiver: broadcast::Receiver<ConfigUpdate>,
    auth_receiver: mpsc::Receiver<AuthRequest>,
    services: HashMap<String, ServiceAuthInfo>,
    apps: HashMap<String, ClientInfo>,
    apps_key: HashMap<String, String>,
    service_path: HashMap<String, String>,
}


impl AuthService {

    pub fn new(conf_receiver: broadcast::Receiver<ConfigUpdate>, auth_receiver: mpsc::Receiver<AuthRequest>) -> Self {
        AuthService {
            conf_receiver,
            auth_receiver,
            services: HashMap::new(),
            apps: HashMap::new(),
            apps_key: HashMap::new(),
            service_path: HashMap::new(),
        }
    }

    pub async fn start(&mut self) {
        println!("auth service started");
        loop {
            tokio::select! {
                conf_update = self.conf_receiver.recv() => {
                    if let Ok(config) = conf_update {
                        self.update_config(config);
                    } else {
                        println!("failed to receive config update")
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
            ConfigUpdate::ClientUpdate(client) => {
                let client_id = client.client_id.clone();
                let client_key = client.app_key.clone();
                self.apps_key.insert(client_key, client_id);
                self.apps.insert(client.client_id.clone(), client);
            },
            ConfigUpdate::ClientRemove(cid) => {
                if let Some(client) = self.apps.get(&cid) {
                    self.apps_key.remove(&client.app_key);
                    self.apps.remove(&cid);
                }
            },
            _ => {},
        }
    }

    pub async fn auth_handler(&mut self, task: AuthRequest) {
        let (mut head, result_channel) = task.into_parts();
        let service_path = Self::extract_service_path(&head.uri.path());
        let mut error = String::from("");
        if let Some(service_id) = self.service_path.get(&service_path) {
            if let Some(service) = self.services.get(service_id) {
                match service.auth {
                    AuthSetting::AppKey(_) => {
                        if let Some(appkey) = Self::get_app_key(&head) {
                            //println!("appkey: {}", appkey);
                            if let Some(app_id) = self.apps_key.get(&appkey) {
                                if let Some(client) = self.apps.get(app_id) {
                                    if client.app_key == appkey {  // appkey might be updated
                                        if let Some((sla, sf, cf)) = Self::get_filters(client, service) {
                                            let resp = AuthResponse {
                                                success: true,
                                                error: error,
                                                client_id: client.client_id.clone(),
                                                service_id: service_id.clone(),
                                                sla: sla,
                                                service_filters: sf,
                                                client_filters: cf,
                                            };
                                            // replace appkey in url path
                                            let url = head.uri.to_string();
                                            let replaced = format!("/~{}/", appkey);
                                            let url = url.replace(&replaced, "/");
                                            head.uri = hyper::Uri::from_str(&url).unwrap();
                                            
                                            let _ = result_channel.send((head, resp));
                                            return
                                        } else {
                                            error = "No available SLA assigned".into();
                                        }
                                    } else {
                                        self.apps_key.remove(&appkey);
                                        error = "Invalid app-key".into();
                                    }
                                } else {
                                    error = "Invalid app-id".into();
                                }
                            } else {
                                error = "Invalid app-key".into();
                            }
                        } else {
                            error = "Failed to extract app-key".into();
                        }
                    },
                    AuthSetting::JWT(_) => {
                        if let Some(app_id) =  Self::get_jwt_app_id(&head, None) {
                            if let Some(client) = self.apps.get(&app_id) {
                                if let Some(_app_id) =  Self::get_jwt_app_id(&head, Some(client.pub_key.clone())) {
                                    if let Some((sla, sf,  cf)) = Self::get_filters(client, service) {
                                        let resp = AuthResponse {
                                            success: true,
                                            error: error,
                                            client_id: client.client_id.clone(),
                                            service_id: service.service_id.clone(),
                                            sla: sla,
                                            service_filters: sf,
                                            client_filters: cf,
                                        };
                                        let _ = result_channel.send((head, resp));
                                        return
                                    } else {
                                        error = "No available SLA assigned".into();
                                    }
                                } else {
                                    error = "Invalid JWT signature".into();
                                }
                            } else {
                                error = "Invalid app-id".into();
                            }
                        } else {
                            error = "Invalid JWT payload".into();
                        }
                    },
                }
            } else {
                error = format!("No service matched for service_id {}", service_id);
            }
        } else {
            error = format!("No service matched for path {}", service_path);
        }

        // no match, return error
        let _ = result_channel.send((head, AuthResponse { 
            success: false, 
            error: error,
            client_id: "".into(),
            service_id: "".into(),
            sla: "".into(),
            service_filters: vec![],
            client_filters: vec![],
        }));
    }

    fn get_filters(client: &ClientInfo, service: &ServiceAuthInfo) -> Option<(String, Vec<FilterSetting>, Vec<FilterSetting>)> {
        if let Some(sla) = client.services.get(&service.service_id) {
            if let Some(sla_filters) = service.slas.get(sla) {
                return Some((sla.clone(), service.filters.clone(), sla_filters.clone()))
            }
        }
        None
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
    
    fn get_app_key(head: &Parts) -> Option<String> {
        // find in authorization header
        if let Some(token) = head.headers.get("X-APP-KEY") {  
            if let Ok(token_str) = token.to_str() {
                return Some(String::from(token_str));
            }
        }

        // find in url query
        if let Some(query) = head.uri.query() {
            let query_pairs = serde_urlencoded::from_str::<Vec<(String, String)>>(query);
            if let Ok(pairs) = query_pairs {
                for (k, v) in pairs {
                    if k.eq("_app_key") {
                        return Some(v);
                    }
                }
            } else {
                println!("{:?}", query_pairs);
            }
        }

        // find in url path
        let pattern = Regex::new(r"^/.+?/~(.+?)/").unwrap();
        if let Some(appkey_match) = pattern.captures(head.uri.path()) {
            if let Some(am) = appkey_match.get(1) {
                return Some(String::from(am.as_str()))
            }
        }

        None
    }

    fn get_jwt_app_id(head: &Parts, pubkey: Option<String>) -> Option<String> {
        let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        if let Some(token) = head.headers.get(hyper::header::AUTHORIZATION) {  // find in authorization header
            let segs: Vec<&str> = token.to_str().unwrap().split(' ').collect();
            let token = *(segs.get(1).unwrap_or(&""));
            let t = {
                if let Some(key) = pubkey {
                    let verifier = jwt::Validation::new(jwt::Algorithm::ES256);
                    let verify_key = jwt::DecodingKey::from_ec_pem(key.as_bytes());
                    if let Ok(vk) = verify_key {
                        jwt::decode::<JwtClaims>(&token, &vk, &verifier)
                    } else {
                        Err(jwt::errors::ErrorKind::InvalidEcdsaKey.into())
                    }
                } else {
                    jwt::dangerous_insecure_decode::<JwtClaims>(token)
                }
            };
            if let Ok(td) = t {
                if td.claims.exp > ts.as_secs() {
                    return Some(td.claims.sub);
                }
            } else {
                println!("{:?}", t);
            }
        }
        None
    }

}


#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub exp: u64,                    // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: Option<u64>,            // Optional. Issued at (as UTC timestamp)
    pub iss: Option<String>,         // Optional. Issuer
    pub sub: String,                 // Optional. Subject (whom token refers to)
}
