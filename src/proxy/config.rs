use serde::{Serialize, Deserialize};
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayConfig {
    pub env: String,
    pub listen: String,
    pub ssl_certificate: Option<String>,
    pub ssl_certificate_key: Option<String>,
    pub daemon: bool,
    pub logging: String,
    pub config_provider: Option<String>,
    pub reload_trigger: String,
    pub filters_setting: bool,
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct ClientInfo {
    pub id: String,
    pub app_key: String,
    pub app_secret: String,

}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceInfo {
    pub service_id: String,
    pub api: String,
    pub api_type: String,
    pub auth_type: String,
    pub auth_setting: String,
    pub upstreams: Vec<String>,
    pub lb_schema: String,
    pub filters: Vec<FilterSetting>,
    pub clients: HashMap<String, Vec<FilterSetting>>
}


#[derive(Serialize, Deserialize, Debug)]
pub struct RateLimit {
    pub duration: i32,  // ms
    pub limit: i32,
    pub burst: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateLimitSetting {
    pub limits: Vec<RateLimit>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IPAclSetting {
    pub white_list: Vec<String>,
    pub black_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderSetting {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoggingSetting {
    pub message_expr: String,
    pub condition_expr: String,
    pub category: String,
    pub level: String,
    pub pre_request: bool,
    pub post_request: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CorsSetting {
    pub public: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CacheSetting {
    pub cache_key_expr: String,
    pub max_entries: i32,
    pub cache_ttl: i32,  // seconds
    pub http_cache_directive: bool,
    pub request_condition_expr: String,
    pub response_condition_expr: String,
    pub invalidation_header: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScriptSetting {
    pub request_expr: Option<String>,
    pub response_expr: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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


impl FilterSetting {
    pub fn get_type(fs: &FilterSetting) -> String {
        match fs {
            FilterSetting::RateLimit(_) => String::from("proxy"),
            FilterSetting::IPAcl(_) => String::from("ip_acl"),
            FilterSetting::Header(_) => String::from("header"),
            FilterSetting::Logging(_) => String::from("logging"),
            FilterSetting::Cors(_) => String::from("cors"),
            FilterSetting::Cache(_) => String::from("cache"),
            FilterSetting::Script(_) => String::from("script"),
        }
    }
}