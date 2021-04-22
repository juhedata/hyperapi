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
    use crate::auth::service::JwtClaims;


    fn gen_jwt_token(user_id: &str, sign_key: &str) -> String {
        let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let claims = service::JwtClaims {
            sub: user_id.to_owned(),
            exp: (ts + Duration::from_secs(3600)).as_secs(),
            iat: None,
            iss: None,
        };
        let sk = base64::decode_config(sign_key, base64::URL_SAFE).unwrap();
        let priv_key = jwt::EncodingKey::from_ec_der(sk.as_slice());

        let header = jwt::Header::new(jwt::Algorithm::ES256);
        let token = jwt::encode(&header, &claims, &priv_key);
        println!("{:?}", token);
        token.unwrap()
    }

    fn verify_jwt_token(token: &str, verify_key: &str) -> Option<service::JwtClaims> {
        //let vk = base64::decode_config(verify_key, base64::URL_SAFE).unwrap();
        let pub_key = jwt::DecodingKey::from_ec_pem(verify_key.as_bytes()).unwrap();
        let validation = jwt::Validation::new(jwt::Algorithm::ES256);
        let claims = jwt::decode::<JwtClaims>(&token, &pub_key, &validation);
        if let Ok(claim) = claims {
            Some(claim.claims)
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_jwt() {
        //let pub_key = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE0BGBOXCIP7euFi-GDsNa-3ZqwYzyUvDujXtya49q5_2wE4diZfEqNBEoftro49fWdtRfTWZgv64vt0j26OOX5Q==";
        let pub_key_pem = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE0BGBOXCIP7euFi+GDsNa+3ZqwYzy
UvDujXtya49q5/2wE4diZfEqNBEoftro49fWdtRfTWZgv64vt0j26OOX5Q==
-----END PUBLIC KEY-----";
        let priv_key = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgVRAbZnTAadaqZQjNoVMJ5tIsnlRkAGYe4NKTa26JYvuhRANCAATQEYE5cIg_t64WL4YOw1r7dmrBjPJS8O6Ne3Jrj2rn_bATh2Jl8So0ESh-2ujj19Z21F9NZmC_ri-3SPbo45fl";
        let token = gen_jwt_token("leric/app", priv_key);
        println!("{:?}", token);

        let claim = verify_jwt_token(&token, pub_key_pem);
        println!("{:?}", claim);
        assert!(claim.is_some())
    }

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
            timeout: 10,
            load_balance: String::from("conn"),
            upstreams: vec![
                Upstream {
                    target: String::from("http://api.leric.net/test/"), 
                    weight: 100, 
                    version: "0.1".into(), 
                    id: "1".into(), 
                    max_conn: 100,
                    error_threshold: 100,
                    error_reset: 60,
                    retry_delay: 10,
                }
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
        let update = ConfigUpdate::ServiceUpdate(ServiceInfo {
            service_id: String::from("leric/test2"),
            path: String::from("/test1"),
            protocol: String::from("http"),
            auth: AuthSetting::JWT(JwtAuth {}),
            timeout: 10,
            load_balance: String::from("load"),
            upstreams: vec![
                Upstream {
                    id: "1".into(), 
                    max_conn: 100,
                    target: String::from("http://api.leric.net/test/"),
                    error_threshold: 100,
                    error_reset: 60,
                    retry_delay: 10,
                    weight: 100,
                    version: "0.1".into(),
                }
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
        let app_key = "8cb0a8d3c9bb0e56659b6e2e30dc18d5";
        let pub_key = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE0BGBOXCIP7euFi+GDsNa+3ZqwYzy
UvDujXtya49q5/2wE4diZfEqNBEoftro49fWdtRfTWZgv64vt0j26OOX5Q==
-----END PUBLIC KEY-----";
        let priv_key = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgVRAbZnTAadaqZQjNoVMJ5tIsnlRkAGYe4NKTa26JYvuhRANCAATQEYE5cIg_t64WL4YOw1r7dmrBjPJS8O6Ne3Jrj2rn_bATh2Jl8So0ESh-2ujj19Z21F9NZmC_ri-3SPbo45fl";
        let mut client_services = HashMap::new();
        client_services.insert(String::from("leric/test"), String::from("Default"));
        client_services.insert(String::from("leric/test2"), String::from("Default"));
        let update = ConfigUpdate::ClientUpdate(ClientInfo{ 
            client_id: String::from("leric/app"),
            app_key: String::from(app_key),
            pub_key: String::from(pub_key),
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

        // send request to /test1
        {
            let token = gen_jwt_token("leric/app", priv_key);
            let (tx, rx) = oneshot::channel();
            let req = Request::get("http://api.juhe.cn/test1/v1/test")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty()).unwrap();
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
            assert!(resp.success);
        }

        handler.abort();
    }
}