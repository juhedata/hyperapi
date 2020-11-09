use tracing::{Level, event};
use hyper::Response;
use tokio::sync::mpsc;
use std::time::Duration;
use tower::{Service, ServiceExt};
use std::collections::HashMap;
use crate::config::{ServiceInfo, FilterSetting};
use crate::proxy::ServiceRequest;
use crate::layer::{ProxyService, CorsService, HeaderService, RateLimitService};


pub struct ServiceHandler {
    worker_queues: HashMap<String, mpsc::Sender<ServiceRequest>>,
}


impl ServiceHandler {

    pub fn new(config: &Vec<ServiceInfo>) -> Self {
        let mut worker_queues: HashMap<String, mpsc::Sender<ServiceRequest>> = HashMap::new();

        for c in config.iter() {
            let (tx, rx) = mpsc::channel::<ServiceRequest>(100);
            let conf = c.clone();
            tokio::spawn(async move {
                ServiceHandler::service_worker(rx, conf).await;
            });
            worker_queues.insert(c.service_id.clone(), tx);
        }

        ServiceHandler { worker_queues }
    }

    pub async fn service_chain(&mut self, mut rx: mpsc::Receiver<ServiceRequest>) {
        event!(Level::DEBUG, "start service handler");
        while let Some(x) = rx.recv().await {
            if let Some(ch) = self.worker_queues.get_mut(&x.service) {
                ch.send(x).await.unwrap();
            } else {
                x.result.send(Response::new("Invalid Service ID".into())).unwrap();
            }
        }
    }

    pub async fn service_worker(mut rx: mpsc::Receiver<ServiceRequest>, conf: ServiceInfo) {
        let us = conf.upstreams.iter().map(|u| u.target.clone()).collect();
        let proxy = ProxyService::new(us, Duration::from_millis(conf.timeout));

        // apply filters
        let mut cors_layer = Vec::new();
        let mut header_layer = Vec::new();
        let mut rate_limit_layer = Vec::new();
        for f in conf.filters.iter() {
            match f {
                FilterSetting::Cors(x) => cors_layer.push(x.clone()),
                FilterSetting::Header(x) => header_layer.push(x.clone()),
                FilterSetting::RateLimit(x) => rate_limit_layer.push(x.clone()),
            };
        }
        let proxy = CorsService::new(cors_layer, proxy); 
        let proxy = HeaderService::new(header_layer, proxy);
        let mut proxy = RateLimitService::new(rate_limit_layer, proxy);
        
        while let Some(x) = rx.recv().await {
            let result_tx = x.result;
            if let Ok(px) = proxy.ready_and().await {
                let f = px.call(x.request);
                tokio::spawn(async move {
                    if let Ok(resp) = f.await {
                        match result_tx.send(resp) {
                            Ok(_) => {},
                            Err(_e) => println!("failed to send result"),
                        }
                    } else {
                        result_tx.send(Response::new("Server Internal Error".into())).unwrap();
                    }
                });
            }
        }
    }
    
}

