use hyper::{Response, Request, Body};
use std::collections::HashMap;
use tower::{Service, layer::Layer};
use mlua::{Function, Lua, MetaMethod, UserData, UserDataMethods, Variadic, StdLib};
use mlua::Result as LuaResult;
use anyhow::Error;
use std::future::Future;
use std::task::Context;
use std::task::Poll;
use crate::config::ScriptSetting;


pub struct ScriptService<S> {
    pub request_expr: Option<String>,
    pub response_expr: Option<String>,
    lua: Lua,
    inner: S,
}


impl<S> Layer<S> for ScriptSetting {
    type Service = ScriptService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let lua_features = StdLib::TABLE | StdLib::JIT | StdLib::STRING | StdLib::BIT | StdLib::MATH;
        let vm = Lua::new_with(lua_features).unwrap();
        ScriptService {
            request_expr: self.request_expr.clone(),
            response_expr: self.response_expr.clone(),
            lua: vm,
            inner,
        }
    }
}

impl<S> Service<Request<Body>> for ScriptService<S>
    where S: Service<Request<Body>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut req_new = Option::Some(req);
        if self.request_expr.is_some() {
            let result = self.lua.scope(|scope| {
                self.lua.globals().set("request", &req);
                self.lua.load(self.request_expr).eval::<Request<Body>>()
            });
            if result.is_ok() {
                req_new.replace(result.unwrap());
            }
        }

        async {
            let mut resp = self.inner.call(req_new.unwrap()).await?;
            if self.response_expr.is_some() {
                let result = self.lua.scope(|scope| {
                    self.lua.globals().set("request", req_new.unwrap());
                    self.lua.globals().set("response", resp);
                    self.lua.load(self.request_expr).eval::<Response<Body>>()
                });
                return result;
            }
            Ok(resp)
        }
    }

}

