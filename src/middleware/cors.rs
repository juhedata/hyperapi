use std::collections::HashMap;
use hyper::{Body, Response, StatusCode};
use hyper::http::HeaderValue;
use std::pin::Pin;
use std::future::Future;
use crate::middleware::{MwPostRequest, MwPreRequest, Middleware};
use crate::config::{RequestMatcher, ConfigUpdate, FilterSetting};


#[derive(Debug)]
pub struct CorsMiddleware {
    settings: HashMap<String, Vec<RequestMatcher>>,
}

impl Default for CorsMiddleware {
    fn default() -> Self {
        CorsMiddleware { settings: HashMap::new() }
    }
}


impl Middleware for CorsMiddleware {
    fn name(&self) -> String {
        "cors".into()
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPreRequest {mut context, request, result} = task;
        let mut path_match = false;
        match self.settings.get(&context.service_id) {
            Some(matchers) => {
                for pm in matchers {
                    if pm.is_match(&request.method(), &request.uri()) {
                        path_match = true;
                    }
                }
            },
            None => {},
        };

        if path_match && request.method() == "OPTION" {
            let mut resp = Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Body::empty())
                .unwrap();
            let headers = resp.headers_mut();
            headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, OPTIONS"));
            headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range"));
            headers.insert("Access-Control-Max-Age", HeaderValue::from_static("1728000"));
            result.send(Err(resp)).unwrap();
        } else {
            if path_match {
                context.args.insert(String::from("CORS"), String::from(""));
            }
            result.send(Ok((request, context))).unwrap();
        }
        Box::pin(async {})
    }

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPostRequest {context, mut response, result} = task;
        if context.args.contains_key("CORS") {
            let headers = response.headers_mut();
            headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, OPTIONS"));
            headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range"));
            headers.insert("Access-Control-Expose-Headers", HeaderValue::from_static("Content-Length,Content-Range"));
        }
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
                        FilterSetting::Cors(fs) => {
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


