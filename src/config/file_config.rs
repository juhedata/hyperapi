use tokio::sync::mpsc;
use crate::config::{ConfigUpdate, ClientInfo, ServiceInfo};
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServiceConfig {
    pub clients: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


pub async fn watch_config(config_file: String, sender: mpsc::Sender<ConfigUpdate>) {
    let content = tokio::fs::read_to_string(config_file).await.expect("Failed to read config file");
    let config = serde_yaml::from_str::<ServiceConfig>(&content).expect("Failed to parse config file");
    for s in config.services.iter() {
        let _ = sender.send(ConfigUpdate::ServiceUpdate(s.clone())).await;
    }

    for c in config.clients.iter() {
        let _ = sender.send(ConfigUpdate::ClientUpdate(c.clone())).await;
    }

    let _ = sender.send(ConfigUpdate::ConfigReady(true)).await;
}
