mod service;

pub use service::{AuthService, AuthRequest, AuthResponse};



#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use tokio::sync::{broadcast, mpsc, oneshot};
    use crate::config::*;
    use hyper::{Body, Request};
    use std::time::{Duration, SystemTime};
    use jsonwebtoken as jwt;


    // fn gen_jwt_token(user_id: &str, sign_key: &str) -> String {
    //     let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    //     let claims = service::JwtClaims {
    //         sub: user_id.to_owned(),
    //         exp: (ts + Duration::from_secs(3600)).as_secs(),
    //         iat: None,
    //         iss: None,
    //     };
    //     let priv_key = jwt::EncodingKey::from_ec_pem(sign_key.as_bytes()).unwrap();
    //
    //     let header = jwt::Header::new(jwt::Algorithm::ES256);
    //     let token = jwt::encode(&header, &claims, &priv_key);
    //     println!("{:?}", token);
    //     token.unwrap()
    // }


    #[tokio::test]
    async fn test_auth_service() {
        let (conf_tx, conf_rx) = broadcast::channel(16);
        let (auth_tx, auth_rx) = mpsc::channel(16);
        let handler = tokio::spawn(async move {
            let mut auth_service = AuthService::new(conf_rx, auth_rx);
            auth_service.start().await
        });

        // add a service with AppKeyAuth
        let update = ConfigUpdate::ServiceUpdate(ServiceInfo{ 
            service_id: String::from("leric/test"),
            path: String::from("/test"),
            protocol: String::from("http"),
            auth: AuthSetting::AppKey(AppKeyAuth {}),
            timeout: 3000,
            upstreams: vec![
                Upstream { target: String::from("http://api.leric.net/test/"), timeout: 3, id: "1".into() }
            ],
            filters: vec![
                FilterSetting::Header(HeaderSetting { 
                    operate_on: String::from("request"),
                    injection: vec![(String::from("X-APP-ID"), String::from("hyperapi"))],
                    removal: vec![String::from("Authorization")],
                }),
            ],
            sla: vec![ServiceLevel { 
                name: String::from("Default"), 
                filters: vec![
                    FilterSetting::RateLimit(RateLimitSetting { 
                        interval: 1,
                        limit: 100, 
                        burst: 200,
                     }),
                ],
            }],
        });
        conf_tx.send(update).unwrap();

        // add a service with JwtAuth
        let update = ConfigUpdate::ServiceUpdate(ServiceInfo{
            service_id: String::from("leric/test2"),
            path: String::from("/test2"),
            protocol: String::from("http"),
            auth: AuthSetting::JWT(JwtAuth {}),
            timeout: 3000,
            upstreams: vec![
                Upstream { target: String::from("http://api.leric.net/test/"), timeout: 3, id: "1".into() }
            ],
            filters: vec![
                FilterSetting::ACL(ACLSetting {
                    access_control: String::from("allow"),
                    paths: vec![
                        PathMatcher { methods: String::from("GET"), path_regex: String::from(".*") },
                    ],
                }),
            ],
            sla: vec![ServiceLevel {
                name: String::from("Default"),
                filters: vec![
                    FilterSetting::RateLimit(RateLimitSetting {
                        interval: 1,
                        limit: 100,
                        burst: 200,
                    }),
                ],
            }],
        });
        conf_tx.send(update).unwrap();

        // add client
        let app_key = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEiODjuXXNBXqRrpZYV0bYHM9Es2rjDFS7JyJmGVush5CXk0LaoS5OCXrw_zHIh6wGcvKQP8LCwHq_vnN1FnUhnA==";
        let app_secret = "MHcCAQEEINlRoHzdE_xarFy3kJI9DDdm4934ahistSnv00YnjXW2oAoGCCqGSM49AwEHoUQDQgAEiODjuXXNBXqRrpZYV0bYHM9Es2rjDFS7JyJmGVush5CXk0LaoS5OCXrw_zHIh6wGcvKQP8LCwHq_vnN1FnUhnA==";
        let mut client_services = HashMap::new();
        client_services.insert(String::from("leric/test"), String::from("Default"));
        client_services.insert(String::from("leric/test2"), String::from("Default"));
        let update = ConfigUpdate::ClientUpdate(ClientInfo{ 
            client_id: String::from("leric/app"),
            app_key: String::from(app_key),
            ip_whitelist: vec![],
            services: client_services,
        });
        conf_tx.send(update).unwrap();

        // yield for ConfigService to process
        tokio::time::sleep(Duration::from_secs(1)).await;

        // send a request to /test
        {
            let (tx, rx) = oneshot::channel();
            let req = Request::get(format!("http://api.juhe.cn/test/v1/test?_app_key={}", app_key)).body(Body::empty()).unwrap();
            let (head, _body) = req.into_parts();
            let request = AuthRequest {
                head: head,
                result: tx,
            };
            let _ = auth_tx.send(request).await;
            // get response
            let result = rx.await;
            assert!(result.is_ok());
            let (_parts, resp) = result.unwrap();
            println!("{:?}", resp);
            assert!(resp.service_id.eq("leric/test"));
            assert!(resp.client_id.eq("leric/app"));
        }

        // send a invalid request to /test1
        {
            let (tx, rx) = oneshot::channel();
            let req = Request::get(format!("http://api.juhe.cn/test1/v1/test?_app_key={}", app_key)).body(Body::empty()).unwrap();
            let (head, _body) = req.into_parts();
            let request = AuthRequest {
                head: head,
                result: tx,
            };
            let _ = auth_tx.send(request).await;
            // get response
            let result = rx.await;
            assert!(result.is_ok());
            let (_parts, resp) = result.unwrap();
            println!("{:?}", resp);
            assert!(!resp.success);
        }

        // // send request to /test1
        // {
        //     let token = gen_jwt_token("leric/app", app_secret);
        //     let (tx, rx) = oneshot::channel();
        //     let req = Request::get("http://api.juhe.cn/test1/v1/test")
        //         .header("Authorization", format!("Bearer {}", token))
        //         .body(Body::empty()).unwrap();
        //     let (head, _body) = req.into_parts();
        //     let request = AuthRequest {
        //         head: head,
        //         result: tx,
        //     };
        //     let _ = auth_tx.send(request).await;
        //     // get response
        //     let result = rx.await;
        //     assert!(result.is_ok());
        //     let (_parts, resp) = result.unwrap();
        //     println!("{:?}", resp);
        //     assert!(resp.service_id.eq("leric/test1"));
        //     assert!(resp.client_id.eq("leric/app"));
        // }

        handler.abort();
    }
}