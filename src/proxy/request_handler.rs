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
    pub ready: u8,
}


impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if self.ready == 0 {  // starting
            return Box::pin(async {
                Ok(Response::new("Server is initializing...".into()))
            })
        }

        if self.ready == 2 {  // closing
            return Box::pin(async {
                Ok(Response::new("Server is closing...".into()))
            })
        }

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
                let req = Request::from_parts(head_part, body);
                let context = RequestContext::new(&req, &auth_resp);
                let resp: Response<Body> = middleware_chain(req, context, stack).await;
                Ok(resp)
            } else {
                Ok(Response::builder().status(502).body("Auth Error".into()).unwrap())
            }
        }.instrument(span))
    }
}

