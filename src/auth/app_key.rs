use std::collections::HashMap;
use serde_urlencoded;
use crate::config::{ClientInfo, ConfigUpdate};
use super::{AuthProvider, AuthResult};
use hyper::http::request::Parts;
use std::str::FromStr;
use regex::Regex;
use tracing::{event, Level};

#[derive(Debug)]
pub struct AppKeyAuthProvider {
    app_key: HashMap<String, ClientInfo>,
    app_id: HashMap<String, String>,   // app_id -> app_key
}


impl AuthProvider for AppKeyAuthProvider {
    fn update_config(&mut self, update: crate::config::ConfigUpdate) {
        match update {
            ConfigUpdate::ClientUpdate(client) => {
                let client_key = client.app_key.clone();
                let app_id = client.client_id.clone();
                if let Some(old_app_key) = self.app_id.insert(app_id, client_key.clone()) {
                    self.app_key.remove(&old_app_key);
                }
                self.app_key.insert(client_key, client);
            },
            ConfigUpdate::ClientRemove(cid) => {
                if let Some(app_key) = self.app_id.remove(&cid) {
                    self.app_key.remove(&app_key);
                }
            },
            _ => {},
        }
    }

    fn identify_client(&self, mut head: Parts, service_id: &str) -> (Parts, Result<AuthResult, String>) {
        if let Some(appkey) = Self::get_app_key(&head) {
            if let Some(client) = self.app_key.get(&appkey) {
                if let Some(sla) = client.services.get(service_id) {
                    // replace appkey in url path
                    let url = head.uri.to_string();
                    let replaced = format!("/~{}/", appkey);
                    let url = url.replace(&replaced, "/");
                    head.uri = hyper::Uri::from_str(&url).unwrap();
                    let result =AuthResult {
                        client_id: client.client_id.clone(), 
                        sla: sla.clone(),
                    };
                    return (head, Ok(result));
                } else {
                    return (head, Err( "No available SLA assigned".into()))
                }
            } else {
                return (head, Err( "Invalid app-key".into()));
            }
        } else {
            return (head, Err( "Failed to extract app-key".into()));
        }
    }
}


impl AppKeyAuthProvider {

    pub fn new() -> Self {
        AppKeyAuthProvider {
            app_key: HashMap::new(),
            app_id: HashMap::new(),
        }
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
                event!(Level::DEBUG, "{:?}", query_pairs);
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
}

