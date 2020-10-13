use crate::proxy::config::{ServiceProvider, ClientProvider, ClientInfo};
use crate::proxy::proxy::ProxyHandler;
use hyper::{Request, Body};


pub struct WSServiceProvider {

}

pub struct WSClientProvider {

}


impl WSServiceProvider {
    pub fn new(config: &str) -> WSServiceProvider {

    }
}

impl ServiceProvider for WSServiceProvider {
    fn get_service_handler(&self, service_id: &str, client_id: &str) -> Result<&ProxyHandler, String> {
        unimplemented!()
    }
}

impl WSClientProvider {
    pub fn new(config: &str) -> WSClientProvider {

    }
}

impl ClientProvider for WSClientProvider {
    fn authenticate(&self, req: &Request<Body>) -> Result<&ClientInfo, String> {
        unimplemented!()
    }
}
