use hyper::{Response, Request, Body, Uri, Parts};
use hyper::{Client, client::HttpConnector};
use tower::Service;
use std::task::{Poll, Context};
use anyhow::Error;
use std::future::Future;
use std::pin::Pin;
use tower::discover::ServiceList;
use tower::load::Load;
use tower::balance::Balance;
use std::time;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::config::ServiceInfo;
use crate::stack::Stack;


pub struct ProxyHandler {
    pub upstream: Uri,
    pub latency: AtomicU32,
}


pub fn build_proxy_handler(upstreams: Vec<Uri>) -> Stack<Balance> {
    let discover = ServiceList::new(upstreams.map(|s| ProxyHandler::new(s)));
    let balance = Balance::new(discover);
    Stack::new(balance)
}


impl ProxyHandler {

    pub fn new(uri: Uri) -> ProxyHandler {
        ProxyHandler { upstream: uri,  latency: AtomicU32::new(100) }
    }


    pub fn alter_request(req: Request<Body>, endpoint: Uri) -> Request<Body> {
        let (parts, body) = req.into_parts();
        let mut uri = parts.uri.clone();

        let new_parts = Parts {
            uri: uri,
            method: parts.method,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            _priv: (),
        };

        Request::from_parts(parts, body)
    }

}


impl Service<Request<Body>> for ProxyHandler {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output=Result<Response<Body>, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let result = async {
            let req = ProxyHandler::alter_request(req, self.upstream);
            let now = time::Instant::now();
            let client = Client::new();
            let resp = client.request(req).await?;
            let latency = now.elapsed().as_millis() as u32;
            self.latency.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| Some((x + latency) / 2));
            Ok(resp)
        };
        Box::pin(result)
    }
}

impl Load for ProxyHandler {
    type Matric = u32;
    fn load(&self) -> Self::Metric {
        self.latency.load(Ordering::SeqCst)
    }
}

