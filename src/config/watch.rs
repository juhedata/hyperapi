use hyper::{Body, Uri, body::Buf};
use tokio::sync::broadcast;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::config::{GatewayConfig, ClientInfo, ServiceInfo};


#[derive(Serialize, Deserialize, Debug)]
struct WebConfigSource {
    pub apihub: ServiceConfig,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServiceConfig {
    pub version: i32,
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Debug, Clone)]
pub enum ConfigUpdate {
    ServiceUpdate(ServiceInfo),
    ServiceRemove(String),
    ClientUpdate(ClientInfo),
    ClientRemove(String),
}


pub struct ConfigSource {
    pub config: GatewayConfig
}


impl ConfigSource {
    pub async fn watch(&mut self, updates: broadcast::Sender<ConfigUpdate>) {
        for s in self.config.services.iter() {
            updates.send(ConfigUpdate::ServiceUpdate(s.clone())).unwrap();
        }

        for c in self.config.apps.iter() {
            updates.send(ConfigUpdate::ClientUpdate(c.clone())).unwrap();
        }

        if let Some(url) = &self.config.config_source {
            if url.starts_with("http") {
                let mut http_source = HttpConfigSource::new(url.clone());
                while let Some(update) = http_source.rx.recv().await {
                    updates.send(update).unwrap();
                }
            } else if url.starts_with("etcd") {
                let mut etcd_source = EtcdConfigSource::new(url.clone());
                while let Some(update) = etcd_source.rx.recv().await {
                    updates.send(update).unwrap();
                }
            } else if url.starts_with(""){
                panic!("Invalid config source {}", url);
            }
        }
    }
}

pub struct HttpConfigSource {
    pub rx: mpsc::Receiver<ConfigUpdate>,
}

impl HttpConfigSource {

    pub fn new(url: String) -> Self {
        let uri = url.parse::<Uri>().unwrap();
        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
            Self::config_updates(uri, Duration::from_secs(30), tx).await;
        });
        HttpConfigSource { rx }
    }

    async fn config_updates(
        source: Uri, 
        timeout: Duration, 
        mut tx: mpsc::Sender<ConfigUpdate>
    ) {
        let tls = HttpsConnector::new();
        let client = Client::builder()
            .pool_idle_timeout(timeout)
            .build::<_, Body>(tls);
        let mut ts = Instant::now();

        let mut old_services = HashMap::new();
        let mut old_clients = HashMap::new();

        loop {
            // poll config update from server
            if let Ok(mut resp) = client.get(source.clone()).await {
                let body = resp.body_mut();
                if let Ok(data) = hyper::body::aggregate(body).await {
                    if let Ok(conf) = serde_json::from_slice::<WebConfigSource>(data.bytes()) {
                        let config = conf.apihub;

                        let mut new_services = HashMap::new();
                        for s in config.services.iter() {
                            new_services.insert(s.service_id.clone(), s.clone());
                        }
                        let mut new_clients = HashMap::new();
                        for c in config.apps.iter() {
                            new_clients.insert(c.app_key.clone(), c.clone());
                        }

                        let diff = Self::config_diff(config, old_services.clone(), old_clients.clone());
                        for cd in diff {
                            tx.send(cd).await.unwrap();
                        }
                        old_services = new_services;
                        old_clients = new_clients;
                    }
                }
            }
            
            // delay for next interval
            let interval = ts.elapsed();
            if interval < timeout {
                tokio::time::delay_for(timeout - interval).await;
            }
            ts = Instant::now();
        }
    }

    fn config_diff(config: ServiceConfig, mut old_services: HashMap<String, ServiceInfo>, mut old_clients: HashMap<String, ClientInfo>) -> Vec<ConfigUpdate> {
        let mut diff = Vec::new();
        for s in config.services.iter() {
            if let Some(exist) = old_services.get(&s.service_id) {
                if *s == *exist {
                    continue;
                } else {
                    diff.push(ConfigUpdate::ServiceUpdate(s.clone()));
                }
                old_services.remove(&s.service_id);
            } else {
                diff.push(ConfigUpdate::ServiceUpdate(s.clone()));
            }
        }
        for (k, _v) in old_services.iter() {
            diff.push(ConfigUpdate::ServiceRemove(k.clone()));
        }

        for c in config.apps.iter() {
            if let Some(exist) = old_clients.get(&c.app_key) {
                if *c == *exist {
                    continue;
                } else {
                    diff.push(ConfigUpdate::ClientUpdate(c.clone()));
                }
                old_clients.remove(&c.app_key);
            } else {
                diff.push(ConfigUpdate::ClientUpdate(c.clone()));
            }
        }
        for (k, _v) in old_clients.iter() {
            diff.push(ConfigUpdate::ClientRemove(k.clone()));
        }

        diff
    }

}



pub struct EtcdConfigSource {
    pub rx: mpsc::Receiver<ConfigUpdate>,
}

impl EtcdConfigSource {
    pub fn new(url: String) -> Self {
        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
            Self::config_updates(url, tx).await;
        });
        EtcdConfigSource { rx }
    }

    async fn config_updates(_source: String, mut _tx: mpsc::Sender<ConfigUpdate>) {
        todo!()
    }
 }

 