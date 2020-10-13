use hyper::{Response, Request, Body, Uri};
use hyper::Client;
use hyper::client::HttpConnector;
use tower::Service;
use futures::task::Context;
use std::task::Poll;
use anyhow::Error;
use futures::Future;
use crate::proxy::config::ServiceInfo;


pub struct Upstream {
    source: Uri,
    status: bool,
    response_time: i32,
}

pub struct ProxyHandler {
    pub client: Client<HttpConnector, Body>,
    pub upstreams: Vec<Uri>,
    pub loadbalance: String,
}


impl ProxyHandler {

    pub fn new(service: &ServiceInfo) -> ProxyHandler {
        ProxyHandler {
            client: Client::new(),
            upstreams: service.upstreams,
            loadbalance: service.lb_schema,
        }
    }


    pub async fn handle(&self, req: Request<Body>) -> Result<Response<Body>, Error> {

        let resp = self.client.request(req).await?;
        Ok(resp)
    }
}

impl Service<Request<Body>> for ProxyHandler {
    type Response = Response<Body>;
    type Error = Error;
    type Future = impl Future<Output = Result<Response<Body>, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        unimplemented!()
    }
}

