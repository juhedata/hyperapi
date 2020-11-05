use hyper::{Response, Request, Body};
use hyper::header::{HeaderName, HeaderValue};
use crate::config::{HeaderSetting, RequestMatcher};
use std::collections::HashMap;
use tower::Service;
use anyhow::Error;
use std::future::Future;
use std::task::{Context, Poll};
use pin_project::pin_project;
use std::pin::Pin;
use futures::ready;


pub struct HeaderService<S> {
    settings: Vec<(RequestMatcher, HeaderOperation)>,
    inner: S,
}

#[derive(Clone)]
pub struct HeaderOperation {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}


impl<S> HeaderService<S> {
    pub fn new(settings: Vec<HeaderSetting>, inner: S) -> HeaderService<S> {
        let mut st: Vec<(RequestMatcher, HeaderOperation)> = Vec::new();
        for s in settings.iter() {
            let pt = RequestMatcher::new(s.methods.clone(), s.path_pattern.clone());
            let op = HeaderOperation {
                request_inject: s.request_inject.clone(),
                request_remove: s.request_remove.clone(),
                response_inject: s.response_inject.clone(),
                response_remove: s.response_remove.clone(),
            };
            st.push((pt, op));
        }
        HeaderService {
            settings: st,
            inner,
        }
    }
}


impl<S> Service<Request<Body>> for HeaderService<S>
    where S: Service<Request<Body>, 
                    Error=Error,
                    Response=Response<Body>, 
                    Future=Pin<Box<dyn Future<Output=Result<Response<Body>, Error>> + Send + 'static>>>
{

    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let (mut head, body) = req.into_parts();
        let mut resp_ops: Vec<HeaderOperation> = Vec::new();
        for (p, s) in self.settings.iter() {
            if !p.is_match(&head.method, &head.uri) {
                continue;
            }
            for k in s.request_remove.iter() {
                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                head.headers.remove(kn);
            }
            for (k, v) in s.request_inject.iter() {
                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                head.headers.insert(kn, HeaderValue::from_str(v).unwrap());
            }
            resp_ops.push(s.clone());
        }
        let new_req = Request::from_parts(head, body);
        Box::pin(HeaderFuture::new(resp_ops, self.inner.call(new_req)))
    }

}

#[pin_project]
pub struct HeaderFuture<F> {
    ops: Vec<HeaderOperation>,
    #[pin]
    inner: F,
}

impl<F> HeaderFuture<F> {
    pub fn new(ops: Vec<HeaderOperation>, inner: F) -> HeaderFuture<F> {
        HeaderFuture { ops, inner }
    }
}


impl<F> Future for HeaderFuture<F>
    where F: Future<Output=Result<Response<Body>, Error>>
{
    type Output=F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut resp = ready!(this.inner.poll(cx))?;
        let headers = resp.headers_mut();
        for o in this.ops.iter() {
            for k in o.response_remove.iter() {
                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                headers.remove(kn);
            }
            for (k, v) in o.response_inject.iter() {
                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                headers.insert(kn, HeaderValue::from_str(v).unwrap());
            }
        }
        Poll::Ready(Ok(resp))
    }
}