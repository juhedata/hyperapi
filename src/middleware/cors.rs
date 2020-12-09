use std::collections::HashMap;
use hyper::{Body, Response, StatusCode};
use hyper::http::HeaderValue;
use tokio::sync::mpsc;
use crate::middleware::MiddlewareRequest;
use crate::config::RequestMatcher;
use super::middleware::{MwPostRequest, MwPreRequest};


#[derive(Debug)]
pub struct CorsMiddleware {
    pub tx: mpsc::Sender<MiddlewareRequest>,
    rx: mpsc::Receiver<MiddlewareRequest>,
    settings: HashMap<String, Vec<RequestMatcher>>,
}


impl CorsMiddleware {

    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        let settings = HashMap::new();
        CorsMiddleware { tx, rx, settings }
    }

    pub async fn worker(&mut self) {
        while let Some(x) = self.rx.recv().await {
            match x {
                MiddlewareRequest::Request(MwPreRequest {context, request, result}) => {
                    let setting = self.settings.get(&context.service_id);

                    if request.method() == "OPTION" {
                        let mut resp = Response::builder().status(StatusCode::NO_CONTENT).body(Body::empty()).unwrap();
                        let headers = resp.headers_mut();
                        headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
                        headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, OPTIONS"));
                        headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range"));
                        headers.insert("Access-Control-Max-Age", HeaderValue::from_static("1728000"));
                        result.send(Err(resp)).unwrap()
                    } else {
                        result.send(Ok((request, context))).unwrap()
                    }
                },
                MiddlewareRequest::Response(MwPostRequest {context, mut response, result}) => {
                    let headers = response.headers_mut();
                    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
                    headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, OPTIONS"));
                    headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range"));
                    headers.insert("Access-Control-Expose-Headers", HeaderValue::from_static("Content-Length,Content-Range"));
                    result.send(response).unwrap()
                },
            }
        }
    }

}



