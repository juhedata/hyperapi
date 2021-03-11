use futures::Stream;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::config::{ClientInfo, ServiceInfo};
use tungstenite::Message;
use pin_project::pin_project;


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ServiceConfig {
    pub apps: Vec<ClientInfo>,
    pub services: Vec<ServiceInfo>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConfigUpdate {
    ServiceUpdate(ServiceInfo),
    ServiceRemove(String),
    ClientUpdate(ClientInfo),
    ClientRemove(String),
}

#[pin_project]
pub struct ConfigSource {
    #[pin]
    reciever: mpsc::Receiver<ConfigUpdate>
}


impl ConfigSource {
    pub fn new(source: String) -> Self {
        let (tx, rx) = mpsc::channel(16);
        if source.starts_with("file://") {
            tokio::spawn(async move {
                ConfigSource::load_config_file(source, tx).await
            });
        } else if source.starts_with("ws://") || source.starts_with("wss://") {
            tokio::spawn(async move {
                ConfigSource::watch_websocket(source, tx)
            });
        } else {
            panic!("Invalid config source")
        }
        ConfigSource { reciever: rx }
    }

    pub async fn load_config_file(config_file: String, sender: mpsc::Sender<ConfigUpdate>) {
        let content = tokio::fs::read_to_string(config_file).await.expect("Failed to read config file");
        let config = serde_yaml::from_str::<ServiceConfig>(&content).expect("Failed to parse config file");
        for s in config.services.iter() {
            sender.send(ConfigUpdate::ServiceUpdate(s.clone())).await.unwrap();
        }

        for c in config.apps.iter() {
            sender.send(ConfigUpdate::ClientUpdate(c.clone())).await.unwrap();
        }
    }

    pub async fn watch_websocket(ws_url: String, sender: mpsc::Sender<ConfigUpdate>) {
        let (mut ws, _) = tungstenite::client::connect(ws_url).expect("Failed to connect config source");
        ws.write_message(Message::text("SERVICES")).unwrap();
        ws.write_message(Message::text("CLIENTS")).unwrap();
        ws.write_message(Message::text("WATCH")).unwrap();
        while let Ok(msg) = ws.read_message() {
            match msg {
                Message::Text(txt) => {
                    let update = serde_json::from_str::<ConfigUpdate>(&txt);
                    if let Ok(up) = update {
                        sender.send(up).await.unwrap();
                    }
                }
                _ => {}
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

