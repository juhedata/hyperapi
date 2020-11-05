use hyper::{Request, Body};
use crate::config::{FilterSetting, RequestMatcher};
use crate::layer::TokenBucket;
use std::time::Instant;
use anyhow::anyhow;


#[derive(Clone)]
pub struct ClientFilter {
    pub rate_limit: Vec<(RequestMatcher, Vec<TokenBucket>)>,
}

impl ClientFilter {
    pub fn new(fs: &Vec<FilterSetting>) -> Self {
        let mut rate_limit = Vec::new();

        for f in fs.iter() {
            match f {
                FilterSetting::RateLimit(c) => {
                    let re = RequestMatcher::new(c.methods.clone(), c.path_pattern.clone());
                    let rls = c.limits.iter().map(|l| TokenBucket::new(l)).collect();
                    rate_limit.push((re, rls));
                },
                _ => {}, // ignore other unsupported filters
            }
        }

        ClientFilter {
            rate_limit,
        }
    }

    pub async fn filter(&mut self, req: Request<Body>) -> Result<Request<Body>, anyhow::Error> {
        let now = Instant::now();
        let mut result = true;
        for (pt, limits) in self.rate_limit.iter_mut() {
            if pt.is_match(&req.method(), &req.uri()) {
                for rl in limits.iter_mut() {
                    if !rl.check(now) {
                        result = false;
                    }
                }
            }
        }
        if result {
            Ok(req)
        } else {
            Err(anyhow!("Rate limited"))
        }
    }
}
