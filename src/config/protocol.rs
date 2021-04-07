use std::collections::HashMap;

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag="type", content="data")]
pub enum ConfigUpdate {
    ServiceUpdate(ServiceInfo),
    ServiceRemove(String),
    ClientUpdate(ClientInfo),
    ClientRemove(String),
    ConfigReady(bool),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayConfig {
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClientInfo {
    pub client_id: String,
    pub app_key: String,
    pub pub_key: String,
    pub ip_whitelist: Vec<String>,
    pub services: HashMap<String, String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ServiceInfo {
    pub service_id: String,
    pub path: String,
    pub protocol: String,
    pub auth: AuthSetting,
    pub upstreams: Vec<Upstream>,
    pub load_balance: String,
    pub timeout: u64,
    pub filters: Vec<FilterSetting>,
    pub sla: Vec<ServiceLevel>,
    pub error_threshold: u64,
    pub error_reset: u64,
    pub retry_delay: u64,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ServiceLevel {
    pub name: String,
    pub filters: Vec<FilterSetting>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Upstream {
    pub target: String,
    pub id: String,
    pub timeout: u64,
    pub max_conn: u64,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RateLimitSetting {
    pub interval: i32,  // seconds
    pub limit: i32,
    pub burst: i32,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HeaderSetting {
    pub operate_on: String,
    pub injection: Vec<(String, String)>,
    pub removal: Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ACLSetting {
    pub access_control: String,
    pub paths: Vec<PathMatcher>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PathMatcher {
    pub methods: String,
    pub path_regex: String,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag="type", content="setting")]
pub enum FilterSetting {
    RateLimit(RateLimitSetting),
    Header(HeaderSetting),
    ACL(ACLSetting),
}


impl FilterSetting {
    pub fn get_type(setting: &FilterSetting) -> String {
        match setting {
            FilterSetting::ACL(_) => "ACL".into(),
            FilterSetting::Header(_) => "Header".into(),
            FilterSetting::RateLimit(_) => "RateLimit".into(),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppKeyAuth {}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JwtAuth {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NoAuth {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum AuthSetting {
    None(NoAuth),
    AppKey(AppKeyAuth),
    JWT(JwtAuth),
}


