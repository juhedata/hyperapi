use hyper::{Body, Request, Response, Uri, header::HeaderValue, StatusCode};
use hyper::client::HttpConnector;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use rustls::ClientConfig;
use tower::Service;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::future::Future;
use std::time::Duration;
use tracing::{event, Level};
use crate::config::Upstream;


lazy_static::lazy_static! {

    static ref HTTP_REQ_INPROGRESS: prometheus::IntGaugeVec = prometheus::register_int_gauge_vec!(
        "gateway_requests_in_progress",
        "Request in progress count",
        &["service", "upstream"]
    ).unwrap();

}


#[derive(Debug, Clone)]
pub struct ProxyHandler {
    service_id: String,
    upstream_id: String,
    upstream: String,
    timeout: Duration,
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl ProxyHandler {

    pub fn new(service_id: &str, upstream: &Upstream) -> Self {
        let mut connector = HttpConnector::new();
        let timeout = Duration::from_secs(upstream.timeout);
        connector.set_connect_timeout(Some(timeout));
        connector.set_keepalive(Some(Duration::from_secs(30)));

        let mut tls_config = ClientConfig::new();
        tls_config.root_store = match rustls_native_certs::load_native_certs() {
            Ok(store) => store,
            Err((Some(store), err)) => {
                log::warn!("Could not load all certificates: {:?}", err);
                store
            }
            Err((None, err)) => Err(err).expect("cannot access native cert store"),
        };
        if tls_config.root_store.is_empty() {
            panic!("no CA certificates found");
        }

        let tls = HttpsConnector::from((connector, tls_config));
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(upstream.timeout))
            .build::<_, Body>(tls);

        ProxyHandler { 
            service_id: String::from(service_id), 
            client, 
            timeout,
            upstream: upstream.target.clone(), 
            upstream_id: upstream.id.clone(),
        }
    }

    fn alter_request(req: Request<Body>, endpoint: &str) -> Request<Body> {
        let (mut parts, body) = req.into_parts();
        parts.version = hyper::http::Version::HTTP_11;
        let path_and_query = parts.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
        let path = path_and_query.strip_prefix("/").unwrap_or("/");
        let path_left = if let Some(offset) = path.find("/") {
            let (_service_id, path_left) = path.split_at(offset);
            path_left
        } else {
            ""
        };
        let mut new_uri = String::from(endpoint.trim_end_matches('/'));
        new_uri.push_str(path_left);

        parts.uri = new_uri.parse::<Uri>().unwrap();
        Request::from_parts(parts, body)
    }
}

impl Service<Request<Body>> for ProxyHandler
{
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let req = ProxyHandler::alter_request(req, &self.upstream);
        event!(Level::DEBUG, "{:?}", req.uri());
        let upstream_id = self.upstream_id.to_string();
        let service_id = self.service_id.clone();
        HTTP_REQ_INPROGRESS.with_label_values(&[
            &service_id, 
            &upstream_id,
        ]).inc();
        let sleep = tokio::time::sleep(self.timeout.clone());
        let fut = self.client.request(req);
        Box::pin(async move {
            let result = tokio::select! {
                resp = fut => {
                    resp.map_err(|e| e.into())
                },
                _ = sleep => {
                    Err(anyhow::anyhow!("Request Timeout"))
                },
            };
            HTTP_REQ_INPROGRESS.with_label_values(&[
                &service_id,
                &upstream_id,
            ]).dec();
            if let Ok(mut resp) = result {
                let header = resp.headers_mut();
                let header_value = HeaderValue::from_str(&upstream_id).unwrap();
                header.append("X-UPSTREAM-ID", header_value);
                Ok(resp)
            } else {
                let err = Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from(format!("Error request upstream: {:?}", result)))
                    .unwrap();
                Ok(err)
            }
        })
    }
}

