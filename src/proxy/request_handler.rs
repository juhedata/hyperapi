use hyper::{Request, Response, Body};
use tokio::sync::{mpsc, oneshot};
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context};
use crate::{auth::AuthRequest, middleware::{MiddlewareHandle, RequestContext, middleware_chain}};
use tracing::{event, span, Level, Instrument};
use prometheus::{Encoder, TextEncoder};


pub struct RequestHandler {
    pub stack: Vec<MiddlewareHandle>,
    pub auth: mpsc::Sender<AuthRequest>,
    pub ready: u8,
}

impl RequestHandler {
    pub fn prometheus_endpoint(_req: &Request<Body>) -> Response<Body> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();

        let response = Response::builder()
            .status(200)
            .header(hyper::header::CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))
            .unwrap();
        response
    }
}

impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
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
        event!(Level::DEBUG, "{:?} {:?}", req.method(), req.uri());
        Box::pin(async move {
            // auth
            let (tx, rx) = oneshot::channel();
            let (head, body) = req.into_parts();
            let auth_request = AuthRequest {
                head: head,
                result: tx,
            };
            let _ = auth.send(auth_request).await;
            let auth_result = rx.await?;

            // handle request
            match auth_result {
                Ok((head_part, auth_resp)) => {
                    let req = Request::from_parts(head_part, body);
                    let context = RequestContext::new(&req, &auth_resp);
                    
                    // prometheus endpoint
                    if context.service_path.eq("/metrics") {
                        let resp = Self::prometheus_endpoint(&req);
                        return Ok(resp);
                    }
                    
                    // apply middleware chain
                    let resp = middleware_chain(req, context, stack).await;
                    match resp {
                        Ok(resp) => Ok(resp),
                        Err(err) => {
                            // TODO 
                            let msg = format!("Gateway Error: {}", err);
                            Ok(Response::builder().status(502).body(msg.into()).unwrap())
                        }
                    }
                },
                Err(err) => {
                    let msg = format!("Auth Error: {}", err);
                    Ok(Response::builder().status(502).body(msg.into()).unwrap())
                }
            }
        }.instrument(span))
    }
}

