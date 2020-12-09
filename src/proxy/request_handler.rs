use hyper::{Request, Response, Body};
use tokio::sync::{mpsc, oneshot};
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context};
use crate::middleware::{middleware_chain, RequestContext, MiddlewareRequest};


pub struct RequestHandler {
    pub stack: Vec<mpsc::Sender<MiddlewareRequest>>,
}



impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let stack = self.stack.clone();
        let mut context = RequestContext::new(&req);
        let fut = middleware_chain(req, context, stack);
        Box::pin(async {
            let resp = fut.await;
            Ok(resp)
        })
    }
}

