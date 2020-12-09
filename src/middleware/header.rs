use std::collections::HashMap;
use hyper::header::{HeaderName, HeaderValue};
use std::future::Future;
use std::pin::Pin;
use super::middleware::{MwPostRequest, MwPreRequest, Middleware, MiddlewareRequest};


#[derive(Debug, Clone)]
pub struct HeaderOperation {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}


#[derive(Debug)]
pub struct HeaderMiddleware {
    settings: HashMap<String, HeaderOperation>,
}

impl Default for HeaderMiddleware {
    fn default() -> Self {
        HeaderMiddleware { settings: HashMap::new() }
    }
}


impl Middleware for HeaderMiddleware {


    fn work(&mut self, task: MiddlewareRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        match task {
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
                result.send(Ok((request, context))).unwrap();
                Box::pin(async {})
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
                result.send(response).unwrap();
                Box::pin(async {})
            },
        }
    }

    fn config_update(&mut self, update: crate::config::ConfigUpdate) {
        todo!()
    }
}
