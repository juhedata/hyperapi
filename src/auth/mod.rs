mod service;

pub use service::{AuthService, AuthRequest, AuthResponse};




#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{broadcast, mpsc, oneshot};
    use crate::config::*;
    use hyper::{Body, Request};

    #[tokio::test]
    async fn test_auth_service() {
        let (conf_tx, conf_rx) = broadcast::channel(16);
        let (auth_tx, auth_rx) = mpsc::channel(16);
        let handler = tokio::spawn(async move {
            let auth_service = AuthService::new(conf_rx, auth_rx);
            auth_service.start().await
        });

        let update = ConfigUpdate::ServiceUpdate(ServiceInfo{ 
            service_id: String::from("leric/test"),
            path: String::from("/test"),
            protocol: String::from("http"),
            auth: AuthSetting::AppKey(AppKeyAuth {}),
            timeout: 3000,
            upstreams: vec![
                Upstream { target: String::from("http://api.leric.net/test/") }
            ],
            filters: vec![
                FilterSetting::ACL(ACLSetting { 
                    access_control: String::from("allow"), 
                    paths: vec![
                        PathMatcher { methods: String::from("GET"), path_regex: String::from(".*") },
                    ],
                 }),
                FilterSetting::Header(HeaderSetting { 
                    operate_on: String::from("request"),
                    methods: String::from("*"),
                    path_regex: String::from(".*"),
                    injection: vec![(String::from("X-APP-ID"), String::from("hyperapi"))],
                    removal: vec![String::from("Authorization")],
                }),
            ],
            sla: vec![ServiceLevel { 
                name: String::from("Default"), 
                filters: vec![
                    FilterSetting::RateLimit(RateLimitSetting { 
                        methods: String::from("*"), 
                        path_regex: String::from(".*"),
                        interval: 1,
                        limit: 100, 
                        burst: 200,
                     }),
                ],
            }],
        });
        conf_tx.send(update);

        let update = ConfigUpdate::ClientUpdate(ClientInfo{ 
            client_id: String::from("leric/app"),
            app_key: String::from("abcdefg"),
            ip_whitelist: vec![],
            services: vec![String::from("leric/test:Default")],
        });
        conf_tx.send(update);

        // send request 
        let (tx, rx) = oneshot::channel();
        let req = Request::get("http://api.juhe.cn/api/v1/test").body(Body::empty()).unwrap();
        let (head, body) = req.into_parts();
        let request = AuthRequest {
            head: head,
            result: tx,
        };
        // get response
        let result = rx.await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.service_id.eq("leric/test"));
        assert!(resp.client_id.eq("leric/app"));
        
        handler.abort();
    }
}