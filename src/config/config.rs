use serde::{Serialize, Deserialize};
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayConfig {
    pub env: String,
    pub listen: String,
    pub ssl_certificate: Option<String>,
    pub ssl_certificate_key: Option<String>,
    pub config_source: Option<String>,
    pub filters_setting: HashMap<String, HashMap<String, String>>,
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientInfo {
    pub app_id: String,
    pub app_key: String,
    pub app_secret: String,
    pub ip_whitelist: Vec<String>,
    pub services: HashMap<String, Vec<FilterSetting>>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInfo {
    pub service_id: String,
    pub api_type: String,
    pub auth: AuthSetting,
    pub upstreams: Vec<Upstream>,
    pub timeout: u64,
    pub filters: Vec<FilterSetting>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Upstream {
    pub target: String,

}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimit {
    pub duration: i32,  // ms
    pub limit: i32,
    pub burst: i32,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimitSetting {
    pub methods: String,
    pub path_pattern: String,
    pub limits: Vec<RateLimit>
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeaderSetting {
    pub methods: String,
    pub path_pattern: String,
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CorsSetting {
    pub methods: String,
    pub path_pattern: String,
    pub public: bool,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FilterSetting {
    RateLimit(RateLimitSetting),
    Header(HeaderSetting),
    Cors(CorsSetting),
}



#[derive(Debug, Clone)]
pub struct ClientId {
    pub app_id: String,
    pub app_key: String,
    pub app_secret: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppKeyAuth {
    pub header_name: Option<String>,
    pub param_name: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicAuth {}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OAuth2Auth {
    pub token_verify_url: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JwtAuth {
    pub identity: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum AuthSetting {
    AppKey(AppKeyAuth),
    Basic(BasicAuth),
    OAuth2(OAuth2Auth),
    JWT(JwtAuth),
}


