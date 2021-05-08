use hyper::http::HeaderValue;
use std::{pin::Pin, time::SystemTime};
use std::future::Future;
use crate::middleware::{MwPostRequest, MwPreRequest, MwPostResponse, Middleware};
use crate::config::ConfigUpdate;


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


impl Middleware for LoggerMiddleware {
    fn name() -> String {
        "Logger".into()
    }

    fn pre() -> bool {
        false
    }

    fn require_setting() -> bool {
        false
    }

    fn request(&mut self, _task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        panic!("never got here");
    }

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPostRequest {context, response, service_filters: _, client_filters: _, result} = task;
        let status = response.status().as_u16().to_string();
        let empty_value = HeaderValue::from_static("");
        let upstream = response.headers().get("X-UPSTREAM-ID").unwrap_or(&empty_value).to_str().unwrap();
        let version = response.headers().get("X-UPSTREAM-VERSION").unwrap_or(&empty_value).to_str().unwrap();
        
        let elapsed = SystemTime::now().duration_since(context.start_time).unwrap();
        HTTP_REQ_DURATION_HIST.with_label_values(&[
            &context.service_id,
            &context.client_id,
            upstream,
            version,
        ]).observe(elapsed.as_secs_f64());

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
        let _ = result.send(Ok(response));
        Box::pin(async {})
    }

    fn config_update(&mut self, _update: ConfigUpdate) {}

}



