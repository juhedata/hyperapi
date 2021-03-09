mod protocol;
mod watch;
mod common;

pub use protocol::*;
pub use watch::{ConfigUpdate, ConfigSource};
pub use common::RequestMatcher;


#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_load_config_file() {
        let file_path = String::from("file:///opt/apihub/hyperapi/src/config/sample_config.yaml");
        let config_source = ConfigSource::new(file_path);
        while let Some(config) = config_source.next().await {
            assert!(true)
        }
    }

    #[tokio::test]
    async fn test_websocket_config() {
        let ws_url = String::from("ws://10.0.49.83:8008/ws/b89c67936b144631817410b599554988");
        let config_source = ConfigSource::new(ws_url);
        let mut count = 0;
        while let Some(config) = config_source.next().await {
            count = count + 1;
            assert!(true);
            if count > 3 {
                break
            }
        }
    }
}

