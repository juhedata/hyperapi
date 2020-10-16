use hyper::{Response, Request, Body, StatusCode};
use crate::config::LoggingSetting;
use std::collections::HashMap;
use log;
use mlua::{Lua, StdLib};
use tower::{layer::Layer, Service};
use std::future::Future;
use std::task::{Context, Poll};
use anyhow::Error;


impl<S> Layer<S> for LoggingSetting {
    type Service = CorService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let lua_features = StdLib::TABLE | StdLib::JIT | StdLib::STRING | StdLib::BIT | StdLib::MATH;
        let vm = Lua::new_with(lua_features).unwrap();
        LoggingService { lua: vm, inner }
    }
}


pub struct LoggingService<S> {
    pub message_expr: String,
    pub condition_expr: String,
    pub category: String,
    pub level: String,
    pub pre_request: bool,
    pub post_request: bool,
    lua: Lua,
    inner: S
}


impl<S> Service<Request<Body>> for LoggingService<S>
    where S: Service<Request<Body>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        self.inner.call(req)
    }

}

