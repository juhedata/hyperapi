use futures::{Stream, StreamExt};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::config::{ClientInfo, ServiceInfo};
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use pin_project::pin_project;


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServiceConfig {
    pub clients: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag="type", content="data")]
pub enum ConfigUpdate {
    ServiceUpdate(ServiceInfo),
    ServiceRemove(String),
    ClientUpdate(ClientInfo),
    ClientRemove(String),
    ConfigReady(bool),
}


#[pin_project]
pub struct ConfigSource {
    #[pin]
    reciever: mpsc::Receiver<ConfigUpdate>
}


impl ConfigSource {
    pub fn new(source: String) -> Self {
        let (tx, rx) = mpsc::channel(16);
        if source.starts_with("file:///") {
            tokio::spawn(async move {
                ConfigSource::load_config_file(source.replace("file:///", ""), tx).await;
            });
        } else if source.starts_with("ws://") || source.starts_with("wss://") {
            tokio::spawn(async move {
                ConfigSource::watch_websocket(source, tx).await;
            });
        } else {
            // try read as config file
            tokio::spawn(async move {
                ConfigSource::load_config_file(source, tx).await;
            });
        }
        ConfigSource { reciever: rx }
    }

    pub async fn load_config_file(config_file: String, sender: mpsc::Sender<ConfigUpdate>) {
        let content = tokio::fs::read_to_string(config_file).await.expect("Failed to read config file");
        let config = serde_yaml::from_str::<ServiceConfig>(&content).expect("Failed to parse config file");
        for s in config.services.iter() {
            sender.send(ConfigUpdate::ServiceUpdate(s.clone())).await.unwrap();
        }

        for c in config.clients.iter() {
            sender.send(ConfigUpdate::ClientUpdate(c.clone())).await.unwrap();
        }
        
        sender.send(ConfigUpdate::ConfigReady(true)).await.unwrap();
    }

    pub async fn watch_websocket(ws_url: String, sender: mpsc::Sender<ConfigUpdate>) {
        let (mut ws, _) = connect_async(ws_url).await.expect("Failed to connect config source");
        while let Some(res) = ws.next().await {
            if let Ok(msg) = res {
                match msg {
                    Message::Text(txt) => {
                        let update = serde_json::from_str::<ConfigUpdate>(&txt);
                        if let Ok(up) = update {
                            sender.send(up).await.unwrap();
                        }
                    },
                    _ => {},
                }
            }
        }
    }
}


impl Stream for ConfigSource {
    type Item = ConfigUpdate;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.reciever.poll_recv(cx)
    }
}

