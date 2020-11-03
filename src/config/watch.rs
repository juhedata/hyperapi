use hyper::{Body, Uri};
use std::sync::{Arc, Mutex};
use crate::proxy::GatewayServer;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use hyper::body::Buf;
use super::config::{ClientInfo, ServiceInfo};


#[derive(Serialize, Deserialize, Debug)]
struct WebConfigSource {
    pub apihub: ServiceConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServiceConfig {
    pub version: i32,
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


pub async fn config_poll(source: Uri, server: Arc<Mutex<GatewayServer>>) {
    let mut version: i32= 0;
    let timeout = Duration::from_secs(30);
    let tls = HttpsConnector::new();
    let client = Client::builder()
        .pool_idle_timeout(timeout)
        .build::<_, Body>(tls);
    let mut ts = Instant::now();
    loop {
        // poll config update from server
        if let Ok(mut resp) = client.get(source.clone()).await {
            let body = resp.body_mut();
            if let Ok(data) = hyper::body::aggregate(body).await {
                if let Ok(conf) = serde_json::from_slice::<WebConfigSource>(data.bytes()) {
                    if conf.apihub.version > version {
                        let mut lock = server.lock().unwrap();
                        lock.update_config(&conf.apihub.apps, &conf.apihub.services);
                        version = conf.apihub.version;
                    }
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


