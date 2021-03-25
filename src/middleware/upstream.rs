use hyper::{Request, Response, Body, StatusCode, header::{HeaderName, HeaderValue}};
use tokio::sync::mpsc;
use std::time::Duration;
use std::collections::HashMap;
use tower::Service;
use tower::steer::Steer;
use tower::timeout::Timeout;
use tower::discover::ServiceList;
use tower::load::{PeakEwmaDiscover, Constant, PendingRequestsDiscover, CompleteOnResponse};
use tower::balance::p2c::Balance;
use tower::limit::concurrency::ConcurrencyLimit;
use tower::load_shed::LoadShed;
use tower::util::{BoxService, ServiceExt};
use std::future::Future;
use std::pin::Pin;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::config::{ConfigUpdate, ServiceInfo};
use crate::middleware::{Middleware, MwPreRequest, MwPreResponse, MwPostRequest};
use crate::middleware::proxy::ProxyHandler;
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
        let timeout = Duration::from_millis(conf.timeout);
        let max_conn = 1000;
        let mut service = match conf.upstreams.len() {
            1 => {
                let u = conf.upstreams.get(0).unwrap();
                let proxy = Timeout::new(ProxyHandler::new(&conf.service_id, u), timeout);
                let limit = ConcurrencyLimit::new(proxy, max_conn);
                let service = LoadShed::new(limit);
                BoxService::new(service)
            },
            _ => {
                let list: Vec<Timeout<ProxyHandler>> = conf.upstreams.iter().map(|u| {
                    Timeout::new(ProxyHandler::new(&conf.service_id, u), timeout)
                }).collect();
                if conf.load_balance.eq("hash") {
                    // todo: hash based load-balance
                    let balance = Steer::new(list, |req: &Request<_>, s: &[_]| {
                        let total = s.len();
                        let client_id = req.headers().get("X-CLIENT-ID").unwrap().as_bytes();
                        let mut hasher = DefaultHasher::new();
                        Hash::hash_slice(&client_id, &mut hasher);
                        (hasher.finish() as usize) % total
                    });
                    let limit = ConcurrencyLimit::new(balance, max_conn);
                    let service = LoadShed::new(limit);
                    BoxService::new(service)
                } else if conf.load_balance.eq("load") {
                    let discover = ServiceList::new(list);
                    let load = PeakEwmaDiscover::new(discover, Duration::from_millis(50), Duration::from_secs(1), CompleteOnResponse::default());
                    let balance = Balance::new(load);
                    let limit = ConcurrencyLimit::new(balance, max_conn);
                    let service = LoadShed::new(limit);
                    BoxService::new(service)
                } else if conf.load_balance.eq("conn") {
                    let discover = ServiceList::new(list);
                    let load = PendingRequestsDiscover::new(discover, CompleteOnResponse::default());
                    let balance = Balance::new(load);
                    let limit = ConcurrencyLimit::new(balance, max_conn);
                    let service = LoadShed::new(limit);
                    BoxService::new(service)
                } else {  // random
                    let discover = ServiceList::new(list);
                    let load = Constant::new(discover, 1);
                    let balance = Balance::new(load);
                    let limit = ConcurrencyLimit::new(balance, max_conn);
                    let service = LoadShed::new(limit);
                    BoxService::new(service)
                }

            },
        };

        while let Some(MwPreRequest {context, mut request, service_filters: _, client_filters: _, result }) = rx.recv().await {
            event!(Level::DEBUG, "request {:?}", request.uri());
            let header_key = HeaderName::from_static("X-CLIENT-ID");
            let header_val = HeaderValue::from_str(&context.client_id).unwrap();
            let header = request.headers_mut();
            header.insert(header_key, header_val);

            if let Ok(px) = service.ready().await {
                let f = px.call(request);
                tokio::spawn(async move {
                    let proxy_resp: Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>>  = f.await;
                    match proxy_resp {
                        Ok(resp) => {
                            let response = MwPreResponse { context, request: None, response: Some(resp) };
                            let _ = result.send(response);
                        },
                        Err(e) => {
                            let msg = format!("Gateway error\n{:?}", e);
                            let err = Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(Body::from(msg))
                                .unwrap();
                            let response = MwPreResponse { context, request: None, response: Some(err) };
                            let _ = result.send(response);
                        },
                    }
                });
            }
        }
    }

}


impl Middleware for UpstreamMiddleware {

    fn name() -> String {
        "Upstream".into()
    }

    fn post() -> bool {
        false
    }

    fn require_setting() -> bool {
        false
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        if let Some(ch) = self.worker_queues.get_mut(&task.context.service_id) {
            let task_ch = ch.clone();
            Box::pin(async move {
                let _ = task_ch.send(task).await;
            })
        } else {
            Box::pin(async {
                let err= Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Invalid Service Id"))
                    .unwrap();
                let resp = MwPreResponse { 
                    context: task.context, 
                    request: Some(task.request), 
                    response: Some(err),
                 };
                let _ = task.result.send(resp);
            })
        }
    }

    fn response(&mut self, _task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        panic!("never got here");
    }

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(conf) => {
                let (tx, rx) = mpsc::channel(10);
                let service_id = conf.service_id.clone();
                if (&conf.upstreams).len() > 0 {
                    tokio::spawn(async move {
                        Self::service_worker(rx, conf).await;
                    });
                    self.worker_queues.insert(service_id, tx);
                } else {
                    self.worker_queues.remove(&service_id);
                }
            },
            ConfigUpdate::ServiceRemove(sid) => {
                self.worker_queues.remove(&sid);
            },
            _ => {},
        }
    }
}


