use hyper::{Request, Body, StatusCode, Response};
use regex::Regex;
use tracing::{event, Level};
use std::{collections::HashMap, str::FromStr};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use crate::middleware::{MwPostRequest, MwPreRequest, MwPreResponse, Middleware};
use crate::config::{ConfigUpdate, FilterSetting, ACLSetting};


#[derive(Debug)]
pub struct ACLMiddleware {
    service_acl: HashMap<String, HashMap<String, Vec<ACLMatcher>>>,   // service_acl[service_id][sla] = Vec<PathMatcher>
}


impl Default for ACLMiddleware {
    fn default() -> Self {
        ACLMiddleware { service_acl: HashMap::new() }
    }
}


impl Middleware for ACLMiddleware {

    fn name() -> String {
        "ACL".into()
    }

    fn post() -> bool {
        false
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPreRequest {context, request, service_filters: _, client_filters: _, result} = task;
        let mut pass = true;
        if let Some(settings) = self.service_acl.get(&context.service_id) {
            if let Some(acl) = settings.get(&context.sla) {
                for m in acl {
                    if !m.check(&request) {
                        pass = false;
                        break;
                    }
                }
            }
        }
        if pass {
            let response = MwPreResponse {context: context, request: Some(request), response: None };
            let _ = result.send(response);
        } else {
            let err = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 Not Found"))
                .unwrap();
            let response = MwPreResponse { context, request: Some(request), response: Some(err) };
            let _ = result.send(response);
        }
        Box::pin(async {})
    }

    fn response(&mut self, _task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        panic!("Never got here")
    }

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ServiceUpdate(service) => {
                let mut matchers = Vec::new();
                for filter in service.filters {
                    if let FilterSetting::ACL(acl) = filter {
                        matchers.push(ACLMatcher::new(&acl));
                    }
                }
                let mut service_acl = HashMap::new();
                for sla in service.sla {
                    let mut m = matchers.clone();
                    for filter in &sla.filters {
                        if let FilterSetting::ACL(acl) = filter {
                            m.push(ACLMatcher::new(acl));
                        }
                    }
                    service_acl.insert(sla.name.clone(), m);
                }
                self.service_acl.insert(service.service_id.clone(), service_acl);
            },
            ConfigUpdate::ServiceRemove(service_id) => {
                self.service_acl.remove(&service_id);
            },
            _ => {},
        }
    }
}


#[derive(Debug, Clone)]
pub struct ACLMatcher{
    on_match: bool,
    paths: Vec<(Regex, HashSet<String>)>,
}


impl ACLMatcher {

    pub fn new(setting: &ACLSetting) -> Self {
        let on_match = setting.access_control == "allow";
        let mut paths = Vec::new();
        for p in &setting.paths {
            let mut methodset: HashSet<String> = HashSet::new();
            let msplit = p.methods.split(",");
            for m in msplit {
                methodset.insert(String::from(m));
            }
            if let Ok(regex) = Regex::from_str(&p.path_regex) {
                paths.push((regex, methodset));
            } else {
                event!(Level::ERROR, "bad regex {}", p.path_regex);
            }
        }
        ACLMatcher { on_match, paths }
    }

    pub fn check(&self, req: &Request<Body>) -> bool {
        let method = req.method().as_str();
        let path = req.uri().path();
        let path = path.strip_prefix("/").unwrap_or(path);

        for (path_regex, methodset) in &self.paths {
            if methodset.contains(method) {
                let (_sid, path_left) = path.split_at(path.find("/").unwrap_or(0));
                if path_regex.is_match(path_left) {
                    return self.on_match
                }
            }
        }
        !self.on_match
    }
}