use hyper::service::Service;
use hyper::{Request, Response, Body};
use crate::config::{ClientInfo, ServiceInfo};
use std::task::{Context, Poll};
use std::future::Future;
use tokio::sync::oneshot;
use std::marker::PhantomData;
use std::pin::Pin;
use anyhow::Error;
use std::sync::Arc;
use std::net::SocketAddr;
use crate::stack::Stack;
use crate::config::ServiceProvider;


pub struct RequestHandler {
    pub address: SocketAddr,
    pub services: Arc<ServiceProvider>,
    pub _req: PhantomData<Self>,
}


impl RequestHandler {

    pub fn get_service_id<'a>(&self, req:  &'a Request<Body>) -> &'a str {
        req.uri().path()
    }

    pub fn get_auth_token<'a>(&self, req: &'a Request<Body>) -> &'a str {
        req.uri().query().unwrap()
    }
}


impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let service_id = self.get_service_id(&req);
        let token = self.get_auth_token(&req);
        let future =  async {
            let client = self.services.authenticate(token, service_id).await;
            if let Ok(result) = client {
                self.services.get_service_stack(service_id, client);
            }
            Ok(Response::new("Something wrong".into()))
        };
        Box::pin(future)
    }
}

