use std::collections::HashMap;
use hyper::header::{HeaderName, HeaderValue};
use std::future::Future;
use std::pin::Pin;
use crate::middleware::{MwPostRequest, MwPreRequest, Middleware, MiddlewareRequest};
use crate::config::{ConfigUpdate, FilterSetting, HeaderSetting};


#[derive(Debug, Clone)]
pub struct HeaderOperation {
    pub request_inject: HashMap<String, String>,
    pub request_remove: Vec<String>,
    pub response_inject: HashMap<String, String>,
    pub response_remove: Vec<String>,
}

impl Default for HeaderOperation {
    fn default() -> Self {
        HeaderOperation {
            request_inject: HashMap::new(),
            request_remove: Vec::new(),
            response_inject: HashMap::new(),
            response_remove: Vec::new(),
        }
    }
}


impl HeaderOperation {
    pub fn add_setting(&mut self, setting: HeaderSetting) {
        for (k, v) in setting.request_inject {
            self.request_inject.insert(k, v);
        }
        for h in setting.request_remove {
            self.request_remove.push(h);
        }
        for (k, v) in setting.response_inject {
            self.response_inject.insert(k, v);
        }
        for h in setting.response_remove {
            self.response_remove.push(h);
        }
    }
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

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(service) => {
                let service_id = service.service_id.clone();
                let mut ops = HeaderOperation::default();
                for filter in service.filters {
                    match filter {
                        FilterSetting::Header(fs) => {
                            ops.add_setting(fs);
                        },
                        _ => {},
                    }
                }
                self.settings.insert(service_id, ops);
            },
            ConfigUpdate::ServiceRemove(sid) => {
                self.settings.remove(&sid);
            },
            _ => {},
        }
    }
}
