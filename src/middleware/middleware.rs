use std::{collections::HashMap, pin::Pin, time::SystemTime};
use hyper::{Request, Response, Body};
use tokio::sync::oneshot::error::RecvError;
use tokio::sync::{mpsc, broadcast};
use tokio::sync::oneshot;
use std::future::Future;
use tracing::{span, Level, Instrument};
use crate::{auth::AuthResponse, config::ConfigUpdate, config::FilterSetting};
use uuid::Uuid;
use thiserror::Error;


#[derive(Error, Debug, Clone)]
pub enum GatewayError { 
    #[error("Upstream request timeout")]
    UpstreamRequestError(String),

    #[error("Upstream request timeout")]
    TimeoutError,

    #[error("Service not found")]
    ServiceNotFound(String),

    #[error("Service not ready")]
    ServiceNotReady(String),

    #[error("Upstream error")]
    UpstreamError(String),

    #[error("Rate Limit")]
    RateLimited(String),

    #[error("URL Access Deny")]
    AccessBlocked(String),

    #[error("Interal server error")]
    GatewayInteralError(String),

    #[error("Middleware comm error")]
    ChannelRecvError(String),

    #[error("Unknown auth error")]
    Unknown,
}

impl From<hyper::Error> for GatewayError {
    fn from(_e: hyper::Error) -> Self {
        // TODO
        GatewayError::UpstreamError("upstream error".into())
    }
}

impl From<RecvError> for GatewayError {
    fn from(_e: RecvError) -> Self {
        // TODO
        GatewayError::ChannelRecvError("Channel Receive".into())
    }
}


#[derive(Clone)]
pub struct MiddlewareHandle {
    pub name: String,
    pub pre: bool,
    pub post: bool, 
    pub require_setting: bool,
    pub chan: mpsc::Sender<MiddlewareRequest>,
}


#[derive(Debug)]
pub struct MwPreRequest {
    pub context: RequestContext,
    pub request: Request<Body>,
    pub service_filters: Vec<FilterSetting>,
    pub client_filters: Vec<FilterSetting>,
    pub result: oneshot::Sender<Result<MwPreResponse, GatewayError>>,
}


#[derive(Debug)]
pub struct MwPreResponse {
    pub context: RequestContext,
    pub next: MwNextAction,
}

#[derive(Debug)]
pub enum MwNextAction {
    Next(Request<Body>),
    Return(Response<Body>),
}

#[derive(Debug)]
pub struct MwPostRequest {
    pub context: RequestContext,
    pub response: Response<Body>,
    pub service_filters: Vec<FilterSetting>,
    pub client_filters: Vec<FilterSetting>,
    pub result: oneshot::Sender<Result<MwPostResponse, GatewayError>>,
}


#[derive(Debug)]
pub struct MwPostResponse {
    pub context: RequestContext,
    pub response: Response<Body>,
}


#[derive(Debug)]
pub enum MiddlewareRequest {
    Request(MwPreRequest),
    Response(MwPostRequest),
}


pub trait Middleware {
    fn name() -> String;

    // this middleware intercept request 
    fn pre() -> bool {
        true
    }

    // this middleware handles response
    fn post() -> bool {
        true
    }

    // this middlewares requires setting to work
    fn require_setting() -> bool {
        true
    }

    // pre-request handler, result is sent-back through one-shot channel in MwPreRequest
    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    // post-response handler, result is sent-back through one-shot channel in MwPostRequest
    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    // handle config update events
    fn config_update(&mut self, update: ConfigUpdate);
}


#[derive(Debug, Clone)]
pub struct RequestContext {
    pub service_id: String,
    pub client_id: String,
    pub service_path: String,
    pub api_path: String,
    pub sla: String,
    pub start_time: SystemTime,
    pub service_filters: HashMap<String, Vec<FilterSetting>>,
    pub client_filters: HashMap<String, Vec<FilterSetting>>,
    pub request_id: Uuid,
}

impl RequestContext {
    pub fn new(req: &Request<Body>, auth: &AuthResponse) -> Self {
        let req_id = Self::extract_request_id(req);
        let (service_path, api_path) = Self::split_path(req.uri().path());
        let mut context = RequestContext {
            service_id: auth.service_id.clone(),
            client_id: auth.client_id.clone(),
            service_path,
            api_path,
            sla: auth.sla.clone(),
            start_time: SystemTime::now(),
            service_filters: HashMap::new(),
            client_filters: HashMap::new(),
            request_id: req_id,
        };
        
        // group FilterSettings by Middlewares
        for sf in &auth.service_filters {
            let filter_type = FilterSetting::get_type(&sf);
            if let Some(filters) = context.service_filters.get_mut(&filter_type) {
                filters.push(sf.clone());
            } else {
                context.service_filters.insert(filter_type, vec![sf.clone()]);
            }
        }
        for cf in &auth.client_filters {
            let filter_type = FilterSetting::get_type(&cf);
            if let Some(filters) = context.client_filters.get_mut(&filter_type) {
                filters.push(cf.clone());
            } else {
                context.client_filters.insert(filter_type, vec![cf.clone()]);
            }
        }
        context
    }

