use hyper::{Request, Response, Body};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::pin::Pin;
use futures::stream::StreamExt;
use base64::decode as base64_decode;
use serde::{Serialize, Deserialize};
use anyhow::Error;
use jsonwebtoken as jwt;
use tower::Service;
use std::future::Future;
use crate::config::*;
use crate::stack::Stack;
use crate::proxy::build_proxy_handler;


type ProxyService = dyn Service<Request<Body>, 
    Response=Response<Body>, 
    Error=Error, 
    Future=Pin<Box<dyn Future<Output=Result<Response<Body>, Error>> + Send + 'static>>
>;

pub struct ServiceProvider {
    client_info: RwLock<HashMap<String, ClientInfo>>,
    service_info: RwLock<HashMap<String, ServiceInfo>>,
    service_stack: RwLock<HashMap<String, Stack<Box<ProxyService>>>>,
}


pub async fn build_service_provider(config: &GatewayConfig) -> ServiceProvider {
    let client_info = HashMap::new();
    let service_info = HashMap::new();
    let service_stack = HashMap::new();

    for s in config.services {
        service_info.insert(s.service_id, s);
    }
    for a in config.apps {
        client_info.insert(a.app_key, a);
    }

    let provider = ServiceProvider {
        client_info: RwLock::new(client_info),
        service_info: RwLock::new(service_info), 
        service_stack: RwLock::new(service_stack),
    };

    provider
}


impl<'a> ServiceProvider {

    pub async fn authenticate(&'a self, token: &str, auth_type: &str) -> Option<&'a ClientInfo> {
        let app_key = self.verify_token(token, auth_type).await?;
        let cr_lock = self.client_info.read().await;
        cr_lock.get(&app_key)
    }

    pub async fn update_client_info(&self, client: ClientInfo) {
        let wlock = self.client_info.write().await;
        wlock.insert(client.app_key, client);
    }

    pub async fn update_service_info(&self, service: ServiceInfo) {
        let wlock = self.service_info.write().await;
        wlock.insert(service.service_id, service);
    }

    pub async fn verify_token(&self, token: &str, auth_type: &str) -> Option<String> {
        let app_key = match auth_type {
            "appkey" => {
                Some(String::from(token))
            },
            "basic" => {
                let ts = base64_decode(token).ok()
                        .map(|s| String::from_utf8(s).unwrap_or(String::from(":")))
                        .unwrap_or(String::from(":"));
                let (key, secret) = ts.split_once(':')?;
                let app_key: Option<String> = {
                    let rlock = self.client_info.read().await;
                    let client = rlock.get(key)?;
                    if client.app_secret.eq(secret) {
                        Some(String::from(key))
                    } else {
                        None
                    }
                };
                app_key
            },
            "jwt" => {
                let t = jwt::dangerous_insecure_decode::<JwtClaims>(token).ok()?;
                let app_key = &t.claims.sub;
                let app_secret = {
                    let rlock = self.client_info.read().await;
                    rlock.get(app_key).map(|c| c.app_secret)?
                };
                let v = jwt::Validation::new(jwt::Algorithm::HS256);
                let st = jwt::decode::<JwtClaims>(token, &jwt::DecodingKey::from_secret(app_secret.as_bytes()), &v);
                st.ok().filter(|r| r.claims.iss.eq("APIHUB")).map(|r| r.claims.sub)
            },
            "oauth2" => {
                Some(String::from("APPKEY"))
            }
        };
        app_key
    }

    pub async fn get_service_stack(&self, service_id: &str, client: &ClientInfo) -> &Stack<Box<ProxyService>> {
        let rlock = self.service_stack.read().await;
        let key = format!("{}\t{}", service_id, client.app_key);
        if let Some(stack) = rlock.get(&key) {
            return stack;
        }

        let rlock = self.service_info.read().await;
        let service = rlock.get(service_id).unwrap();
        let stack = build_proxy_handler(service.upstreams);
        for f in service.filters {
            stack = stack.push(f);
        }

        let wlock = self.service_stack.write().await;
        wlock.insert(key, stack);
        wlock.get(key).unwrap()
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

