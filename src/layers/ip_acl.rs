use hyper::{Response, Request, Body, StatusCode};
use crate::config::IPAclSetting;
use std::collections::HashMap;
use tower::{layer::Layer, Service};
use anyhow::Error;
use std::future::Future;
use std::task::{Context, Poll};


impl<S> Layer<S> for IPAclSetting {
    type Service = IPAclService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IPAclService {
            white_list: self.white_list.clone(),
            black_list: self.black_list.clone(),
            inner,
        }
    }
}

pub struct IPAclService<S> {
    white_list: Vec<String>,
    black_list: Vec<String>,
    inner: S,
}


impl<S> Service<Request<Body>> for IPAclService<S>
    where S: Service<Request<Body>, Response=Response<Body>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let ip = get_remote_address(&req);
        let mut block = Response::new("Blocked".into());
        let status = block.status_mut();
        *status = StatusCode::from_u16(403).unwrap();

        if s.white_list.len() > 0 {
            for w in s.white_list.iter() {
                if ip.eq(w) {
                    return self.inner.call(req);
                }
            }
            return async {
                Ok(block)
            }
        } else {
            for b in s.black_list.iter() {
                if ip.eq(b) {
                    return async {
                        Ok(block)
                    }
                }
            }
            self.inner.call(req)
        }
    }

}


fn get_remote_address(req: &Request<Body>) -> &str {

    return ""
}

