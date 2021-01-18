use std::collections::HashMap;
use hyper::{Body, Response, StatusCode};
use hyper::http::HeaderValue;
use std::pin::Pin;
use std::future::Future;
use crate::middleware::{MwPostRequest, MwPreRequest, Middleware};
use crate::config::{RequestMatcher, ConfigUpdate, FilterSetting};


#[derive(Debug)]
pub struct LoggerMiddleware {
    settings: HashMap<String, Vec<RequestMatcher>>,
}

impl Default for LoggerMiddleware {
    fn default() -> Self {
        LoggerMiddleware { settings: HashMap::new() }
    }
}


impl Middleware for LoggerMiddleware {
    fn name(&self) -> String {
        "logger".into()
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPreRequest {mut context, request, result} = task;
        Box::pin(async {
            result.send(Ok((request, context))).unwrap();
        })
    }

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPostRequest {context, mut response, result} = task;
        // todo write log
        result.send(response).unwrap();
        Box::pin(async {})
    }

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(service) => {
                let service_id = service.service_id.clone();
                let mut spec = Vec::new();
                for filter in service.filters {
                    match filter {
                        FilterSetting::Logger(fs) => {
                            spec.push(RequestMatcher::new(fs.methods, fs.path_pattern))
                        },
                        _ => {},
                    }
                }
                if spec.len() > 0 {
                    self.settings.insert(service_id, spec);
                } else {
                    self.settings.remove(&service_id);
                }
            },
            ConfigUpdate::ServiceRemove(sid) => {
                self.settings.remove(&sid);
            },
            _ => {},
        }
    }
}



