use std::collections::HashMap;
use hyper::{Response, Body, StatusCode};
use std::time::{Instant, Duration};
use std::future::Future;
use std::pin::Pin;
use crate::middleware::{Middleware, MwPreRequest, MiddlewareRequest};
use crate::config::{ConfigUpdate, FilterSetting, RateLimit};


#[derive(Debug)]
pub struct RateLimitMiddleware {
    limiter: HashMap<String, HashMap<String, Vec<TokenBucket>>>,
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        RateLimitMiddleware { limiter: HashMap::new() }
    }
}


impl Middleware for RateLimitMiddleware {

    fn work(&mut self, task: MiddlewareRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let now = Instant::now();
        match task {
            MiddlewareRequest::Request(MwPreRequest { context, request, result}) => {
                let service_id = context.service_id.clone();
                let client = context.client.clone();
                let setting = client
                    .map(|c| self.limiter.get_mut(&c.app_key))
                    .flatten()
                    .map(|sl| sl.get_mut(&service_id))
                    .flatten();

                match setting {
                    Some(buckets) => {
                        let mut pass = true;
                        for limit in buckets.iter_mut() {
                            if !limit.check(now) {
                                pass = false;
                            }
                        }
                        if pass {
                            result.send(Ok((request, context))).unwrap();
                        } else {
                            let err = Response::builder()
                                .status(StatusCode::TOO_MANY_REQUESTS)
                                .body(Body::from("Rate limit"))
                                .unwrap();
                            result.send(Err(err)).unwrap();
                        }
                    },
                    None => {
                        result.send(Ok((request, context))).unwrap();
                    },
                };
                Box::pin(async {})
            },
            MiddlewareRequest::Response(resp) => Box::pin(async {
                resp.result.send(resp.response).unwrap();
            }),
        }
    }

    fn config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::ClientUpdate(client) => {
                let mut limits = HashMap::new();
                for (service_id, filters) in client.services {
                    let mut buckets: Vec<TokenBucket> = Vec::new();
                    for fs in filters {
                        match fs {
                            FilterSetting::RateLimit(s) => {
                                let b = s.limits.iter().map(|rl| TokenBucket::new(rl));
                                buckets.extend(b);
                            },
                            _ => {},
                        }
                    }
                    if buckets.len() > 0 {
                        limits.insert(service_id, buckets);
                    }
                }
                self.limiter.insert(client.app_key.clone(), limits);
            },
            ConfigUpdate::ClientRemove(app_key) => {
                self.limiter.remove(&app_key);
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

    pub fn new(limit: &RateLimit) -> Self {
        TokenBucket {
            interval: Duration::from_secs(limit.duration as u64),
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





// impl<S> RedisRateLimitService<S> {
//     pub fn new(redis: redis::Client) -> RedisRateLimitService {
//         let script = redis::Script::new(r#"
// local tokens_key = KEYS[1]
// local timestamp_key = KEYS[2]

// local seconds = tonumber(ARGV[1])
// local rate = tonumber(ARGV[2])
// local capacity = tonumber(ARGV[3])
// local now = tonumber(ARGV[4])
// local requested = 1

// local ttl = math.floor(capacity/rate * seconds * 2)

// local last_tokens = tonumber(redis.call("get", tokens_key))
// if last_tokens == nil then
//   last_tokens = capacity
// end

// local last_refreshed = tonumber(redis.call("get", timestamp_key))
// if last_refreshed == nil then
//   last_refreshed = 0
// end

// local delta = math.max(0, math.floor((now - last_refreshed) / seconds))
// local filled_tokens = math.min(capacity, last_tokens + (delta * rate))
// local allowed = filled_tokens >= requested
// local new_tokens = filled_tokens
// if allowed then
//   new_tokens = filled_tokens - requested
// end

// redis.call("setex", tokens_key, ttl, new_tokens)
// redis.call("setex", timestamp_key, ttl, now)

// return allowed
//         "#);

//         RedisRateLimitService {redis, script}
//     }

//     async fn check(&self, key: &str, duration: i32, limit: i32, burst: i32, now: i64) -> Result<bool, redis::RedisError> {
//         let token_key = key + ":" + duration.into() + ":v";
//         let ts_key = key + ":" + duration.into() + ":t";
//         let result = self.script.key(token_key).key(ts_key).arg(duration).arg(limit).arg(burst).arg(now)
//             .invoke_async(&self.redis)?;
//         result.await
//     }
// }
