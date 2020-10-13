use hyper::service::Service;
use hyper::{Request, Response, Body};
use crate::proxy::config::{ClientInfo, ServiceInfo};
use futures::task::Context;
use futures::Future;
use tokio::sync::oneshot;
use tokio::sync::mpsc;
use std::task::Poll;
use std::marker::PhantomData;
use std::pin::Pin;
use anyhow::Error;
use std::net::SocketAddr;


pub enum AuthQuery {
    AppUpdate(ClientInfo),
    VerifyToken{ service: String, client: String, result: oneshot::Sender<ClientInfo> }
}

pub enum ServiceQuery {
    ServiceUpdate(ServiceInfo),
    GetSettings{ service: String, client: String, result: oneshot::Sender<ServiceInfo> }
}

pub struct RequestHandler
{
    pub address: SocketAddr,
    pub auth_factory: mpsc::Sender<AuthQuery>,
    pub service_factory: mpsc::Sender<ServiceQuery>,
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


impl Service<Request<Body>> for RequestHandler
where
{
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        // let service_id = self.get_service_id(&req);
        // let token = self.get_auth_token(&req);
        let future =  async {
            Ok(Response::new("Hello, World".into()))
        };
        Box::pin(future)
    }
}
