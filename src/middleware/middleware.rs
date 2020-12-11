use std::{collections::HashMap, pin::Pin};
use hyper::{Request, Response, Body, StatusCode};
use tokio::sync::{mpsc, broadcast};
use tokio::sync::oneshot;
use std::future::Future;
use tracing::{event, Level};
use crate::config::{ClientId, ConfigUpdate};


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
    
    fn work(&mut self, task: MiddlewareRequest) -> Pin<Box<dyn Future<Output=()> + Send>>;

    fn config_update(&mut self, update: ConfigUpdate);
    
}


#[derive(Debug, Clone)]
pub struct RequestContext {
    pub service_id: String,
    pub client: Option<ClientId>,
    pub args: HashMap<String, String>,
}

impl RequestContext {
    pub fn new(req: &Request<Body>) -> Self {
        let service_id = Self::extract_service_id(req);
        RequestContext {
            service_id,
            client: None,
            args: HashMap::new(),
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
}


pub async fn start_middleware<MW>(mut tasks: mpsc::Receiver<MiddlewareRequest>, mut updates: broadcast::Receiver<ConfigUpdate>) 
where MW: Middleware + Default
{
    let mut mw = MW::default();

    loop {
        tokio::select! {
            task = tasks.recv() => {
                match task {
                    Some(x) => {
                        mw.work(x).await;
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
    if mw_stack.len() == 0 {
        return Box::pin(async{
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Middleware misconfiguration"))
                .unwrap()
        });
    }
    event!(Level::DEBUG, "enter middleware chain");
    let mut mw = mw_stack.pop().unwrap();
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
            Err(resp) => resp,
        }
    };

    Box::pin(fut)
}

