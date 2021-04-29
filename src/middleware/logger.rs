use hyper::{Body, Request, Response, http::HeaderValue};
use std::{pin::Pin, time::{SystemTime, UNIX_EPOCH}};
use std::future::Future;
use crate::middleware::{MwPostRequest, MwPreRequest, MwPostResponse, MwPreResponse, Middleware};
use crate::config::ConfigUpdate;
use prometheus::{Encoder, TextEncoder};


lazy_static::lazy_static! {
    static ref HTTP_COUNTER: prometheus::IntCounterVec = prometheus::register_int_counter_vec!(
        "gateway_requests_total",
        "Number of HTTP requests.",
        &["service", "app", "upstream", "version", "status_code", "path"]
    ).unwrap();
    static ref HTTP_REQ_DURATION_HIST: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "gateway_request_duration_seconds",
        "Request latency histgram",
        &["service", "app", "upstream", "version"],
        vec![0.01, 0.05, 0.25, 1.0, 5.0]
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
    pub fn prometheus_endpoint(_req: &Request<Body>) -> Response<Body> {
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

    fn extract_service_path(path: &str) -> String {
        let path = path.strip_prefix("/").unwrap_or(path);
        let (service_path, _path) = match path.find("/") {
            Some(pos) => {
                path.split_at(pos)
            },
            None => {
                (path, "")
            }
        };
        format!("/{}", service_path)
    }
    
}


impl Middleware for LoggerMiddleware {
    fn name() -> String {
        "Logger".into()
    }

    fn require_setting() -> bool {
        false
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPreRequest {mut context, request, service_filters: _, client_filters: _, result} = task;
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        context.args.insert("TS".into(), ts.as_secs_f64().to_string());

        if context.service_id.len() == 0 || context.client_id.len() == 0 {  // auth failed, or metrics request
            let url = request.uri().path();
            let listen_path = Self::extract_service_path(url);
            if listen_path.eq("/metrics") {
                let resp = Self::prometheus_endpoint(&request);
                let response = MwPreResponse {context: context, request: Some(request), response: Some(resp) };
                let _ = result.send(response);
            } else {
                let error = {
                    if let Some(e) = context.args.get("ERROR") {
                        e.clone()
                    } else {
                        String::from("Authentication Error")
                    }
                };
                let resp = Response::builder().status(403)
                        .body(error.into())
                        .unwrap();
                let response = MwPreResponse {context: context, request: Some(request), response: Some(resp) };
                let _ = result.send(response);
            }
            Box::pin(async {})
        } else {
            let response = MwPreResponse {context: context, request: Some(request), response: None };
            let _ = result.send(response);
            Box::pin(async {})
        }
    }

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPostRequest {context, response, service_filters: _, client_filters: _, result} = task;
        let status = response.status().as_u16().to_string();
        let empty_value = HeaderValue::from_static("");
        let upstream = response.headers().get("X-UPSTREAM-ID").unwrap_or(&empty_value).to_str().unwrap();
        let version = response.headers().get("X-UPSTREAM-VERSION").unwrap_or(&empty_value).to_str().unwrap();
        
        if let Ok(start_time) = context.args.get("TS").unwrap().parse::<f64>() {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            HTTP_REQ_DURATION_HIST.with_label_values(&[
                &context.service_id,
                &context.client_id,
                upstream,
                version,
            ]).observe(now.as_secs_f64() - start_time);
        }

        let path = context.api_path.clone();
        HTTP_COUNTER.with_label_values(&[
            &context.service_id, 
            &context.client_id, 
            upstream, 
            version,
            &status, 
            &path,
        ]).inc_by(1);

        let response = MwPostResponse {context: context, response: response };
        let _ = result.send(response);
        Box::pin(async {})
    }

    fn config_update(&mut self, _update: ConfigUpdate) {}


}



