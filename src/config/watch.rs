use futures::Stream;
use tokio::sync::mpsc;
use std::{pin::Pin, time::Duration};
use std::task::{Context, Poll};
use crate::config::{file_config, ws_config, etcd_config, ConfigUpdate};
use pin_project::pin_project;
use rand::Rng;
use tracing::{event, Level};


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
                file_config::watch_config(source.replace("file:///", ""), tx).await;
            });
        } else if source.starts_with("ws://") || source.starts_with("wss://") {
            tokio::spawn(async move {
                loop {
                    ws_config::watch_config(source.clone(), tx.clone()).await;
                    let wait_time = rand::thread_rng().gen_range(10..100);
                    event!(Level::WARN, "ws connection lost, sleep {}s to reconnect", &wait_time);
                    tokio::time::sleep(Duration::from_secs(wait_time)).await;
                }
            });
        } else if source.starts_with("etcd://") {
            tokio::spawn(async move {
                loop {
                    etcd_config::watch_config(source.clone(), tx.clone()).await;
                    let wait_time = rand::thread_rng().gen_range(10..100);
                    event!(Level::WARN, "etcd connection lost, sleep {}s to reconnect", &wait_time);
                    tokio::time::sleep(Duration::from_secs(wait_time)).await;
                }
            });
        } else {
            // try read as config file
            tokio::spawn(async move {
                file_config::watch_config(source, tx).await;
            });
        }
        ConfigSource { reciever: rx }
    }
}


impl Stream for ConfigSource {
    type Item = ConfigUpdate;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.reciever.poll_recv(cx)
    }
}

