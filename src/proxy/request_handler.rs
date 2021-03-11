use hyper::{Request, Response, Body};
use tokio::sync::{mpsc, oneshot};
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context};
use crate::{auth::AuthRequest, middleware::{MiddlewareHandle, RequestContext, middleware_chain}};

use tracing::{event, span, Level, Instrument};


pub struct RequestHandler {
    pub stack: Vec<MiddlewareHandle>,
    pub auth: mpsc::Sender<AuthRequest>,
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
        let auth = self.auth.clone();

        let span = span!(Level::DEBUG, "request");
        event!(Level::INFO, "{:?} {:?}", req.method(), req.uri());
        Box::pin(async move {
            // auth
            let (tx, rx) = oneshot::channel();
            let (head, body) = req.into_parts();
            let auth_request = AuthRequest {
                head: head,
                result: tx,
            };
            let _ = auth.send(auth_request).await;
            let auth_result = rx.await;

            // handle request
            if let Ok((head_part, auth_resp)) = auth_result {
                if auth_resp.success {
                    let req = Request::from_parts(head_part, body);
                    let context = RequestContext::new(&req, &auth_resp);
                    let resp: Response<Body> = middleware_chain(req, context, stack).await;
                    Ok(resp)
                } else {
                    Ok(Response::new(auth_resp.error.into()))
                }
            } else {
                Ok(Response::new("Auth Failed".into()))
            }
        }.instrument(span))
    }
}

