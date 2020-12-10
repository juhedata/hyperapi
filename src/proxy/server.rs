use tokio::sync::{mpsc, broadcast};
use std::net::SocketAddr;
use crate::middleware::{AuthMiddleware, CorsMiddleware, HeaderMiddleware, MiddlewareRequest, RateLimitMiddleware, UpstreamMiddleware, start_middleware};
use crate::config::{ConfigSource, GatewayConfig};
use super::RequestHandler;


pub struct GatewayServer {
    pub stack: Vec<mpsc::Sender<MiddlewareRequest>>,

}


impl GatewayServer {

    pub fn new(config: GatewayConfig) -> Self {

        let mut stack = Vec::new();
        let (conf_tx, _conf_rx) = broadcast::channel(16);
        
        // start upstream middleware, last in stack run first
        let (tx, rx) = mpsc::channel(16);
        let conf_update = conf_tx.subscribe();
        tokio::spawn(async move {
            start_middleware::<UpstreamMiddleware>(rx, conf_update).await
        });
        stack.push(tx);

        // start header middleware
        let (tx, rx) = mpsc::channel(16);
        let conf_update = conf_tx.subscribe();
        tokio::spawn(async move {
            start_middleware::<HeaderMiddleware>(rx, conf_update).await
        });
        stack.push(tx);

        // start cors middleware
        let (tx, rx) = mpsc::channel(16);
        let conf_update = conf_tx.subscribe();
        tokio::spawn(async move {
            start_middleware::<CorsMiddleware>(rx, conf_update).await
        });
        stack.push(tx);

        // start ratelimit middleware
        let (tx, rx) = mpsc::channel(16);
        let conf_update = conf_tx.subscribe();
        tokio::spawn(async move {
            start_middleware::<RateLimitMiddleware>(rx, conf_update).await
        });
        stack.push(tx);

        // start auth middleware
        let (tx, rx) = mpsc::channel(16);
        let conf_update = conf_tx.subscribe();
        tokio::spawn(async move {
            start_middleware::<AuthMiddleware>(rx, conf_update).await
        });
        stack.push(tx);

        // poll config update
        let mut config_source = ConfigSource { config };
        tokio::spawn(async move {
            config_source.watch(conf_tx).await
        });

        GatewayServer { stack }
    }


    pub fn make_service(&self, _addr: SocketAddr) -> RequestHandler {
        let stack = self.stack.clone();
        RequestHandler { stack }
    }

}
 
