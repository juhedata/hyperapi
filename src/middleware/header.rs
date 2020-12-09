
use std::collections::HashMap;
use hyper::header::{HeaderName, HeaderValue};
use tokio::sync::mpsc;
use crate::{config::ServiceInfo, middleware::MiddlewareRequest};

use super::middleware::{MwPostRequest, MwPreRequest};


#[derive(Debug, Clone)]
pub struct HeaderOperation {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}


#[derive(Debug)]
pub struct HeaderMiddleware {
    pub tx: mpsc::Sender<MiddlewareRequest>,
    rx: mpsc::Receiver<MiddlewareRequest>,
    settings: HashMap<String, HeaderOperation>,
}


impl HeaderMiddleware {

    pub fn new(config: &Vec<ServiceInfo>) -> Self {
        let (tx, rx) = mpsc::channel(10);
        let settings = HashMap::new();



        HeaderMiddleware { tx, rx, settings }
    }

    pub async fn worker(&mut self) {
        while let Some(x) = self.rx.recv().await {
            match x {
                MiddlewareRequest::Request(MwPreRequest {context, mut request, result}) => {
                    match self.settings.get(&context.service_id) {
                        Some(s) => {
                            let headers = request.headers_mut();
                            for k in s.request_remove.iter() {
                                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                                headers.remove(kn);
                            }
                            for (k, v) in s.request_inject.iter() {
                                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                                headers.insert(kn, HeaderValue::from_str(v).unwrap());
                            }
                        }
                        None => {},
                    };
                    result.send(Ok((request, context))).unwrap()
                },
                MiddlewareRequest::Response(MwPostRequest {context, mut response, result}) => {
                    match self.settings.get(&context.service_id) {
                        Some(o) => {
                            let headers = response.headers_mut();
                            for k in o.response_remove.iter() {
                                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                                headers.remove(kn);
                            }
                            for (k, v) in o.response_inject.iter() {
                                let kn = HeaderName::from_bytes(k.as_bytes()).unwrap();
                                headers.insert(kn, HeaderValue::from_str(v).unwrap());
                            }
                        },
                        None => {},
                    }
                    result.send(response).unwrap()
                },
            }
        }
    }

}
