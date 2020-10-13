use crate::proxy::config::{ServiceProvider, ClientProvider};

mod yaml_config;
mod etcd_config;
mod ws_config;


pub fn new_service_provider(config: &str) -> Result<Box<dyn ServiceProvider>, String> {
    if config.ends_with(".yaml") {
        let sp = self::yaml_config::YamlServiceProvider::new(config)?;
        return Ok(Box::new(sp));
    } else if config.start_with("wss://") {
        let sp = self::ws_config::WSServiceProvider::new(config);
        return Ok(Box::new(sp));
    } else if config.starts_with("etcd://") {
        let sp = self::etcd_config::EtcdServiceProvider::new(config);
        return Ok(Box::new(sp));
    } else {
        Err(String::from("Unsupported config source"))
    }
}

pub fn new_client_provider(config: &str) -> Result<Box<dyn ClientProvider>, String> {
    if config.ends_with(".yaml") {
        let sp = self::yaml_config::YamlClientProvider::new(config)?;
        return Ok(Box::new(sp));
    } else if config.start_with("wss://") {
        let sp = self::ws_config::WSClientProvider::new(config);
        return Ok(Box::new(sp));
    } else if config.starts_with("etcd://") {
        let sp = self::etcd_config::EtcdClientProvider::new(config);
        return Ok(Box::new(sp));
    } else {
        Err(String::from("Unsupported config source"))
    }
}

