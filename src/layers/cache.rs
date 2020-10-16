use hyper::{Response, Request, Body, StatusCode};
use crate::config::CacheSetting;
use std::collections::HashMap;
use redis::{Client, Script, RedisError};
use mlua::{Lua, StdLib};
use anyhow::Error;
use tower::{layer::Layer, Service};
use std::future::Future;
use std::task::{Context, Poll};


impl<S> Layer<S> for CacheSetting {
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let lua_features = StdLib::TABLE | StdLib::JIT | StdLib::STRING | StdLib::BIT | StdLib::MATH;
        let vm = Lua::new_with(lua_features).unwrap();
        CacheService { lua: vm, inner }
    }
}


pub struct CacheService<S> {
    redis: redis::Client,
    lua: Lua,
    inner: S
}



impl<S> Service<Request<Body>> for CacheService<S>
    where S: Service<Request<Body>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // todo
        self.inner.call(req)
    }

}


impl<S> CacheService<S> {

    pub fn set(&self, ns: &str, key: &str, data: &str, ttl: i32, max_entry: i32) -> Result<(), String> {

        Ok(())
    }

    pub fn get(&self, ns: &str, key: &str) -> Result<String, String> {

        Ok(String::from(""))
    }
}

