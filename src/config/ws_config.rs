use tokio::sync::mpsc;
use crate::config::ConfigUpdate;
use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};


pub async fn watch_config(ws_url: String, sender: mpsc::Sender<ConfigUpdate>) {
    println!("connecting to websocket");
    let result = connect_async(ws_url.clone()).await;
    if let Ok((mut ws, _)) = result {
        while let Some(res) = ws.next().await {
            if let Ok(msg) = res {
                match msg {
                    Message::Text(txt) => {
                        let update = serde_json::from_str::<ConfigUpdate>(&txt);
                        if let Ok(up) = update {
                            let _ = sender.send(up).await;
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

}