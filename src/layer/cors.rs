use tower::Service;
use hyper::{Request, Response, Body, StatusCode, http::HeaderValue};
use std::task::{Context, Poll};
use std::future::Future;
use crate::config::CorsSetting;
use regex::Regex;
use pin_project::pin_project;
use std::pin::Pin;
use futures::ready;
use anyhow::Error;


pub struct CorsService<S> {
    pub patterns: Vec<Regex>,
    pub inner: S,
}


impl<S> CorsService<S> {

    pub fn new(settings: Vec<CorsSetting>, inner: S) -> CorsService<S> {
        let mut patterns = Vec::new();
        for s in settings {
            if !s.public {
                continue;
            }
            let p = Regex::new(&s.path_pattern).unwrap();
            patterns.push(p);
        }
        CorsService { patterns, inner }
    }
}


impl<S> Service<Request<Body>> for CorsService<S>
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
        let mut public = false;
        let path = req.uri().path().strip_prefix("/").unwrap();
        let (_service_id, path_left) = path.split_at(path.find("/").unwrap());
        for p in self.patterns.iter() {
            if p.is_match(path_left) {
                public = true;
                break;
            }
        }

        if public && req.method() == "OPTIONS" {
            let mut resp = hyper::Response::new("".into());
            let status = resp.status_mut();
            *status = StatusCode::from_u16(204).unwrap();
            let headers = resp.headers_mut();
            headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, OPTIONS"));
            headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range"));
            headers.insert("Access-Control-Max-Age", HeaderValue::from_static("1728000"));
            headers.insert("Content-Type", HeaderValue::from_static("text/plain; charset=utf-8"));
            headers.insert("Content-Length", HeaderValue::from_static("0"));
            return Box::pin(async {
                Ok(resp)
            })
        }
        Box::pin(CorsFuture::new(public, self.inner.call(req)))
    }

}


#[pin_project]
pub struct CorsFuture<F> {
    #[pin]
    f: F,
    public: bool,
}

impl<F> CorsFuture<F> {
    pub fn new(public: bool, inner: F) -> CorsFuture<F> {
        CorsFuture {
            f: inner,
            public,
        }
    }
}

impl<F> Future for CorsFuture<F>
    where F: Future<Output=Result<Response<Body>, Error>>
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut resp = ready!(this.f.poll(cx))?;
        if *this.public {
            let headers = resp.headers_mut();
            headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, OPTIONS"));
            headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range"));
            headers.insert("Access-Control-Expose-Headers", HeaderValue::from_static("Content-Length,Content-Range"));
        }
        Poll::Ready(Ok(resp))
    }
}

