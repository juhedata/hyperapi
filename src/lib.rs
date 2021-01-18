pub mod proxy;
pub mod config;
pub mod middleware;


#[macro_export]
macro_rules! start_middleware_macro {
    ($t:ty, $s:expr, $c:expr) => {
        let (tx, rx) = mpsc::channel(16);
        let conf_update = $c.subscribe();
        let name = <$t>::default().name();
        tokio::spawn(async move {
            event!(Level::INFO, "Starting UpstreamMiddleware");
            crate::middleware::start_middleware::<$t>(rx, conf_update).await
        });
        $s.push((name, tx));
    };
}



#[cfg(test)]
mod tests {
    #[test]
    fn exploration() {
        assert_eq!(2 + 2, 4);
    }
}


