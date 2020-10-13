use tower::layer::Layer;
use tower::Service;
use hyper::{Response, Request, Body, StatusCode};
use std::collections::HashMap;
use futures::task::Context;
use std::task::Poll;
use anyhow::Error;
use futures::Future;
use crate::proxy::config::CorsSetting;


pub struct CorsService<S> {
    pub public: bool,
    pub inner: S,
}


impl<S> Layer for CorsSetting {
    type Service = CorService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorsService { public: self.public, inner }
    }
}


impl<S> Service<Request<Body>> for CorsService<S>
    where S: Service<Request<Body>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if self.public && req.method() == "OPTIONS" {
            let mut resp = Response::new("".into());
            let status = resp.status_mut();
            *status = StatusCode::from_u16(204).unwrap();
            let headers = resp.headers_mut();
            headers.insert("Access-Control-Allow-Origin", "*".into());
            headers.insert("Access-Control-Allow-Methods", "GET, POST, OPTIONS".into());
            headers.insert("Access-Control-Allow-Headers", "DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range".into());
            headers.insert("Access-Control-Max-Age", "1728000".into());
            headers.insert("Content-Type", "text/plain; charset=utf-8".into());
            headers.insert("Content-Length", "0".into());
            return async {
                Ok(resp)
            }
        }
        return async {
            let mut resp = self.inner.call(req).await?;
            if self.public {
                let headers = resp.headers_mut();
                headers.insert("Access-Control-Allow-Origin", "*".into());
                headers.insert("Access-Control-Allow-Methods", "GET, POST, OPTIONS".into());
                headers.insert("Access-Control-Allow-Headers", "DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range".into());
                headers.insert("Access-Control-Expose-Headers", "Content-Length,Content-Range".into());
            }
            Ok(resp)
        }
    }

}

