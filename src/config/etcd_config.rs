use tokio::sync::mpsc;
use crate::config::{ConfigUpdate, ServiceInfo, ClientInfo};
use etcd_client::{Client, EventType, GetOptions, WatchOptions};


// e.g. etcd://<env-ns>.<env-name>:<access-token>@<etcd_endpoint>/juapi/<env-ns>/<env-name>
pub async fn watch_config(source: String, sender: mpsc::Sender<ConfigUpdate>) {
    let url = url::Url::parse(&source).unwrap();
    let endpoints = vec![url.host_str().unwrap()];
    let conf_path = url.path();
    let mut client = Client::connect(endpoints, None).await.unwrap();

    let get_option = GetOptions::new().with_prefix();
    if let Ok(resp) = client.get(conf_path, Some(get_option)).await {
        for kv in resp.kvs() {
            let key = kv.key_str().unwrap();
            let val = kv.value_str().unwrap();
            let update = extract_event(key, val, false);
            if let Some(u) = update {
                let _ = sender.send(u).await;
            }
        }
        let _ = sender.send(ConfigUpdate::ConfigReady(true)).await;

        // watch further config changes
        let revision = resp.header().unwrap().revision();
        let watch_option = WatchOptions::new().with_prefix().with_start_revision(revision);
        let (_watcher, mut stream ) = client.watch(conf_path, Some(watch_option)).await.unwrap();
        while let Some(resp) = stream.message().await.unwrap() {
            if resp.canceled() {
                println!("watch canceled!");
                break;
            }
            for event in resp.events() {
                let update = match event.event_type() {
                    EventType::Put => {
                        if let Some(kv) = event.kv() {
                            extract_event(kv.key_str().unwrap(), kv.value_str().unwrap(), false)
                        } else {
                            None
                        }
                    },
                    EventType::Delete => {
                        if let Some(kv) = event.kv() {
                            extract_event(kv.key_str().unwrap(), kv.value_str().unwrap(), true)
                        } else {
                            None
                        }
                    }
                };
                if let Some(u) = update {
                    let _ = sender.send(u).await;
                }
            }
        }
    } else {
        println!("Fail to connect etcd");
    }
}


fn extract_event(key: &str, val: &str, is_delete: bool) -> Option<ConfigUpdate> {
    // key schema:  /juapi/<env-ns>.<env-name>/<services|clients>/<entity-ns>.<entity-name>
    let key_segments: Vec<&str> = key.split('/').collect();
    if key_segments.len() == 5 {
        let _env = key_segments.get(2).unwrap().clone();
        let entity_type = key_segments.get(3).unwrap().clone();
        let entity = key_segments.get(4).unwrap().clone();
        if is_delete {
            if entity_type.eq("services") {
                return Some(ConfigUpdate::ServiceRemove(String::from(entity)));
            } else if entity_type.eq("clients") {
                return Some(ConfigUpdate::ClientRemove(String::from(entity)));
            } else {
                return None;
            }
        } else {
            if entity_type.eq("services") {
                let data = serde_json::from_str::<ServiceInfo>(val);
                if let Ok(conf) = data {
                    return Some(ConfigUpdate::ServiceUpdate(conf));
                }
            } else if entity_type.eq("clients") {
                let data = serde_json::from_str::<ClientInfo>(val);
                if let Ok(conf) = data {
                    return Some(ConfigUpdate::ClientUpdate(conf));
                }
            } else {
                return None;
            }
        }
    }
    None
}
