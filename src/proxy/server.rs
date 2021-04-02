use tokio::sync::{mpsc, broadcast};
use tracing::{event, Level};
use crate::middleware::{MiddlewareHandle, Middleware, HeaderMiddleware, RateLimitMiddleware, 
    UpstreamMiddleware, LoggerMiddleware, ACLMiddleware};
use crate::config::{ConfigSource, ConfigUpdate};
use super::RequestHandler;
use crate::auth::{AuthService, AuthRequest};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use crate::start_middleware_macro;



pub struct GatewayServer {
    pub service_stack: Vec<MiddlewareHandle>,
    pub auth_channel: mpsc::Sender<AuthRequest>,
    pub config_channel: broadcast::Sender<ConfigUpdate>,
    pub status: Arc<Mutex<u8>>,
}


impl GatewayServer {

    pub fn new(mut config: ConfigSource) -> Self {

        let mut stack = Vec::new();
        let (conf_tx, conf_rx) = broadcast::channel(16);
        let config_channel = conf_tx.clone();

        // start upstream middleware, last in stack run first
        start_middleware_macro!(UpstreamMiddleware, stack, conf_tx);
        // start header middleware
        start_middleware_macro!(HeaderMiddleware, stack, conf_tx);
        // start ratelimit middleware
        start_middleware_macro!(RateLimitMiddleware, stack, conf_tx);
        // start acl middleware
        start_middleware_macro!(ACLMiddleware, stack, conf_tx);
        // start log middleware
        start_middleware_macro!(LoggerMiddleware, stack, conf_tx);

        let server_status = Arc::new(Mutex::new(0u8));
        let init_status = server_status.clone();
        tokio::spawn(async move {
            event!(Level::INFO, "Watch Config Update");
            while let Some(config_update) = config.next().await {
                event!(Level::INFO, "Receive Config Update: {:?}", config_update);
                if let ConfigUpdate::ConfigReady(_) = config_update {
                    let mut lock = init_status.lock().unwrap();
                    *lock = 1;
                }
                let _ = conf_tx.send(config_update);
            }
        });
        
        let (auth_tx, auth_rx) = mpsc::channel(16);
        tokio::spawn(async move {
            event!(Level::INFO, "Start auth worker");
            let mut auth_service = AuthService::new(conf_rx, auth_rx);
            auth_service.start().await
        });

        GatewayServer {
            service_stack: stack,
            auth_channel: auth_tx,
            status: server_status,
            config_channel,
        }
    }


    pub fn make_service(&self) -> RequestHandler {
        let lock = self.status.clone();
        let ready = {
            lock.lock().unwrap().clone()
        };
        let stack = self.service_stack.clone();
        let auth = self.auth_channel.clone();
        RequestHandler { stack, auth, ready }
    }

}
 
