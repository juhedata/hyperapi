use std::collections::HashMap;
use hyper::{Request, Response, Body};
use std::time::{Instant, Duration};
use std::future::Future;
use std::pin::Pin;
use crate::{config::ConfigUpdate, middleware::MiddlewareRequest};
use crate::config::RateLimit;

use super::{Middleware, middleware::MwPreRequest};


#[derive(Debug)]
pub struct RateLimitMiddleware {
    limiter: HashMap<String, Vec<TokenBucket>>,
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
                let limit_key = extract_ratelimit_key(&request);
                match self.limiter.get_mut(&limit_key) {
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
                            let err = Response::new(Body::from("Ratelimit"));
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
        todo!()
    }

}


fn extract_ratelimit_key(req: &Request<Body>) -> String {

    todo!()
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
