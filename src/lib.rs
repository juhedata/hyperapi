pub mod proxy;
pub mod config;
pub mod middleware;
pub mod auth;


#[macro_export]
macro_rules! start_middleware_macro {
    ($t:ty, $s:expr, $c:expr) => {
        let (tx, rx) = mpsc::channel(16);
        let conf_update = $c.subscribe();
        tokio::spawn(async move {
            event!(Level::INFO, "Starting UpstreamMiddleware");
            crate::middleware::start_middleware::<$t>(rx, conf_update).await
        });
        $s.push(crate::middleware::MiddlewareHandle {
            name: <$t>::name(),
            pre: <$t>::pre(),
            post: <$t>::post(),
            require_setting: <$t>::require_setting(),
            chan: tx,
        });
    };
}



