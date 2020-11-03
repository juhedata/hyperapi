use hyper::{Request, Body};
use std::collections::HashMap;
use tokio::sync::mpsc;
use base64::decode as base64_decode;
use jsonwebtoken as jwt;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use hyper::http::header::AUTHORIZATION;
use crate::proxy::AuthRequest;
use crate::config::*;


#[derive(Clone)]
pub struct AuthHandler {
    apps: Arc<HashMap<String, ClientFilter>>,
    services: HashMap<String, AuthSetting>
}


impl AuthHandler {
    pub fn new(config: &GatewayConfig) -> Self {
        let mut apps = HashMap::new();
        for c in config.apps.iter() {
            apps.insert(c.app_key.clone(), ClientFilter::new(c));
        }

        let mut services = HashMap::new();
        for s in config.services.iter() {
            services.insert(s.service_id.clone(), s.auth.clone());
        }

        AuthHandler { apps: Arc::new(apps), services }
    }


    pub async fn auth_worker(&mut self, mut rx: mpsc::Receiver<AuthRequest>) {
        while let Some(x) = rx.recv().await {
            let AuthRequest {service_id, request, result} = x;
            let apps = self.apps.clone();
            let apps2 = self.apps.clone();
            if let Some(auth) = self.services.get(&service_id) {
                let key_future = Self::verify_token(request, auth.clone(), apps);
                tokio::spawn(async move {
                    if let Some(app_key) = key_future.await {
                        if let Some(client) = apps2.get(&app_key) {
                            let info = client.filter().await;
                            result.send(info).unwrap();
                            return;
                        }
                    }
                    result.send("".into()).unwrap();
                });
            } else {
                result.send("".into()).unwrap();
            }
        }
    }


    async fn verify_token(request: Request<Body>, auth_type: AuthSetting, apps: Arc<HashMap<String, ClientFilter>>) -> Option<String> {
        let app_key = match auth_type {
            AuthSetting::AppKey(AppKeyAuth { header_name: _header, param_name: _param }) => {
                Some(token.into())
            },
            AuthSetting::Basic(BasicAuth {}) => {
                let ts = base64_decode(&token).ok()
                        .map(|s| String::from_utf8(s).unwrap_or(String::from(":")))
                        .unwrap_or(String::from(":"));
                let segs: Vec<&str> = ts.split(':').collect();
                let key = segs.get(0)?;
                let secret = segs.get(1)?;
                let app_key: Option<String> = {
                    if let Some(client) = apps.get(*key) {
                        if client.app_secret.eq(secret) {
                            return Some(String::from(*key));
                        }
                    }
                    None
                };
                app_key
            },
            AuthSetting::JWT(JwtAuth {identity: _sub}) => {
                let t = jwt::dangerous_insecure_decode::<JwtClaims>(&token).ok()?;
                let app_key = &t.claims.sub;
                let app_secret = {
                    apps.get(app_key).map(|c| c.app_secret.clone())
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


#[derive(Clone)]
pub struct ClientFilter {
    pub app_id: String,
    pub app_key: String,
    pub app_secret: String,
}

impl ClientFilter {
    pub fn new(c: &ClientInfo) -> Self {
        ClientFilter { 
            app_id: c.app_id.clone(),
            app_key: c.app_key.clone(),
            app_secret: c.app_secret.clone(),
        }
    }

    pub async fn filter(&self) -> String {

        "".into()
    }
}