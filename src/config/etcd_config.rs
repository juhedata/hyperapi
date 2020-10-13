use crate::proxy::config::{ServiceProvider, ClientProvider, ClientInfo};
use crate::proxy::proxy::ProxyHandler;
use hyper::{Request, Body};


pub struct EtcdServiceProvider {

}

pub struct EtcdClientProvider {

}


impl EtcdServiceProvider {
    pub fn new(config: &str) -> EtcdServiceProvider {

    }
}

impl ServiceProvider for EtcdServiceProvider {
    fn get_service_handler(&self, service_id: &str, client_id: &str) -> Result<&ProxyHandler, String> {
        unimplemented!()
    }
}

impl EtcdClientProvider {
    pub fn new(config: &str) -> EtcdClientProvider {

    }
}

impl ClientProvider for EtcdClientProvider {
    fn authenticate(&self, req: &Request<Body>) -> Result<&ClientInfo, String> {
        unimplemented!()
    }
}
