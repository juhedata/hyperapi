use hyper::{Request, Body};
use regex::Regex;
use crate::config::{FilterSetting};
use crate::layer::TokenBucket;


#[derive(Clone)]
pub struct ClientFilter {
    pub rate_limit: Vec<(Regex, Vec<TokenBucket>)>,
}

impl ClientFilter {
    pub fn new(fs: &Vec<FilterSetting>) -> Self {
        let mut rate_limit = Vec::new();

        for f in fs.iter() {
            match f {
                FilterSetting::RateLimit(c) => {
                    let re = Regex::new(&c.path_pattern).unwrap();
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

        Ok(req)
    }
}
