use tokio::sync::{mpsc, broadcast};
use std::net::SocketAddr;
use tracing::{event, Level};
use crate::middleware::{MiddlewareRequest, Middleware, AuthMiddleware, CorsMiddleware, HeaderMiddleware, RateLimitMiddleware, UpstreamMiddleware};
use crate::config::{ConfigSource, GatewayConfig};
use super::RequestHandler;

use crate::start_middleware_macro;

pub struct GatewayServer {
    pub stack: Vec<(String, mpsc::Sender<MiddlewareRequest>)>,

}

impl GatewayServer {

    pub fn new(config: GatewayConfig) -> Self {

        let mut stack = Vec::new();
        let (conf_tx, _conf_rx) = broadcast::channel(16);

        // start upstream middleware, last in stack run first
        start_middleware_macro!(UpstreamMiddleware, stack, conf_tx);

        // start header middleware
        start_middleware_macro!(HeaderMiddleware, stack, conf_tx);

        // start cors middleware
        start_middleware_macro!(CorsMiddleware, stack, conf_tx);

        // start ratelimit middleware
        start_middleware_macro!(RateLimitMiddleware, stack, conf_tx);

        // start auth middleware
        start_middleware_macro!(AuthMiddleware, stack, conf_tx);

        // poll config update
        let mut config_source = ConfigSource { config };
        tokio::spawn(async move {
            event!(Level::INFO, "Loading Service Config");
            config_source.watch(conf_tx).await
        });

        GatewayServer { stack }
    }


    pub fn make_service(&self, _addr: SocketAddr) -> RequestHandler {
        let stack = self.stack.clone();
        RequestHandler { stack }
    }

}
 
