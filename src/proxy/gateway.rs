use tokio::sync::mpsc;
use crate::proxy::handler::{RequestHandler, AuthQuery, ServiceQuery};
use crate::proxy::config::GatewayConfig;
use serde::export::PhantomData;
use std::net::SocketAddr;


#[derive(Debug)]
pub struct GatewayServer {
    pub env: String,
    auth_sender: mpsc::Sender<AuthQuery>,
    service_sender: mpsc::Sender<ServiceQuery>,
}


impl GatewayServer {

    pub fn new(config: &GatewayConfig) -> GatewayServer {

        let (atx, arx) = mpsc::channel(100);
        let (stx, srx) = mpsc::channel(100);

        tokio::spawn(async move {
            GatewayServer::auth_factory(arx)
        });

        tokio::spawn(async move {
            GatewayServer::service_factory(srx)
        });

        GatewayServer {
            env: config.env.clone(),
            auth_sender: atx,
            service_sender: stx,
        }
    }

    pub fn make_service(&self, conn: SocketAddr) -> RequestHandler {
        let auth = self.auth_sender.clone();
        let service = self.service_sender.clone();

        RequestHandler {
            address: conn,
            auth_factory: auth,
            service_factory: service,
            _req: PhantomData,
        }
    }

    pub async fn auth_factory(mut rx: mpsc::Receiver<AuthQuery>) {
        while let Some(_msg) = rx.recv().await {

        }
    }

    pub async fn service_factory(mut rx: mpsc::Receiver<ServiceQuery>) {
        while let Some(_msg) = rx.recv().await {

        }
    }

    pub fn test(&self) {
        println!("hello");
    }
}
