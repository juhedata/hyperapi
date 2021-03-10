use std::collections::HashMap;
use crate::config::{ClientInfo, ConfigUpdate, FilterSetting, AuthSetting};
use tokio::sync::{mpsc, broadcast, oneshot};
use hyper::http::request::Parts;
use jsonwebtoken as jwt;
use std::time::SystemTime;
use regex::Regex;
use serde::{Serialize, Deserialize};


pub struct AuthResponse {
    pub success: bool,
    pub error: String,
    pub head: Parts,
    pub client_id: String,
    pub service_id: String,
    pub filters: Vec<FilterSetting>,
}


pub struct AuthRequest {
    pub head: Parts,
    pub result: oneshot::Sender<AuthResponse>,
}

impl AuthRequest {
    pub fn into_parts(self) -> (Parts, oneshot::Sender<AuthResponse>) {
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
    apps_id: HashMap<String, ClientInfo>,
    apps_key: HashMap<String, String>,
}


impl AuthService {

    pub fn new(conf_receiver: broadcast::Receiver<ConfigUpdate>, auth_receiver: mpsc::Receiver<AuthRequest>) -> Self {
        AuthService {
            conf_receiver,
            auth_receiver,
            services: HashMap::new(),
            apps_id: HashMap::new(),
            apps_key: HashMap::new(),
        }
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

    pub fn update_config(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(s) => {
                let slas = HashMap::new();
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
            },
            ConfigUpdate::ServiceRemove(sid) => {
                self.services.remove(&sid);
            },
            ConfigUpdate::ClientUpdate(client) => {
                let client_id = client.client_id.clone();
                let client_key = client.app_key.clone();
                self.apps_key.insert(client_key, client_id);
                self.apps_id.insert(client.client_id.clone(), client);
            },
            ConfigUpdate::ClientRemove(cid) => {
                if let Some(client) = self.apps_id.get(&cid) {
                    self.apps_key.remove(&client.app_key);
                    self.apps_id.remove(&cid);
                }
            },
        }
    }

    pub async fn auth_handler(&mut self, task: AuthRequest) {
        let (head, result_channel) = task.into_parts();
        let service_id = Self::extract_service_id(&head.uri.path());
        if let Some(service) = self.services.get(&service_id) {
            match service.auth {
                AuthSetting::AppKey(_) => {
                    if let Some(appkey) = Self::get_app_key(&head) {
                        if let Some(app_id) = self.apps_key.get(&appkey) {
                            if let Some(client) = self.apps_id.get(app_id) {
                                if let Some(filters) = Self::get_filters(client, service) {
                                    let resp = AuthResponse {
                                        success: true,
                                        error: String::from(""),
                                        head: head,
                                        client_id: client.client_id.clone(),
                                        service_id: service.service_id.clone(), 
                                        filters: filters,
                                    };
                                    result_channel.send(resp);
                                    return
                                }
                            }
                        }
                    }
                },
                AuthSetting::JWT(_) => {
                    if let Some(app_id) =  Self::get_jwt_app_id(&head, None) {
                        if let Some(client) = self.apps_id.get(&app_id) {
                            if let Some(app_id) =  Self::get_jwt_app_id(&head, Some(client.app_key.clone())) {
                                if let Some(filters) = Self::get_filters(client, service) {
                                    let resp = AuthResponse {
                                        success: true,
                                        error: String::from(""),
                                        head: head,
                                        client_id: client.client_id.clone(),
                                        service_id: service.service_id.clone(), 
                                        filters: filters,
                                    };
                                    result_channel.send(resp);
                                    return
                                }
                            }
                        }
                    }
                },
            }
        }
        // no match, return error
        result_channel.send(AuthResponse { 
            success: false, 
            error: "Auth failed".into(), 
            head: head,
            client_id: "".into(),
            service_id: service_id, 
            filters: vec![],
        });
    }

    fn get_filters(client: &ClientInfo, service: &ServiceAuthInfo) -> Option<Vec<FilterSetting>> {
        if let Some(sla) = client.services.get(&service.service_id) {
            if let Some(sla_filters) = service.slas.get(sla) {
                return Some([service.filters.clone(), sla_filters.clone()].concat())
            }
        }
        None
    }

    fn extract_service_id(path: &str) -> String {
        let path = path.strip_prefix("/").unwrap();
        let (service_id, _path) = match path.find("/") {
            Some(pos) => {
                path.split_at(pos)
            },
            None => {
                (path, "")
            }
        };
        String::from(service_id)
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
                    let v = jwt::Validation::new(jwt::Algorithm::ES256);
                    let vkey = key;
                    jwt::decode::<JwtClaims>(&token, &jwt::DecodingKey::from_secret(vkey.as_bytes()), &v)
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
struct JwtClaims {
    pub aud: Option<String>,         // Optional. Audience
    pub exp: u64,                  // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: Option<usize>,          // Optional. Issued at (as UTC timestamp)
    pub iss: Option<String>,         // Optional. Issuer
    pub nbf: Option<usize>,          // Optional. Not Before (as UTC timestamp)
    pub sub: String,                 // Optional. Subject (whom token refers to)
}