    fn split_path(path: &str) -> (String, String) {
        let path = path.strip_prefix("/").unwrap_or(path);
        let (service_path, api_path) = match path.find("/") {
            Some(pos) => {
                path.split_at(pos)
            },
            None => {
                (path, "/")
            }
        };
        (format!("/{}", service_path), String::from(api_path))
    }

    fn extract_request_id(_req: &Request<Body>) -> Uuid {
        // if let Some(value) = req.headers().get("request_id".into()) {
        //     if let Ok(id) = Uuid::parse_str(value.to_str().unwrap_or("")) {
        //         return id
        //     }
        // }
        Uuid::new_v4()
    }
}


pub async fn start_middleware<MW>(mut tasks: mpsc::Receiver<MiddlewareRequest>, mut updates: broadcast::Receiver<ConfigUpdate>) 
where MW: Middleware + Default
{
    let mut mw = MW::default();

    loop {
        tokio::select! {
            task = tasks.recv() => {
                match task {
                    Some(MiddlewareRequest::Request(x)) => {
                        let ctx = x.context.clone();
                        let span = span!(Level::DEBUG, "pre_filter",
                                        service=ctx.service_id.as_str(),
                                        trace_id=ctx.request_id.to_string().as_str(),
                                        app_id=ctx.client_id.as_str(),
                                        middleware=MW::name().as_str());
                        mw.request(x).instrument(span).await;
                    },
                    Some(MiddlewareRequest::Response(x)) => {
                        let ctx = x.context.clone();
                        let span = span!(Level::DEBUG, "post_filter",
                                        service=ctx.service_id.as_str(),
                                        trace_id=ctx.request_id.to_string().as_str(),
                                        app_id=ctx.client_id.as_str(),
                                        middleware=MW::name().as_str());
                        mw.response(x).instrument(span).await;
                    },
                    None => {},
                }
            },
            update = updates.recv() => {
                match update {
                    Ok(c) => {
                        mw.config_update(c);
                    },
                    Err(_e) => {},
                }
            },
        }
    }
}

// recursively apply middlewares
pub fn middleware_chain(req: Request<Body>, context: RequestContext, mut mw_stack: Vec<MiddlewareHandle>)
        -> Pin<Box<dyn Future<Output=Result<Response<Body>, GatewayError>> + Send>> 
{
    let mw = mw_stack.pop();
    if mw.is_none() {
        // middleware stack is empty, and no Response obtained, return error
        return Box::pin(async {
            Err(GatewayError::GatewayInteralError("Middleware misconfiguration".into()))
        })
    }

    let MiddlewareHandle {name, chan, pre, post, require_setting} = mw.unwrap();
    // extract middleware settings from context
    let service_filters = {
        if let Some(sfs) = context.service_filters.get(&name) {
            sfs.clone()
        } else {
            Vec::new()
        }
    };
    let client_filters = {
        if let Some(cfs) = context.client_filters.get(&name) {
            cfs.clone()
        } else {
            Vec::new()
        }
    };
    // clone settings for later use
    let resp_service_filters = service_filters.clone();
    let resp_client_filters = client_filters.clone();

    // if middleware requires setting to work, and settings are empty, skip this middleware
    if require_setting && service_filters.len() == 0 && client_filters.len() == 0 {
        return middleware_chain(req, context, mw_stack);
    }

    let fut = async move {
        // request middleware pre-filter
        let pre_resp: Result<MwPreResponse, GatewayError> = {
            if pre {
                let (tx, rx) = oneshot::channel();
                let pre_req = MwPreRequest {
                    context,
                    request: req,
                    service_filters: service_filters,
                    client_filters: client_filters,
                    result: tx,
                };
                let _ = chan.send(MiddlewareRequest::Request(pre_req)).await;

                let result = rx.await??;
                Ok(result)
            } else {
                Ok(MwPreResponse { context, next: MwNextAction::Next(req) })
            }
        };

        let MwPreResponse { context, next } = pre_resp?;

        match next {
            // call inner middleware
            MwNextAction::Next(request) => {
                let context_copy = context.clone();
                let inner_resp = middleware_chain(request, context, mw_stack).await?;

                // call middleware post-filter
                if post {
                    let (tx, rx) = oneshot::channel();
                    let post_req = MwPostRequest {
                        context: context_copy,
                        response: inner_resp,
                        service_filters: resp_service_filters,
                        client_filters: resp_client_filters,
                        result: tx,
                    };
                    let _ = chan.send(MiddlewareRequest::Response(post_req)).await;
                    let resp =rx.await??;
                    Ok(resp.response)
                } else {
                    Ok(inner_resp)
                }
            },
            // if pre-filter returns response, terminate chain and return
            MwNextAction::Return(response) => {
                Ok(response)
            }
        }
    };

    Box::pin(fut)
}

