use hyper::{Request, Response, Body, Uri};
use hyper::client::connect::HttpConnector;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use tower::Service;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::future::Future;
use std::time::Duration;



#[derive(Debug, Clone)]
pub struct ProxyHandler {
    upstream: String,
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl ProxyHandler {

    pub fn new(upstream: String) -> Self {
        let tls = HttpsConnector::new();
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .build::<_, Body>(tls);
        ProxyHandler { client, upstream }
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
    type Error = hyper::error::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let req = ProxyHandler::alter_request(req, &self.upstream);
        //println!("{:?}", req.uri());
        let f = self.client.request(req);
        Box::pin(async move {
            f.await
        })
    }
}

