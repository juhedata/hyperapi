use hyper::{Body, Response, Request};
use std::pin::Pin;
use std::future::Future;
use crate::middleware::{MwPostRequest, MwPreRequest, MwPostResponse, MwPreResponse, Middleware};
use crate::config::ConfigUpdate;
use prometheus::{Encoder, TextEncoder};


lazy_static::lazy_static! {
    static ref HTTP_COUNTER: prometheus::IntCounterVec = prometheus::register_int_counter_vec!(
        "gateway_requests_total",
        "Number of HTTP requests.",
        &["service", "app", "upstream", "status", "path"]
    ).unwrap();
    static ref HTTP_REQ_DURATION_HIST: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "gateway_request_duration_seconds",
        "Request latency histgram",
        &["service", "upstream"],
        vec![0.01, 0.05, 0.25, 1.0, 5.0]
    ).unwrap();
    static ref HTTP_REQ_INPROGRESS: prometheus::IntGaugeVec = prometheus::register_int_gauge_vec!(
        "gateway_requests_in_progress",
        "Request in progress count",
        &["service", "upstream"]
    ).unwrap();
    static ref HOST_CPU_USAGE: prometheus::Gauge = prometheus::register_gauge!(
        "gateway_cpu_usage",
        "Gateway host cpu usage"
    ).unwrap();
    static ref HOST_MEM_USAGE: prometheus::Gauge = prometheus::register_gauge!(
        "gateway_mem_usage",
        "Gateway host memory usage"
    ).unwrap();
}


#[derive(Debug)]
pub struct LoggerMiddleware {}

impl Default for LoggerMiddleware {
    fn default() -> Self {
        LoggerMiddleware {}
    }
}

impl LoggerMiddleware {
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


impl Middleware for LoggerMiddleware {
    fn name() -> String {
        "Logger".into()
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPreRequest {context, request, service_filters: _, client_filters: _, result} = task;
        
        // todo  
        let response = MwPreResponse {context: context, request: Some(request), response: None };
        result.send(response).unwrap();
        Box::pin(async {})
    }

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPostRequest {context, response, service_filters: _, client_filters: _, result} = task;
        
        // todo write log
        let response = MwPostResponse {context: context, response: response };
        result.send(response).unwrap();
        Box::pin(async {})
    }

    fn config_update(&mut self, _update: ConfigUpdate) {}

}



