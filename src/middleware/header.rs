use hyper::{HeaderMap, header::{HeaderName, HeaderValue}};
use std::future::Future;
use std::pin::Pin;
use crate::middleware::{MwPostRequest, MwPreRequest, MwPreResponse, MwPostResponse, Middleware};
use crate::config::{ConfigUpdate, FilterSetting, HeaderSetting};


#[derive(Debug)]
pub struct HeaderMiddleware {}


impl Default for HeaderMiddleware {
    fn default() -> Self {
        HeaderMiddleware {}
    }
}


impl Middleware for HeaderMiddleware {

    fn name() -> String {
        "Header".into()
    }

    fn request(&mut self, task: MwPreRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPreRequest {context, mut request, service_filters, client_filters, result} = task;
        let mut headers = request.headers_mut();
        for sf in service_filters {
            if let FilterSetting::Header(filter) = sf {
                headers = apply_header_filter(headers, &filter, "request");
            }
        }
        for cf in client_filters {
            if let FilterSetting::Header(filter) = cf {
                headers = apply_header_filter(headers, &filter, "request");
            }
        }
        let resp = MwPreResponse {context: context, request: Some(request), response: None };
        result.send(resp).unwrap();
        Box::pin(async {})
    }

    fn response(&mut self, task: MwPostRequest) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let MwPostRequest {context, mut response, service_filters, client_filters, result} = task;
        let mut headers = response.headers_mut();
        for sf in service_filters {
            if let FilterSetting::Header(filter) = sf {
                headers = apply_header_filter(headers, &filter, "response");
            }
        }
        for cf in client_filters {
            if let FilterSetting::Header(filter) = cf {
                headers = apply_header_filter(headers, &filter, "response");
            }
        }
        let resp = MwPostResponse {context: context, response: response };
        result.send(resp).unwrap();
        Box::pin(async {})
    }

    fn config_update(&mut self, _update: ConfigUpdate) {}
    
}


fn apply_header_filter<'a>(header: &'a mut HeaderMap, filter: &HeaderSetting, operate_on: &str) -> &'a mut HeaderMap {
    if !filter.operate_on.eq(operate_on) {
        return header;
    }
    for k in filter.removal.iter() {
        let kn = HeaderName::from_lowercase(k.as_bytes()).unwrap();
        header.remove(kn);
    }
    for (k, v) in filter.injection.iter() {
        let kn = HeaderName::from_lowercase(k.as_bytes()).unwrap();
        header.insert(kn, HeaderValue::from_str(v).unwrap());
    }
    header
}