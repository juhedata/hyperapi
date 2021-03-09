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