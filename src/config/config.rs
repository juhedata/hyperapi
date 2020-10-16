use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use hyper::Uri;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayConfig {
    pub env: String,
    pub listen: String,
    pub ssl_certificate: Option<String>,
    pub ssl_certificate_key: Option<String>,
    pub daemon: bool,
    pub logging: String,
    pub config_provider: Option<String>,
    pub reload_trigger: String,
    pub filters_setting: HashMap<String, HashMap<String, String>>,
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConfigUpdate {
    Client(ClientInfo),
    Service(ServiceInfo),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientInfo {
    pub app_id: String,
    pub app_key: String,
    pub app_secret: String,
    pub services: HashMap<String, Vec<FilterSetting>>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInfo {
    pub service_id: String,
    pub api: String,
    pub api_type: String,
    pub auth_type: String,
    pub auth_setting: String,
    pub upstreams: Vec<Uri>,
    pub lb_schema: String,
    pub filters: Vec<FilterSetting>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimit {
    pub duration: i32,  // ms
    pub limit: i32,
    pub burst: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimitSetting {
    pub limits: Vec<RateLimit>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IPAclSetting {
    pub white_list: Vec<String>,
    pub black_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeaderSetting {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoggingSetting {
    pub message_expr: String,
    pub condition_expr: String,
    pub category: String,
    pub level: String,
    pub pre_request: bool,
    pub post_request: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CorsSetting {
    pub public: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheSetting {
    pub cache_key_expr: String,
    pub max_entries: i32,
    pub cache_ttl: i32,  // seconds
    pub http_cache_directive: bool,
    pub request_condition_expr: String,
    pub response_condition_expr: String,
    pub invalidation_header: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptSetting {
    pub request_expr: Option<String>,
    pub response_expr: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FilterSetting {
    RateLimit(RateLimitSetting),
    IPAcl(IPAclSetting),
    Header(HeaderSetting),
    Logging(LoggingSetting),
    Cors(CorsSetting),
    Cache(CacheSetting),
    Script(ScriptSetting),
}
