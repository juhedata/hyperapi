use hyper::{Request, Response, Body, Uri};
use hyper::client::connect::HttpConnector;
use hyper::client::Client;
use hyper_rustls::HttpsConnector;
use tower::Service;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::future::Future;
use tower::timeout::Timeout;
use tower::discover::ServiceList;
use std::time::Duration;
use tower_load::{PeakEwmaDiscover, NoInstrument};
use tower_balance::p2c::Balance;
use anyhow::anyhow;


pub struct ProxyService {
    pub handler: Balance<PeakEwmaDiscover<ServiceList<Vec<Timeout<ProxyHandler>>>, NoInstrument>, Request<Body>>,
}

impl ProxyService {
    pub fn new(upstreams: Vec<String>, timeout: Duration) -> Self {
        let list: Vec<Timeout<ProxyHandler>> = upstreams.iter().map(|u| {
            Timeout::new(ProxyHandler::new(u.clone()), timeout)
        }).collect();

        let discover = ServiceList::new(list);
        let load = PeakEwmaDiscover::new(discover, Duration::from_millis(50), Duration::from_secs(1), NoInstrument);
        let balance = Balance::from_entropy(load);
        
        ProxyService { handler: balance }
    }
}


impl Service<Request<Body>> for ProxyService {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.handler.poll_ready(c).map_err(|_e| anyhow!("poll error"))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let f = self.handler.call(req);
        Box::pin(async move {
            if let Ok(output) = f.await {
                Ok(output)
            } else {
                Err(anyhow!("upstream error"))
            }
        })
    }
}


#[derive(Debug, Clone)]
pub struct ProxyHandler {
    upstream: String,
    client: Client<HttpsConnector<HttpConnector>, Body>,
    //client: Client<HttpConnector, Body>,
}



impl ProxyHandler {

    pub fn new(upstream: String) -> Self {
        let tls = HttpsConnector::new();
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .build::<_, Body>(tls);
        // let client = Client::new();
        ProxyHandler { client, upstream }
    }

    fn alter_request(req: Request<Body>, endpoint: &str) -> Request<Body> {
        let (mut parts, body) = req.into_parts();
        
        let path_and_query = parts.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
        let path = path_and_query.strip_prefix("/").unwrap_or("/");
        let path_left = if let Some(offset) = path.find("/") {
            let (_service_id, path_left) = path.split_at(offset);
            path_left
        } else {
            ""
        };
        let mut new_uri = String::from(endpoint.trim_end_matches('/'));
        new_uri.push_str(path_left);

        parts.uri = new_uri.parse::<Uri>().unwrap();
        Request::from_parts(parts, body)
    }

}

impl Service<Request<Body>> for ProxyHandler
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _c: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let req = ProxyHandler::alter_request(req, &self.upstream);
        //println!("{:?}", req.uri());
        let f = self.client.request(req);
        Box::pin(async move {
            f.await.map_err(Into::into)
        })
    }
}

