use hyper::{Request, Response, Body};
use hyper::header::CONTENT_TYPE;
use tokio::sync::{mpsc, oneshot};
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use std::task::{Poll, Context};
use pin_project::pin_project;
use tracing::{Instrument, span, event, Level};
use prometheus::{Encoder, TextEncoder};


lazy_static::lazy_static! {
    static ref HTTP_COUNTER: prometheus::IntCounterVec = prometheus::register_int_counter_vec!(
        "hyperapi_request_count",
        "Number of HTTP requests.",
        &["service", "client", "status"]
    ).unwrap();
    static ref HTTP_REQ_DURATION_HIST: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "hyperapi_request_duration_seconds",
        "help",
        &["service", "client", "status"],
        vec![0.01, 0.05, 0.25, 1.0, 5.0]
    ).unwrap();
}


#[derive(Debug)]
pub struct AuthRequest {
    pub request: Request<Body>,
    pub service_id: String,
    pub result: oneshot::Sender<Result<Request<Body>, anyhow::Error>>,
}

#[derive(Debug)]
pub struct ServiceRequest {
    pub service: String,
    pub request: Request<Body>,
    pub result: oneshot::Sender<Response<Body>>,
}

#[pin_project]
pub struct RequestHandler {
    pub auth_tx: mpsc::Sender<AuthRequest>,
    pub req_tx: mpsc::Sender<ServiceRequest>,
}


impl RequestHandler {

    pub fn new(auth_tx: mpsc::Sender<AuthRequest>, req_tx: mpsc::Sender<ServiceRequest>) -> Self {
        RequestHandler { 
            auth_tx, 
            req_tx,
        }
    }

    // extract service_id from url path
    pub fn get_service_id(req:  &Request<Body>) -> String {
        let path = req.uri().path();
        let segs: Vec<&str> = path.split('/').collect();
        String::from(*(segs.get(1).unwrap_or(&"")))
    }


    pub fn prometheus_endpoint(&self, _req: Request<Body>) -> Response<Body> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
    
        let response = Response::builder()
            .status(200)
            .header(CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))
            .unwrap();
        response
    }
}


impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let span = span!(Level::INFO, "handle request", ?req);
        let service_id = Self::get_service_id(&req);
        let mut auth_tx = self.auth_tx.clone();
        let mut req_tx = self.req_tx.clone();
        match &service_id[..] {
            "metrics" => {
                let resp = self.prometheus_endpoint(req);
                Box::pin(async move {
                    Ok(resp)
                })
            },
            _ => {
                Box::pin(async move {
                    let timer = Instant::now();
                    event!(Level::INFO, "request Auth");
                    let (tx, rx) = oneshot::channel();
                    auth_tx.send(AuthRequest {
                        service_id: service_id.clone(), 
                        request: req, 
                        result: tx,
                    }).await.unwrap();
                    
                    let resp = match rx.await.unwrap() {
                        Ok(request) => {
                            event!(Level::INFO, "OK return from Auth");
                            let (tx, rx) = oneshot::channel();
                            req_tx.send(ServiceRequest {
                                service: service_id.clone(),
                                request: request,
                                result: tx,
                            }).await.unwrap();
                            let resp = rx.await.unwrap();
                            event!(Level::INFO, "return from Service");
                            HTTP_REQ_DURATION_HIST.with_label_values(&[&service_id, &service_id, &resp.status().to_string()]).observe(timer.elapsed().as_secs_f64());
                            HTTP_COUNTER.with_label_values(&[&service_id, &service_id, &resp.status().to_string()]).inc_by(1);
                            resp
                        },
                        Err(err) => {
                            event!(Level::WARN, "Err return from Auth");
                            HTTP_REQ_DURATION_HIST.with_label_values(&[&service_id, "_unknown", "403"]).observe(timer.elapsed().as_secs_f64());
                            HTTP_COUNTER.with_label_values(&[&service_id, "_unknown", "403"]).inc_by(1);
                            Response::new(err.to_string().into())
                        }
                    };
                    Ok(resp)
                }.instrument(span))
            },
        }
    }
}

