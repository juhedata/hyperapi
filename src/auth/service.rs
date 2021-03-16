use std::collections::HashMap;
use crate::config::{ClientInfo, ConfigUpdate, FilterSetting, AuthSetting};
use tokio::sync::{mpsc, broadcast, oneshot};
use hyper::http::request::Parts;
use jsonwebtoken as jwt;
use std::time::SystemTime;
use regex::Regex;
use serde::{Serialize, Deserialize};
use base64;


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
                    } else {
                        println!("failed to receive auth request")
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
        let (head, result_channel) = task.into_parts();
        let service_path = Self::extract_service_path(&head.uri.path());
        if let Some(service_id) = self.service_path.get(&service_path) {
            if let Some(service) = self.services.get(service_id) {
                match service.auth {
                    AuthSetting::AppKey(_) => {
                        if let Some(appkey) = Self::get_app_key(&head) {
                            if let Some(app_id) = self.apps_key.get(&appkey) {
                                if let Some(client) = self.apps.get(app_id) {
                                    if let Some((sla, sf, cf)) = Self::get_filters(client, service) {
                                        let resp = AuthResponse {
                                            success: true,
                                            error: String::from(""),
                                            client_id: client.client_id.clone(),
                                            service_id: service_id.clone(),
                                            sla: sla,
                                            service_filters: sf,
                                            client_filters: cf,
                                        };
                                        let _ = result_channel.send((head, resp));
                                        return
                                    }
                                }
                            }
                        }
                    },
                    AuthSetting::JWT(_) => {
                        if let Some(app_id) =  Self::get_jwt_app_id(&head, None) {
                            if let Some(client) = self.apps.get(&app_id) {
                                if let Some(_) =  Self::get_jwt_app_id(&head, Some(client.app_key.clone())) {
                                    if let Some((sla, sf,  cf)) = Self::get_filters(client, service) {
                                        let resp = AuthResponse {
                                            success: true,
                                            error: String::from(""),
                                            client_id: client.client_id.clone(),
                                            service_id: service.service_id.clone(),
                                            sla: sla,
                                            service_filters: sf,
                                            client_filters: cf,
                                        };
                                        let _ = result_channel.send((head, resp));
                                        return
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }

        // no match, return error
        let _ = result_channel.send((head, AuthResponse { 
            success: false, 
            error: "Auth failed".into(), 
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
        let path = path.strip_prefix("/").unwrap();
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
            let token_str = token.to_str().unwrap();
            return Some(String::from(token_str));
        } 
        // find in url query
        let url = url::Url::parse(&head.uri.to_string()).unwrap();
        for (k, v) in url.query_pairs() {
            if k.eq("_app_key") {
                return Some(String::from(v));
            }
        }
        // find in url path
        let pattern = Regex::new(r"^\/.+?\/~(.+?)\/").unwrap();
        if let Some(appkey_match) = pattern.captures(url.path()) {
            if let Some(am) = appkey_match.get(1) {
                return Some(String::from(am.as_str()))
            }
        }
        None
    }

    fn get_jwt_app_id(head: &Parts, appkey: Option<String>) -> Option<String> {
        let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        if let Some(token) = head.headers.get(hyper::header::AUTHORIZATION) {  // find in authorization header
            let segs: Vec<&str> = token.to_str().unwrap().split(' ').collect();
            let token = *(segs.get(1).unwrap_or(&""));
            let t = {
                if let Some(key) = appkey {
                    let verifier = jwt::Validation::new(jwt::Algorithm::ES256);
                    let pubkey = base64::decode_config(key, base64::URL_SAFE).unwrap();
                    let verify_key = jwt::DecodingKey::from_ec_der(pubkey.as_slice());
                    jwt::decode::<JwtClaims>(&token, &verify_key, &verifier)
                } else {
                    jwt::dangerous_insecure_decode::<JwtClaims>(token)
                }
            };
            if let Ok(td) = t {
                if td.claims.exp > ts.as_secs() {
                    return Some(td.claims.sub);
                }
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
