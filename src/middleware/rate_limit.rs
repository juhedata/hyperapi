use std::collections::HashMap;
use hyper::{Response, Body, StatusCode};
use std::time::{Instant, Duration};
use std::future::Future;
use std::pin::Pin;
use crate::{config::RateLimitSetting, middleware::{Middleware, MwPreRequest, MwPreResponse, MwPostRequest}};
use crate::config::{ConfigUpdate, FilterSetting};


#[derive(Debug)]
pub struct RateLimitMiddleware {
    service_limit: HashMap<String, Vec<TokenBucket>>,  // service_limit[service_id] = Vec<TokenBucket>
    client_limit: HashMap<String, HashMap<String, Vec<TokenBucket>>>,   // client_limit[service_id][client_id] = Vec<TokenBucket>
    sla: HashMap<String, HashMap<String, Vec<TokenBucket>>>,  // services[service_id][sla_id] = Vec<RateLimit>
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        RateLimitMiddleware { 
            service_limit: HashMap::new(), 
            client_limit: HashMap::new(), 
            sla: HashMap::new(),
        }
    }
}


impl Middleware for RateLimitMiddleware {

    fn name() -> String {
        "RateLimit".into()
    }

    fn post() -> bool {
        false
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let now = Instant::now();
        let MwPreRequest { context, request, service_filters: _, client_filters: _, result} = task;
        let mut pass = true;
        if let Some(service_limits) = self.service_limit.get_mut(&context.service_id) {
            for limit in service_limits {
                if !limit.check(now) {
                    pass = false;
                }
            }
        }
        if let Some(clients) = self.client_limit.get_mut(&context.service_id) {
            if let Some(client_limits) = clients.get_mut(&context.client_id) {
                for limit in client_limits {
                    if !limit.check(now) {
                        pass = false;
                    }
                }
            }
        }
        
        if !pass {  // return error response
            let err = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::from("Rate limit"))
                .unwrap();
            let response = MwPreResponse { context, request: Some(request), response: Some(err) };
            let _ = result.send(response);
            Box::pin(async {})
        } else {
            let response = MwPreResponse { context, request: Some(request), response: None };
            let _ = result.send(response);
            Box::pin(async {})
        }
    }

    fn response(&mut self, _task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        panic!("never got here")
    }

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ClientUpdate(client) => {
                let mut client_limits: HashMap<String, Vec<TokenBucket>> = HashMap::new();
                for (service_id, sla_name) in &client.services {
                    if let Some(clients) = self.sla.get(service_id) {
                        if let Some(settings) = clients.get(sla_name) {
                            client_limits.insert(service_id.clone(), settings.clone());
                        }
                    }
                }
                self.client_limit.insert(client.client_id.clone(), client_limits);
            },
            ConfigUpdate::ClientRemove(client_id) => {
                for (_, clients) in self.client_limit.iter_mut() {
                    clients.remove(&client_id);
                }
            },
            ConfigUpdate::ServiceUpdate(service) => {
                let mut service_limits: Vec<TokenBucket> = Vec::new();

                for filter in &service.filters {
                    if let FilterSetting::RateLimit(f) = filter {
                        service_limits.push(TokenBucket::new(f));
                    }
                }
                self.service_limit.insert(service.service_id.clone(), service_limits);

                let mut service_sla: HashMap<String, Vec<TokenBucket>> = HashMap::new();
                for sla in &service.sla {
                    for filter in &sla.filters {
                        if let FilterSetting::RateLimit(f) = filter {
                            if let Some(ssla) = service_sla.get_mut(&sla.name) {
                                ssla.push(TokenBucket::new(f));
                            } else {
                                service_sla.insert(sla.name.clone(), vec![TokenBucket::new(f)]);
                            }
                        }
                    }
                }
                self.sla.insert(service.service_id.clone(), service_sla);
            },
            ConfigUpdate::ServiceRemove(service_id) => {
                self.service_limit.remove(&service_id);
                self.client_limit.remove(&service_id);
            },
            _ => {},
        }
    }

}



#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub interval: Duration,
    pub limit: u64,
    pub capacity: u64,
    refresh_at: Instant,
    tokens: u64,
}


impl TokenBucket {

    pub fn new(limit: &RateLimitSetting) -> Self {
        TokenBucket {
            interval: Duration::from_secs(limit.interval as u64),
            limit: limit.limit as u64,
            capacity: limit.burst as u64,
            refresh_at: Instant::now(),
            tokens: limit.limit as u64,
        }
    }

    pub fn check(&mut self, now: Instant) -> bool {
        let request = 1;
        let delta = now.duration_since(self.refresh_at).as_secs() / self.interval.as_secs();
        let token_count = std::cmp::min(self.capacity, self.tokens + delta * self.limit);
        if token_count > request {
            self.tokens = token_count - request;
            if delta > 0 {
                self.refresh_at = now;
            }
            true
        } else {
            false
        }
    }
}


