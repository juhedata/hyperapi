use crate::proxy::config::{ServiceProvider, ServiceInfo, ClientProvider, ClientInfo};
use crate::proxy::proxy::ProxyHandler;
use hyper::{Request, Body};
use std::collections::HashMap;
use std::fs;
use std::io;


pub struct YamlServiceProvider {
    services: HashMap<String, ServiceInfo>,
}


pub struct YamlClientProvider {
    clients: HashMap<String, ClientInfo>,  // client_key -> ClientInfo
}


impl YamlServiceProvider {
    pub fn new(config: &str) -> Result<YamlServiceProvider, String> {
        let content = fs::read_to_string(config)?;
        let config: Vec<ServiceInfo> = serde_yaml::from_str(&content)?;
        let mut services: HashMap<String, ServiceInfo> = HashMap::new();
        for mut si in config.iter() {
            &services.insert(&si.id, si);
        }
        Ok(YamlServiceProvider { services })
    }
}

impl ServiceProvider for YamlServiceProvider {
    async fn get_service_handler(&self, service_id: &str, client_id: &str) -> Result<&ProxyHandler, String> {
        unimplemented!()
    }
}


impl YamlClientProvider {
    pub fn new(config: &str) -> Result<YamlClientProvider, String> {
        let content = fs::read_to_string(config)?;
        let config: Vec<ClientInfo> = serde_yaml::from_str(&content)?;
        let mut clients: HashMap<String, ClientInfo> = HashMap::new();
        for mut ci in config.iter() {
            &clients.insert(ci.app_key, ci);
        }
        Ok(YamlClientProvider { clients })
    }

    fn jwt_verify(&self, token: &str) -> Result<&ClientInfo, String> {

    }

    fn basic_verify(&self, token: &str) -> Result<&ClientInfo, String> {

    }

    fn oauth2_verify(&self, token: &str) -> Result<&ClientInfo, String> {

    }
}

impl ClientProvider for YamlClientProvider {
    async fn authenticate(&self, req: &Request<Body>) -> Result<&ClientInfo, String> {
        let headers = req.headers();
        let auth = headers.get("authorization").unwrap_or("".into());
        let parts = auth.to_str()?.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(String::from("Missing auth header"));
        }
        if parts[0].eq("BASIC") {
            let client = self.basic_verify(parts[1])?;
            Ok(client)
        } else if parts[0].eq("BEARER") {
            let client = self.oauth2_verify(parts[1])?;
            Ok(client)
        } else if parts[0].eq("JWT") {
            let client = self.jwt_verify(parts[1])?;
            Ok(client)
        } else {
            Err(String::from("Unsupported auth type"))
        }
    }
}
