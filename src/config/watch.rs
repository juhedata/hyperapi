use futures::{SinkExt, Stream, StreamExt};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::{pin::Pin, time::Duration};
use std::task::{Context, Poll};
use crate::config::{ClientInfo, ServiceInfo};
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use pin_project::pin_project;
use rand::Rng;


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
        loop {  // retry on disconnection
            println!("connecting to websocket");
            let result = connect_async(ws_url.clone()).await;
            if let Ok((mut ws, _)) = result {
                while let Some(res) = ws.next().await {
                    if let Ok(msg) = res {
                        match msg {
                            Message::Text(txt) => {
                                let update = serde_json::from_str::<ConfigUpdate>(&txt);
                                if let Ok(up) = update {
                                    sender.send(up).await.unwrap();
                                } else {
                                    println!("bad config update message: {:?}", update);
                                }
                            },
                            Message::Ping(sn) => {
                                let _ = ws.send(Message::Pong(sn)).await;
                            },
                            Message::Close(_) => {
                                break;
                            },
                            _ => {},
                        }
                    }
                }
            }
            let wait_time = rand::thread_rng().gen_range(1, 61);
            println!("ws connection lost, sleep {}s to reconnect", &wait_time);
            tokio::time::sleep(Duration::from_secs(wait_time)).await;
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

