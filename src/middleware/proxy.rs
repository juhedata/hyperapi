use hyper::{Body, Request, Response, Uri, header::HeaderValue};
use hyper::client::HttpConnector;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use tower::Service;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::future::Future;
use std::time::Duration;
use tracing::{event, Level};

use crate::config::Upstream;


#[derive(Debug, Clone)]
pub struct ProxyHandler {
    upstream_id: u64,
    upstream: String,
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl ProxyHandler {

    pub fn new(upstream: &Upstream) -> Self {
        let tls = HttpsConnector::with_native_roots();
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(upstream.timeout))
            .build::<_, Body>(tls);
        ProxyHandler { client: client, upstream: upstream.target.clone(), upstream_id: upstream.id }
    }

    fn alter_request(req: Request<Body>, endpoint: &str) -> Request<Body> {
        let (mut parts, body) = req.into_parts();

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
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let req = ProxyHandler::alter_request(req, &self.upstream);
        event!(Level::DEBUG, "{:?}", req.uri());
        let f = self.client.request(req);
        let upstream_id = self.upstream_id.to_string();
        Box::pin(async move {
            let mut resp = f.await.unwrap();
            let header = resp.headers_mut();
            let header_value = HeaderValue::from_str(&upstream_id).unwrap();
            header.append("X-UPSTREAM-ID", header_value);
            Ok(resp)
        })
    }
}

