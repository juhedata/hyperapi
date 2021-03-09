use hyper::{Request, Response, Body};
use tokio::sync::{mpsc, oneshot};
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use std::task::{Poll, Context};
use crate::{auth::AuthRequest, middleware::{middleware_chain, RequestContext, MiddlewareRequest}};

use tracing::{event, span, Level, Instrument};


pub struct RequestHandler {
    pub stack: Vec<(String, mpsc::Sender<MiddlewareRequest>)>,
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
        let timer = Instant::now();
        let stack = self.stack.clone();
        let auth = self.auth.clone();

        let context = RequestContext::new(&req);
        let span = span!(Level::DEBUG, "request",
                                service=context.service_id.as_str(),
                                trace_id=context.request_id.to_string().as_str());
        event!(Level::INFO, "{:?} {:?}", req.method(), req.uri());
        Box::pin(async move {
            // auth
            let (tx, rx) = oneshot::channel();
            let (head, body) = req.into_parts();
            let auth_request = AuthRequest {
                head: head,
                result: tx,
            };
            auth.send(auth_request);
            let auth_resp = rx.await;

            // handle request
            if let Ok(auth_resp) = auth_resp {
                let req = Request::from_parts(auth_resp.head, body);
                let resp = middleware_chain(req, context, stack).await;
                Ok(resp)
            } else {
                Ok(Response::new("Auth Failed".into()))
            }
        }.instrument(span))
    }
}

