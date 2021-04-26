use std::{collections::HashMap, sync::Mutex, time::SystemTime};
use super::{AuthProvider, AuthResult};
use hyper::http::request::Parts;
use crate::config::{ClientInfo, ConfigUpdate};
use jsonwebtoken as jwt;
use serde::{Serialize, Deserialize};
use tracing::{event, Level};
use lru::LruCache;

#[derive(Debug)]
pub struct JWTAuthProvider {
    apps: HashMap<String, ClientInfo>,
    token_cache: Mutex<LruCache<String, String>>,
}

impl AuthProvider for JWTAuthProvider {
    fn update_config(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ClientUpdate(client) => {
                let client_id = client.client_id.clone();
                self.apps.insert(client_id, client);
            },
            ConfigUpdate::ClientRemove(cid) => {
                self.apps.remove(&cid);
            },
            _ => {},
        }
    }

    fn identify_client(&self, head: Parts, service_id: &str) -> (Parts, Result<AuthResult, String>) {
        if let Some(token) =  Self::extract_token(&head) {
            if let Some(client_id) = Self::extract_client_id(&token) {
                if let Some(client) =  self.apps.get(&client_id) {
                    if let Some(sla) = client.services.get(service_id) {
                        // check cache
                        let mut cache = self.token_cache.lock().unwrap();
                        if let Some(cached_key) = cache.get(&token) {
                            event!(Level::DEBUG, "cached data {} {}", cached_key, client.app_key);
                            if cached_key.eq(&client.app_key) {
                                return (head, Ok(AuthResult {client_id: client.client_id.clone(), sla: sla.clone()}))
                            } else {
                                return (head, Err( "Invalid JWT Token".into()));
                            }
                        } else {
                            if Self::verify_token(token.clone(), &client.pub_key) {
                                cache.put(token, client.app_key.clone());
                                return (head, Ok(AuthResult {client_id: client.client_id.clone(), sla: sla.clone()}))
                            } else {
                                event!(Level::DEBUG, "Invalid JWT Signature");
                                return (head, Err( "Invalid JWT Signature".into()));
                            }
                        }
                    } else {
                        return (head, Err( "No available SLA assigned".into()))
                    }
                } else {
                    return (head, Err( "Invalid app-id".into()));
                }
            } else {
                return (head, Err( "Invalid JWT payload".into()));
            }
        } else {
           return (head, Err("JWT Token not found".into()));
        }
    }
}


impl JWTAuthProvider {

    pub fn new() -> Self {
        JWTAuthProvider {
            apps: HashMap::new(),
            token_cache: Mutex::new(LruCache::new(1024)),
        }
    }

    fn extract_token(head: &Parts) -> Option<String> {
        if let Some(token) = head.headers.get(hyper::header::AUTHORIZATION) {  // find in authorization header
            let segs: Vec<&str> = token.to_str().unwrap().split(' ').collect();
            let token = *(segs.get(1).unwrap_or(&""));
            Some(String::from(token))
        } else {
            None
        }
    }

    fn extract_client_id(token: &str) -> Option<String> {
        let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        if let Ok(t) = jwt::dangerous_insecure_decode::<JwtClaims>(token) {
            if t.claims.exp > ts.as_secs() {
                return Some(t.claims.sub);
            }
        }
        None
    }

    fn verify_token(token: String, pubkey: &str) -> bool {
        let verifier = jwt::Validation::new(jwt::Algorithm::ES256);
        let verify_key = jwt::DecodingKey::from_ec_pem(pubkey.as_bytes()).unwrap();
        if let Ok(_td) = jwt::decode::<JwtClaims>(&token, &verify_key, &verifier) {
            true
        } else {
            false
        }
    }

}


#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub exp: u64,                    // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: Option<u64>,            // Optional. Issued at (as UTC timestamp)
    pub iss: Option<String>,         // Optional. Issuer
    pub sub: String,                 // Optional. Subject (whom token refers to)
}
