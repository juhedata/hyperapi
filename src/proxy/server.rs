use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::config::ConfigUpdate;
use crate::proxy::handler::RequestHandler;
use crate::config::{GatewayConfig, build_service_provider, ServiceProvider};


pub struct GatewayServer {
    pub config: GatewayConfig,
    services: Arc<ServiceProvider>,
}


impl GatewayServer {

    pub async fn new(config: GatewayConfig) -> GatewayServer {
        let services = build_service_provider(&config).await;
        GatewayServer {
            config: config,
            services: Arc::new(services),
        }
    }

    pub fn make_service(&self, conn: SocketAddr) -> RequestHandler {
        let services = self.services.clone();

        RequestHandler {
            address: conn,
            services: services,
            _req: PhantomData,
        }
    }

    pub async fn watch_config_update(&mut self) {
        if let Some(url) = self.config.config_provider {
            let updates = crate::config::build_config_source(url);
            while let Some(update) = updates.poll_next().await {
                match update {
                    ConfigUpdate::Client(c) => self.services.update_client_info(c).await,
                    ConfigUpdate::Service(s) => self.services.update_service_info(s).await,
                }
            }
        }
    }

}
