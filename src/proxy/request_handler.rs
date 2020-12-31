use hyper::{Request, Response, Body};
use tokio::sync::mpsc;
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use std::task::{Poll, Context};
use crate::middleware::{middleware_chain, RequestContext, MiddlewareRequest};
use prometheus::{Encoder, TextEncoder};
use tracing::{event, span, Level, Instrument};


lazy_static::lazy_static! {
    static ref HTTP_COUNTER: prometheus::IntCounterVec = prometheus::register_int_counter_vec!(
        "hyperapi_request_count",
        "Number of HTTP requests.",
        &["service", "client", "status"]
    ).unwrap();
    static ref HTTP_REQ_DURATION_HIST: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "hyperapi_request_duration_seconds",
        "Request latency",
        &["service", "client", "status"],
        vec![0.01, 0.05, 0.25, 1.0, 5.0]
    ).unwrap();
}


pub struct RequestHandler {
    pub stack: Vec<mpsc::Sender<MiddlewareRequest>>,
}


impl RequestHandler {
    pub fn prometheus_endpoint(&self, _req: Request<Body>) -> Response<Body> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();

        let response = Response::builder()
            .status(200)
            .header(hyper::header::CONTENT_TYPE, encoder.format_type())
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
        let timer = Instant::now();
        let context = RequestContext::new(&req);
        let service_id = context.service_id.clone();

        match &service_id[..] {
            "metrics" => {
                event!(Level::INFO, "get gateway metrics");
                let resp = self.prometheus_endpoint(req);
                Box::pin(async move {
                    Ok(resp)
                })
            },
            _ => {
                let stack = self.stack.clone();
                let span = span!(Level::INFO, "request",
                                        service=context.service_id.as_str(),
                                        trace_id=context.request_id.to_string().as_str());
                let fut = middleware_chain(req, context, stack);
                Box::pin(async move {
                    let resp = fut.await;
                    HTTP_REQ_DURATION_HIST.with_label_values(&[&service_id, "", &resp.status().to_string()]).observe(timer.elapsed().as_secs_f64());
                    HTTP_COUNTER.with_label_values(&[&service_id, "", &resp.status().to_string()]).inc_by(1);
                    Ok(resp)
                }.instrument(span))
            }
        }
    }
}

