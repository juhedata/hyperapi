use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::config::ConfigUpdate;


pub fn build_config_source(url: String) -> impl Stream<Item=ConfigUpdate> {
    WSConfigSource {
        source: url
    }
}


pub struct WSConfigSource {
    pub source: String,
}


impl Stream for WSConfigSource {
    type Item = ConfigUpdate;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>,) -> Poll<Option<Self::Item>>{
        let update = ClientInfo {};
        let item = ConfigUpdate::Client(update);
        Ready(item)
    }
}
