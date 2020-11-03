use log::*;
use hyper::{Request, Body};
use std::collections::HashMap;
use tokio::sync::mpsc;
use base64::decode as base64_decode;
use jsonwebtoken as jwt;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use hyper::http::header::AUTHORIZATION;
use anyhow::anyhow;
use crate::proxy::AuthRequest;
use crate::config::*;
use super::client_filter::ClientFilter;


#[derive(Clone)]
pub struct AuthHandler {
    apps: Arc<HashMap<String, (ClientInfo, HashMap<String, Arc<Mutex<ClientFilter>>>)>>,
    services: HashMap<String, AuthSetting>
}


impl AuthHandler {
    pub fn new(config: &GatewayConfig) -> Self {
        let mut apps = HashMap::new();
        for c in config.apps.iter() {
            let mut ss = HashMap::new();
            for (k, v) in c.services.iter() {
                ss.insert(k.clone(), Arc::new(Mutex::new(ClientFilter::new(v))));
            }
            apps.insert(c.app_key.clone(), (c.clone(), ss));
        }

        let mut services = HashMap::new();
        for s in config.services.iter() {
            services.insert(s.service_id.clone(), s.auth.clone());
        }
        AuthHandler { apps: Arc::new(apps), services }
    }


    pub async fn auth_worker(&mut self, mut rx: mpsc::Receiver<AuthRequest>) {
        debug!("start auth handler");
        while let Some(x) = rx.recv().await {
            let AuthRequest {service_id, request, result} = x;
            if let Some(auth) = self.services.get(&service_id) {
                let app_key = Self::verify_token(&request, auth.clone(), self.apps.clone()).await;
                debug!("app_key {:?}", app_key);
                let client_filter = match app_key {
                    Some(key) => {
                        let cliennt_tuple = self.apps.get(&key);
                        match cliennt_tuple {
                            Some((_client, services)) => {
                                if let Some(cf) = services.get(&service_id) {
                                    Some(cf.clone())
                                } else { None }
                            }, 
                            None => None,
                        }
                    },
                    None => None,
                };

                match client_filter {
                    Some(cf) => {
                        tokio::spawn(async move {
                            let mut lock = cf.lock().await;
                            match lock.filter(request).await {
                                Ok(r) => { result.send(Ok(r)).unwrap(); },
                                Err(e) => { result.send(Err(anyhow!("Auth failed: {}", e))).unwrap(); },
                            }
                        });
                    },
                    None => { result.send(Err(anyhow!("Auth failed"))).unwrap(); }
                }
            } else {
                result.send(Err(anyhow!("Invalid service id"))).unwrap();
            }
        }
    }


    async fn verify_token(request: &Request<Body>, auth_type: AuthSetting, apps: Arc<HashMap<String, (ClientInfo, HashMap<String, Arc<Mutex<ClientFilter>>>)>>) -> Option<String> {
        let app_key = match auth_type {
            AuthSetting::AppKey(AppKeyAuth { header_name: _header, param_name: _param }) => {
                let token = Self::get_auth_token(request);
                Some(token.into())
            },
            AuthSetting::Basic(BasicAuth {}) => {
                let token = Self::get_auth_token(request);
                let ts = base64_decode(&token).ok()
                        .map(|s| String::from_utf8(s).unwrap_or(String::from(":")))
                        .unwrap_or(String::from(":"));
                let segs: Vec<&str> = ts.split(':').collect();
                let key = segs.get(0)?;
                let secret = segs.get(1)?;
                let app_key: Option<String> = {
                    if let Some((client, _)) = apps.get(*key) {
                        if client.app_secret.eq(secret) {
                            return Some(String::from(*key));
                        }
                    }
                    None
                };
                app_key
            },
            AuthSetting::JWT(JwtAuth {identity: _sub}) => {
                let token = Self::get_auth_token(request);
                let t = jwt::dangerous_insecure_decode::<JwtClaims>(&token).ok()?;
                let app_key = &t.claims.sub;
                let app_secret = {
                    apps.get(app_key).map(|(c, _)| c.app_secret.clone())
                };
                if let Some(secret) = app_secret {
                    let v = jwt::Validation::new(jwt::Algorithm::HS256);
                    let st = jwt::decode::<JwtClaims>(&token, &jwt::DecodingKey::from_secret(secret.as_bytes()), &v);
                    st.ok()
                        .filter(|r| r.claims.iss.eq("APIHUB"))
                        .map(|r| r.claims.sub)
                } else {
                    None
                }
            },
            AuthSetting::OAuth2(OAuth2Auth {token_verify_url: _url}) => {
                Some(String::from(""))
            },
        };
        app_key
    }

    // extract token from request
    pub fn get_auth_token(req: &Request<Body>) -> String {
        let headers = req.headers();
        if let Some(token) = headers.get(AUTHORIZATION) {  // find in authorization header
            let segs: Vec<&str> = token.to_str().unwrap().split(' ').collect();
            String::from(*(segs.get(1).unwrap_or(&"")))
        } else {
            String::from("")
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    pub aud: Option<String>,         // Optional. Audience
    pub exp: usize,                  // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: Option<usize>,          // Optional. Issued at (as UTC timestamp)
    pub iss: String,                 // Optional. Issuer
    pub nbf: Option<usize>,          // Optional. Not Before (as UTC timestamp)
    pub sub: String,                 // Optional. Subject (whom token refers to)
}

