use std::{collections::HashMap, pin::Pin};
use hyper::{Request, Response, Body, StatusCode};
use tokio::sync::{mpsc, broadcast};
use tokio::sync::oneshot;
use std::future::Future;
use tracing::{span, event, Level, Instrument};
use crate::config::{ClientId, ConfigUpdate};
use uuid::Uuid;


#[derive(Debug)]
pub struct MwPreRequest {
    pub context: RequestContext,
    pub request: Request<Body>,
    pub result: oneshot::Sender<Result<(Request<Body>, RequestContext), Response<Body>>>,
}

#[derive(Debug)]
pub struct MwPostRequest {
    pub context: RequestContext,
    pub response: Response<Body>,
    pub result: oneshot::Sender<Response<Body>>,
}

#[derive(Debug)]
pub enum MiddlewareRequest {
    Request(MwPreRequest),
    Response(MwPostRequest),
}


pub trait Middleware {
    fn name(&self) -> String;

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    fn config_update(&mut self, update: ConfigUpdate);
}


#[derive(Debug, Clone)]
pub struct RequestContext {
    pub service_id: String,
    pub client: Option<ClientId>,
    pub args: HashMap<String, String>,
    pub request_id: Uuid,
}

impl RequestContext {
    pub fn new(req: &Request<Body>) -> Self {
        let service_id = Self::extract_service_id(req);
        let req_id = Self::extract_request_id(req);
        RequestContext {
            service_id,
            client: None,
            args: HashMap::new(),
            request_id: req_id,
        }
    }

    fn extract_service_id(req: &Request<Body>) -> String {
        let path = req.uri().path().strip_prefix("/").unwrap();
        let (service_id, _path) = match path.find("/") {
            Some(pos) => {
                path.split_at(pos)
            },
            None => {
                (path, "")
            }
        };
        String::from(service_id)
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
                        let app_id = ctx.client.map(|x| x.app_id).unwrap_or("".into());
                        let span = span!(Level::INFO, "pre_filter",
                                        service=ctx.service_id.as_str(),
                                        trace_id=ctx.request_id.to_string().as_str(),
                                        app_id=app_id.as_str(),
                                        middleware=mw.name().as_str());
                        mw.request(x).instrument(span).await;
                    },
                    Some(MiddlewareRequest::Response(x)) => {
                        let ctx = x.context.clone();
                        let app_id = ctx.client.map(|x| x.app_id).unwrap_or("".into());
                        let span = span!(Level::INFO, "post_filter",
                                        service=ctx.service_id.as_str(),
                                        trace_id=ctx.request_id.to_string().as_str(),
                                        app_id=app_id.as_str(),
                                        middleware=mw.name().as_str());
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


pub fn middleware_chain(req: Request<Body>, context: RequestContext, mut mw_stack: Vec<mpsc::Sender<MiddlewareRequest>>)
        -> Pin<Box<dyn Future<Output=Response<Body>> + Send>> 
{
    let depth = mw_stack.len();
    if depth == 0 {
        return Box::pin(async{
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Middleware misconfiguration"))
                .unwrap()
        });
    }
    let mw = mw_stack.pop().unwrap();
    let (tx1, rx1) = oneshot::channel();
    let mw_req = MwPreRequest {
        context,
        request: req,
        result: tx1,
    };

    let fut = async move {
        mw.send(MiddlewareRequest::Request(mw_req)).await.unwrap();
        let result = rx1.await.unwrap();
        match result {
            Ok((req, ctx)) => {
                // execute inner middleware
                let ctx_clone = ctx.clone();
                let resp = middleware_chain(req, ctx_clone, mw_stack).await;
                
                // process response
                let (tx2, rx2) = oneshot::channel();
                let mw_resp = MwPostRequest {
                    context: ctx,
                    response: resp,
                    result: tx2,
                };
                mw.send(MiddlewareRequest::Response(mw_resp)).await.unwrap();
                let resp = rx2.await.unwrap();
                resp
            }, 
            Err(resp) => {
                event!(Level::DEBUG, "Got Err<Response>");
                resp
            },
        }
    };

    Box::pin(fut)
}

