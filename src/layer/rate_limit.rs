use hyper::{Response, Request, Body};
use crate::config::{RateLimitSetting, RateLimit, RequestMatcher};
use tower::Service;
use anyhow::{Error, anyhow};
use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::time::Instant;
use std::time::Duration;


pub struct RateLimitService<S> {
    limits: Vec<(RequestMatcher, Vec<TokenBucket>)>,
    inner: S,
}

#[derive(Clone)]
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


impl<S> RateLimitService<S> {
    pub fn new(settings: Vec<RateLimitSetting>, inner: S) -> RateLimitService<S> {
        let mut limits: Vec<(RequestMatcher, Vec<TokenBucket>)> = Vec::new();
        for s in settings.iter() {
            let rm = RequestMatcher::new(s.methods.clone(), s.path_pattern.clone());
            let bucket = s.limits.iter().map(|l| TokenBucket::new(l)).collect();
            limits.push((rm, bucket));
        }

        RateLimitService {
            limits,
            inner,
        }
    }
}


impl<S> Service<Request<Body>> for RateLimitService<S>
    where S: Service<Request<Body>, 
        Error=Error,
        Response=Response<Body>, 
        Future=Pin<Box<dyn Future<Output=Result<Response<Body>, Error>> + Send + 'static>>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;
    
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let now = Instant::now();
        let mut pass = true;
        for (pattern, limits) in self.limits.iter_mut() {
            if pattern.is_match(&req.method(), &req.uri()) {
                for rl in limits.iter_mut() {
                    if !rl.check(now) {
                        pass = false; 
                    }
                }
            }
        }
        if pass {
            self.inner.call(req)
        } else {
            Box::pin(async {
                Err(anyhow!("Rate limited"))
            })
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