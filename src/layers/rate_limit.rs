use redis;
use hyper::{Response, Request, Body, StatusCode};
use crate::config::{RateLimitSetting, RateLimit};
use std::collections::HashMap;
use time::OffsetDateTime;
use log;
use tower::{layer::Layer, Service};
use std::sync::Arc;
use anyhow::Error;
use std::future::Future;
use std::task::{Context, Poll};


impl<S> Layer<S> for RateLimitSetting {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {

        RateLimitService { inner }
    }
}

pub struct RateLimitService<S> {
    redis: Arc<redis::Client>,
    script: Arc<redis::Script>,
    service_id: String,
    client_id: String,
    limits: Vec<RateLimit>,
    inner: S,
}


impl<S> Service<Request<Body>> for RateLimitService<S>
    where S: Service<Request<Body>, Response=Response<Body>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let key = self.service_id + ":" + self.client_id;
        let now = OffsetDateTime::now_utc();
        for rl in self.limits.iter() {
            if let Ok(pass) = self.check(key, rl.duration, rl.limit, rl.burst, now.timestamp()) {
                if !pass {
                    let mut exceed = Response::new("Rate limit exceed".into());
                    let status = exceed.status_mut();
                    *status = StatusCode::from_u16(400).unwrap();
                    return async {
                        Ok(exceed)
                    }
                }
            } else {
                log::warn!("[RateLimit] Redis request failed");
            }
        }
        self.inner.call(req)
    }

}


impl<S> RateLimitService<S> {
    pub fn new(redis: redis::Client) -> RateLimitService {
        let script = redis::Script::new(r#"
local tokens_key = KEYS[1]
local timestamp_key = KEYS[2]

local seconds = tonumber(ARGV[1])
local rate = tonumber(ARGV[2])
local capacity = tonumber(ARGV[3])
local now = tonumber(ARGV[4])
local requested = 1

local ttl = math.floor(capacity/rate * seconds *2)

local last_tokens = tonumber(redis.call("get", tokens_key))
if last_tokens == nil then
  last_tokens = capacity
end

local last_refreshed = tonumber(redis.call("get", timestamp_key))
if last_refreshed == nil then
  last_refreshed = 0
end

local delta = math.max(0, math.floor((now-last_refreshed)/seconds))
local filled_tokens = math.min(capacity, last_tokens+(delta*rate))
local allowed = filled_tokens >= requested
local new_tokens = filled_tokens
if allowed then
  new_tokens = filled_tokens - requested
end

redis.call("setex", tokens_key, ttl, new_tokens)
redis.call("setex", timestamp_key, ttl, now)

return allowed
        "#);

        RateLimitService {redis, script}
    }

    async fn check(&self, key: &str, duration: i32, limit: i32, burst: i32, now: i64) -> Result<bool, redis::RedisError> {
        let token_key = key + ":" + duration.into() + ":v";
        let ts_key = key + ":" + duration.into() + ":t";
        let result = self.script.key(token_key).key(ts_key).arg(duration).arg(limit).arg(burst).arg(now)
            .invoke_async(&self.redis)?;
        result.await
    }
}