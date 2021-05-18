use std::collections::HashMap;

use tokio::sync::mpsc;
use crate::config::{ConfigUpdate, ClientInfo, ServiceInfo};
use serde::{Serialize, Deserialize};
use tracing::{event, Level};
// use tokio::signal::unix::{signal, SignalKind};


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServiceConfig {
    pub clients: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


pub async fn watch_config(config_file: String, sender: mpsc::Sender<ConfigUpdate>) {
    let content = tokio::fs::read_to_string(&config_file).await.expect("Failed to read config file");
    let mut config = serde_yaml::from_str::<ServiceConfig>(&content).expect("Failed to parse config file");
    for s in config.services.iter() {
        let _ = sender.send(ConfigUpdate::ServiceUpdate(s.clone())).await;
    }
    for c in config.clients.iter() {
        let _ = sender.send(ConfigUpdate::ClientUpdate(c.clone())).await;
    }
    let _ = sender.send(ConfigUpdate::ConfigReady(true)).await;

    // // reload config file on USR2 signal
    // let mut usr2 = signal(SignalKind::user_defined2()).expect("Failed to bind on USR2 signal");
    // loop {
    //     usr2.recv().await;
    //     if let Ok(new_content) = tokio::fs::read_to_string(&config_file).await {
    //         if let Ok(new_config) = serde_yaml::from_str::<ServiceConfig>(&new_content) {
    //             for cu in config_diff(&config, &new_config) {
    //                 let _ = sender.send(cu).await;
    //             }
    //             config = new_config;
    //         } else {
    //             event!(Level::ERROR, "Failed to parse config file")
    //         }
    //     } else {
    //         event!(Level::ERROR, "Failed to read config file")
    //     }
    // }
}


fn config_diff(old: &ServiceConfig, new: &ServiceConfig) -> Vec<ConfigUpdate> {
    let mut result = Vec::new();

    let mut exist_service: HashMap<String, bool> = HashMap::new();
    for s in new.services.iter() {
        exist_service.insert(s.service_id.clone(), true);
        result.push(ConfigUpdate::ServiceUpdate(s.clone()));
    }
    for os in old.services.iter() {
        if let Some(_) = exist_service.get(&os.service_id) {
            // pass
        } else {
            result.push(ConfigUpdate::ServiceRemove(os.service_id.clone()))
        }
    }

    let mut exist_client: HashMap<String, bool> = HashMap::new();
    for c in new.clients.iter() {
        exist_client.insert(c.client_id.clone(), true);
        result.push(ConfigUpdate::ClientUpdate(c.clone()));
    }
    for oc in old.clients.iter() {
        if let Some(_) = exist_client.get(&oc.client_id) {
            // pass
        } else {
            result.push(ConfigUpdate::ClientRemove(oc.client_id.clone()))
        }
    }

    return result;
}



