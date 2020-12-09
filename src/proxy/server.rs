//use hyper::{Request, Body};
use tokio::sync::mpsc;
use std::net::SocketAddr;
use crate::config::{GatewayConfig, ServiceInfo, ClientInfo};
use crate::proxy::{ RequestHandler };
use crate::middleware::{MiddlewareRequest, UpstreamMiddleware};


pub struct GatewayServer {
    pub stack: Vec<mpsc::Sender<MiddlewareRequest>>,

}


impl GatewayServer {

    pub fn new(config: GatewayConfig) -> Self {
        let mut stack = Vec::new();
        
        let upstream_middleware = UpstreamMiddleware::new(&config.services);
        stack.push(upstream_middleware.tx);

        

        GatewayServer { stack }
    }


    pub fn make_service(&self, _addr: SocketAddr) -> RequestHandler {
        let stack = self.stack.clone();
        RequestHandler { stack }
    }

}
 
