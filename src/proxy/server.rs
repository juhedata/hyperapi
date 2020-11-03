//use hyper::{Request, Body};
use tokio::sync::mpsc;
use std::net::SocketAddr;
use crate::config::GatewayConfig;
use crate::proxy::{ AuthRequest, ServiceRequest, RequestHandler, ServiceHandler, AuthHandler };


pub struct GatewayServer {
    pub config: GatewayConfig,
    pub auth_tx: mpsc::Sender<AuthRequest>,
    pub req_tx: mpsc::Sender<ServiceRequest>,
}


impl GatewayServer {

    pub fn new(config: GatewayConfig) -> Self {
        let (atx, arx) = mpsc::channel::<AuthRequest>(100);
        let mut ah = AuthHandler::new(&config);
        tokio::spawn(async move {
            ah.auth_worker(arx).await
        });

        let (req_tx, req_rx) = mpsc::channel::<ServiceRequest>(100);
        let mut sh = ServiceHandler::new(config.services.clone());
        tokio::spawn(async move {
            sh.service_chain(req_rx).await
        });

        GatewayServer { config, auth_tx: atx, req_tx }
    }

    pub fn make_service(&self, _addr: SocketAddr) -> RequestHandler {
        let auth_tx = self.auth_tx.clone();
        let req_tx = self.req_tx.clone();
        RequestHandler::new(auth_tx, req_tx)
    }

}

