use std::{collections::HashMap, pin::Pin};
use hyper::{Request, Response, Body, StatusCode};
use tokio::sync::{mpsc, broadcast};
use tokio::sync::oneshot;
use std::future::Future;
use tracing::{span, Level, Instrument};
use crate::{auth::AuthResponse, config::ConfigUpdate, config::FilterSetting};
use uuid::Uuid;

#[derive(Clone)]
pub struct MiddlewareHandle {
    pub name: String,
    pub pre: bool,
    pub post: bool, 
    pub chan: mpsc::Sender<MiddlewareRequest>,
}


#[derive(Debug)]
pub struct MwPreRequest {
    pub context: RequestContext,
    pub request: Request<Body>,
    pub service_filters: Vec<FilterSetting>,
    pub client_filters: Vec<FilterSetting>,
    pub result: oneshot::Sender<MwPreResponse>,
}


#[derive(Debug)]
pub struct MwPreResponse {
    pub context: RequestContext,
    pub request: Option<Request<Body>>,
    pub response: Option<Response<Body>>,
}


#[derive(Debug)]
pub struct MwPostRequest {
    pub context: RequestContext,
    pub response: Response<Body>,
    pub service_filters: Vec<FilterSetting>,
    pub client_filters: Vec<FilterSetting>,
    pub result: oneshot::Sender<MwPostResponse>,
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

    fn pre() -> bool {
        true
    }

    fn post() -> bool {
        true
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    fn config_update(&mut self, update: ConfigUpdate);
}


#[derive(Debug, Clone)]
pub struct RequestContext {
    pub service_id: String,
    pub client_id: String,
    pub service_path: String,
    pub api_path: String,
    pub sla: String,
    pub args: HashMap<String, String>,
    pub service_filters: HashMap<String, Vec<FilterSetting>>,
    pub client_filters: HashMap<String, Vec<FilterSetting>>,
    pub request_id: Uuid,
}

impl RequestContext {
    pub fn new(req: &Request<Body>, auth: &AuthResponse) -> Self {
        let req_id = Self::extract_request_id(req);
        let (service_path, api_path) = Self::extract_path(req.uri().path());
        let mut context = RequestContext {
            service_id: auth.service_id.clone(),
            client_id: auth.client_id.clone(),
            service_path,
            api_path,
            sla: auth.sla.clone(),
            args: HashMap::new(),
            service_filters: HashMap::new(),
            client_filters: HashMap::new(),
            request_id: req_id,
        };
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

    fn extract_path(path: &str) -> (String, String) {
        let path = path.strip_prefix("/").unwrap();
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


pub fn middleware_chain(req: Request<Body>, context: RequestContext, mut mw_stack: Vec<MiddlewareHandle>)
        -> Pin<Box<dyn Future<Output=Response<Body>> + Send>> 
{
    let depth = mw_stack.len();
    if depth == 0 {
        return Box::pin(async{
            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Middleware misconfiguration"))
                .unwrap();
            resp
        });
    }

    let MiddlewareHandle {name, chan, pre, post} = mw_stack.pop().unwrap();
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

    let resp_service_filters = service_filters.clone();
    let resp_client_filters = client_filters.clone();

    let fut = async move {
        // request middleware pre-filter
        let MwPreResponse{context, request, response} = {
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
                rx.await.unwrap()
            } else {
                MwPreResponse { context, request: Some(req), response: None }
            }
        };

        // if pre-filter returns response, terminate chain and return
        if let Some(early_resp) = response {
            return early_resp;
        }

        // call inner middleware
        let context_copy = context.clone();
        let inner_resp: Response<Body> = middleware_chain(request.unwrap(), context, mw_stack).await;

        // call middleware post-filter
        let final_resp = {
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
                rx.await.unwrap()
            } else {
                MwPostResponse { context: context_copy, response: inner_resp }
            }
        };
        
        final_resp.response
    };

    Box::pin(fut)
}

