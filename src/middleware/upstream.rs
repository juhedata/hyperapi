use hyper::{Request, Response, Body, StatusCode};
use tokio::sync::mpsc;
use std::time::Duration;
use tower::{Service, ServiceExt};
use std::collections::HashMap;
use tower::timeout::Timeout;
use tower::discover::ServiceList;
use tower_load::{PeakEwmaDiscover, NoInstrument};
use tower_balance::p2c::Balance;
use tower_limit::concurrency::ConcurrencyLimit;
use tower_load_shed::LoadShed;
use tower_util::BoxService;
use std::future::Future;
use std::pin::Pin;
use crate::config::{ConfigUpdate, ServiceInfo};
use crate::middleware::MiddlewareRequest;
use crate::middleware::proxy::ProxyHandler;
use super::{Middleware, middleware::MwPreRequest};
use tracing::{event, Level};


#[derive(Debug)]
pub struct UpstreamMiddleware {
    pub worker_queues: HashMap<String, mpsc::Sender<MwPreRequest>>,
}

impl Default for UpstreamMiddleware {
    fn default() -> Self {
        UpstreamMiddleware { worker_queues: HashMap::new() }
    }
}

impl UpstreamMiddleware {

    async fn service_worker(mut rx: mpsc::Receiver<MwPreRequest>, conf: ServiceInfo) {
        let us: Vec<String> = conf.upstreams.iter().map(|u| u.target.clone()).collect();
        let timeout = Duration::from_millis(conf.timeout);
        let max_conn = 1000;

        let mut service = match us.len() {
            1 => {
                let u = us.get(0).unwrap();
                let proxy = Timeout::new(ProxyHandler::new(u.clone()), timeout);
                let limit = ConcurrencyLimit::new(proxy, max_conn);
                let service = LoadShed::new(limit);
                BoxService::new(service)
            },
            _ => {
                let list: Vec<Timeout<ProxyHandler>> = us.iter().map(|u| {
                    Timeout::new(ProxyHandler::new(u.clone()), timeout)
                }).collect();
                let discover = ServiceList::new(list);
                let load = PeakEwmaDiscover::new(discover, Duration::from_millis(50), Duration::from_secs(1), NoInstrument);
                let balance = Balance::from_entropy(load);
                let limit = ConcurrencyLimit::new(balance, max_conn);
                let service = LoadShed::new(limit);
                BoxService::new(service)
            },
        };

        while let Some(MwPreRequest {context: _, request, result }) = rx.recv().await {
            event!(Level::DEBUG, "request {:?}", request.uri());
            if let Ok(px) = service.ready_and().await {
                let f = px.call(request);
                tokio::spawn(async move {
                    match f.await {
                        Ok(resp) => {
                            match result.send(Err(resp)) {
                                Ok(_) => {},
                                Err(_e) => println!("failed to send result"),
                            }
                        },
                        Err(e) => {
                            let msg = format!("Gateway error\n{:?}", e);
                            let err = Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(Body::from(msg))
                                .unwrap();
                            result.send(Err(err)).unwrap();
                        }
                    }
                });
            }
        }
    }

}


impl Middleware for UpstreamMiddleware {

    fn work(&mut self, task: MiddlewareRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        match task {
            MiddlewareRequest::Request(req) => {
                if let Some(ch) = self.worker_queues.get_mut(&req.context.service_id) {
                    let mut task_ch = ch.clone();
                    Box::pin(async move {
                        task_ch.send(req).await.unwrap();
                    })
                } else {
                    Box::pin(async {
                        let err= Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("Invalid Service Id"))
                            .unwrap();
                        req.result.send(Err(err)).unwrap();
                    })
                }
            },
            MiddlewareRequest::Response(resp) => Box::pin(async {
                resp.result.send(resp.response).unwrap();
            }),
        }
    }

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(conf) => {
                let (tx, rx) = mpsc::channel(10);
                let service_id = conf.service_id.clone();
                tokio::spawn(async move {
                    Self::service_worker(rx, conf).await;
                });
                self.worker_queues.insert(service_id, tx);
            },
            ConfigUpdate::ServiceRemove(sid) => {
                self.worker_queues.remove(&sid);
            },
            _ => {},
        }
    }
}


