use hyper::{Response, Request, Body};
use crate::config::HeaderSetting;
use std::collections::HashMap;
use tower::{layer::Layer, Service};
use anyhow::Error;
use std::future::Future;
use std::task::{Context, Poll};


impl<S> Layer<S> for HeaderSetting {
    type Service = HeaderService<S>;

    fn layer(&self, inner: S) -> Self::Service {

        HeaderService {
            request_inject: self.request_inject.clone(),
            request_remove: self.request_remove.clone(),
            response_inject: self.response_inject.clone(),
            response_remove: self.response_remove.clone(),
            inner,
        }
    }
}

pub struct HeaderService<S> {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
    inner: S,
}

impl<S> Service<Request<Body>> for HeaderService<S>
    where S: Service<Request<Body>, Response=Response<Body>>
{

    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut req_mut = Clone::clone(&req);
        let headers = req_mut.headers_mut();
        for k in self.request_remove.iter() {
            headers.remove(k);
        }
        for (k, v) in self.request_inject.iter() {
            headers.insert(k, v.into());
        }

        return async {
            let mut resp = self.inner.call(req_mut).await?;

            let headers = resp.headers_mut();
            for k in self.response_remove.iter() {
                headers.remove(k);
            }
            for (k, v) in self.response_inject.iter() {
                headers.insert(k, v.into());
            }
            Ok(resp)
        }
    }

}

