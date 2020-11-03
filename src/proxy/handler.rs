use hyper::{Request, Response, Body};
use tokio::sync::{mpsc, oneshot};
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context};
use pin_project::pin_project;


#[derive(Debug)]
pub struct AuthRequest {
    pub request: Request<Body>,
    pub service_id: String,
    pub result: oneshot::Sender<Result<Request<Body>, anyhow::Error>>,
}

#[derive(Debug)]
pub struct ServiceRequest {
    pub service: String,
    pub request: Request<Body>,
    pub result: oneshot::Sender<Response<Body>>,
}

#[pin_project]
pub struct RequestHandler {
    pub auth_tx: mpsc::Sender<AuthRequest>,
    pub req_tx: mpsc::Sender<ServiceRequest>,
}


impl RequestHandler {

    pub fn new(auth_tx: mpsc::Sender<AuthRequest>, req_tx: mpsc::Sender<ServiceRequest>) -> Self {
        RequestHandler { 
            auth_tx, 
            req_tx,
        }
    }

    // extract service_id from url path
    pub fn get_service_id(req:  &Request<Body>) -> String {
        let path = req.uri().path();
        let segs: Vec<&str> = path.split('/').collect();
        String::from(*(segs.get(1).unwrap_or(&"")))
    }

}


impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let service_id = Self::get_service_id(&req);
        let mut auth_tx = self.auth_tx.clone();
        let mut req_tx = self.req_tx.clone();
        Box::pin(async move {
            let (tx, rx) = oneshot::channel();
            auth_tx.send(AuthRequest {
                service_id: service_id.clone(), 
                request: req, 
                result: tx,
            }).await.unwrap();

            let resp = match rx.await.unwrap() {
                Ok(request) => {
                    let (tx, rx) = oneshot::channel();
                    req_tx.send(ServiceRequest {
                        service: service_id.clone(),
                        request: request,
                        result: tx,
                    }).await.unwrap();
                    let resp = rx.await.unwrap();
                    resp
                },
                Err(err) => {
                    Response::new(err.to_string().into())
                }
            };
            Ok(resp)
        })
    }
}

