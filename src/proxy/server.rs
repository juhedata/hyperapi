//use hyper::{Request, Body};
use tokio::sync::mpsc;
use std::net::SocketAddr;
use crate::config::{GatewayConfig, ServiceInfo, ClientInfo};
use crate::proxy::{ AuthRequest, ServiceRequest, RequestHandler, ServiceHandler };
use crate::auth::AuthHandler;


pub struct GatewayServer {
    pub auth_tx: mpsc::Sender<AuthRequest>,
    pub req_tx: mpsc::Sender<ServiceRequest>,
}


impl GatewayServer {

    pub fn new(config: GatewayConfig) -> Self {
        let (auth_tx, req_tx) = Self::start_worker(&config.apps, &config.services);
        GatewayServer { auth_tx, req_tx }
    }

    pub fn make_service(&self, _addr: SocketAddr) -> RequestHandler {
        let auth_tx = self.auth_tx.clone();
        let req_tx = self.req_tx.clone();
        RequestHandler::new(auth_tx, req_tx)
    }

    pub fn update_config(&mut self, apps_conf: &Vec<ClientInfo>, services_conf: &Vec<ServiceInfo>) {
        let (auth_tx, req_tx) = Self::start_worker(apps_conf, services_conf);

        self.auth_tx = auth_tx;
        self.req_tx = req_tx;
    }

    fn start_worker(apps_conf: &Vec<ClientInfo>, services_conf: &Vec<ServiceInfo>) -> (mpsc::Sender<AuthRequest>, mpsc::Sender<ServiceRequest>) {
        let (atx, arx) = mpsc::channel::<AuthRequest>(100);
        let mut ah = AuthHandler::new(apps_conf, services_conf);
        tokio::spawn(async move {
            ah.auth_worker(arx).await
        });

        let (req_tx, req_rx) = mpsc::channel::<ServiceRequest>(100);
        let mut sh = ServiceHandler::new(services_conf);
        tokio::spawn(async move {
            sh.service_chain(req_rx).await
        });

        (atx, req_tx)
    }
}
 
