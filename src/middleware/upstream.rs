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
use crate::config::ServiceInfo;
use crate::middleware::MiddlewareRequest;
use crate::middleware::proxy::ProxyHandler;
use super::middleware::MwPreRequest;


type BoxError = Box<dyn std::error::Error + Send + Sync>;


#[derive(Debug)]
pub struct UpstreamMiddleware {
    pub tx: mpsc::Sender<MiddlewareRequest>,
    rx: mpsc::Receiver<MiddlewareRequest>,
    worker_queues: HashMap<String, mpsc::Sender<MiddlewareRequest>>,
}


impl UpstreamMiddleware {

    pub fn new(config: &Vec<ServiceInfo>) -> Self {
        let mut worker_queues: HashMap<String, mpsc::Sender<MiddlewareRequest>> = HashMap::new();

        for c in config.iter() {
            let (tx, rx) = mpsc::channel::<MiddlewareRequest>(100);
            let conf = c.clone();
            tokio::spawn(async move {
                Self::service_worker(rx, conf).await;
            });
            worker_queues.insert(c.service_id.clone(), tx);
        }

        let (tx, rx) = mpsc::channel(10);

        UpstreamMiddleware { tx, rx, worker_queues }
    }

    pub async fn worker(&mut self) {
        while let Some(x) = self.rx.recv().await {
            match x {
                MiddlewareRequest::Request(req) => {
                    if let Some(ch) = self.worker_queues.get_mut(&req.context.service_id) {
                        ch.send(MiddlewareRequest::Request(req)).await.unwrap();
                    } else {
                        let err= Response::new(Body::from("Invalid Service Id"));
                        req.result.send(Err(err)).unwrap();
                    }
                },
                MiddlewareRequest::Response(resp) => {
                    resp.result.send(resp.response).unwrap()
                },
            }
        }
    }

    async fn service_worker(mut rx: mpsc::Receiver<MiddlewareRequest>, conf: ServiceInfo) {
        let us: Vec<String> = conf.upstreams.iter().map(|u| u.target.clone()).collect();
        let timeout = Duration::from_millis(conf.timeout);
        let max_conn = 1000;
        let mut service = build_service(us, timeout, max_conn);

        while let Some(x) = rx.recv().await {
            match x {
                MiddlewareRequest::Request(MwPreRequest {context: _, request, result }) => {
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
                },
                MiddlewareRequest::Response(post) => {
                    post.result.send(post.response).unwrap()
                },
            }
            
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

