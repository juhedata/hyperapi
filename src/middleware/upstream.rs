use hyper::{Request, Response, Body};
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


type BoxError = Box<dyn std::error::Error + Send + Sync>;


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
        let mut service = build_service(us, timeout, max_conn);

        while let Some(MwPreRequest {context: _, request, result }) = rx.recv().await {
            if let Ok(px) = service.ready_and().await {
                let f = px.call(request);
                tokio::spawn(async move {
                    if let Ok(resp) = f.await {
                        match result.send(Err(resp)) {
                            Ok(_) => {},
                            Err(_e) => println!("failed to send result"),
                        }
                    } else {
                        let err = Response::new(Body::from("Server Internal Error"));
                        result.send(Err(err)).unwrap();
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
                        let err= Response::new(Body::from("Invalid Service Id"));
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


fn build_service(upstream: Vec<String>, timeout: Duration, max_conn: usize) -> BoxService<Request<Body>, Response<Body>, BoxError>{
    let list: Vec<Timeout<ProxyHandler>> = upstream.iter().map(|u| {
        Timeout::new(ProxyHandler::new(u.clone()), timeout)
    }).collect();
    let discover = ServiceList::new(list);
    let load = PeakEwmaDiscover::new(discover, Duration::from_millis(50), Duration::from_secs(1), NoInstrument);
    let balance = Balance::from_entropy(load);
    let limit = ConcurrencyLimit::new(balance, max_conn);
    let shed = LoadShed::new(limit);
    BoxService::new(shed)
}

