use tokio;
use tracing::{event, Level};
use clap::{App, Arg};
use hyper::{Server, Uri};
use hyper::server::conn::AddrStream;
use hyper::service::make_service_fn;
use serde_yaml;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};
use hyperapi::config::{GatewayConfig, config_poll};
use hyperapi::proxy::GatewayServer;
use std::sync::{Arc, Mutex};
use tracing_subscriber;


#[derive(Serialize, Deserialize, Debug)]
struct ServerConfigFile {
    pub apihub: GatewayConfig,
}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let matches = App::new("apihub.rs")
          .version("0.1")
          .author("Leric Zhang <leric.zhang@gmail.com>")
          .about("The way to API")
          .arg(Arg::with_name("config").required(true).takes_value(true)
               .short("c").long("config")
               .value_name("FILE")
               .help("Set config file path"))
          .arg(Arg::with_name("test")
               .short("t").long("test")
               .help("Validate config file"))
          .get_matches();
    let config_file = matches.value_of("config").unwrap();
    let is_testing = matches.is_present("test");

    if is_testing {
        // todo
        event!(Level::INFO, "Validating config file");
    } else {
        let content = tokio::fs::read_to_string(config_file).await.expect("Failed to read config file");
        let config_file = serde_yaml::from_str::<ServerConfigFile>(&content).expect("Failed to parse config file");
        let config = config_file.apihub;
        let addr = config.listen.parse().expect("Invalid listen address");
        let config_source = config.config_source.clone();
        // let cert_file = config.ssl_certificate.clone();
        // let cert_key_file = config.ssl_certificate_key.clone();

        let server = GatewayServer::new(config);
        let server = Arc::new(Mutex::new(server));

        if let Some(source) = config_source {
            let s = server.clone();
            tokio::spawn(async move {
                let source_uri = source.parse::<Uri>().unwrap();
                config_poll(source_uri, s).await
            });
        }

        event!(Level::INFO, "Starting http gateway edge server");
        let make_svc = make_service_fn(|socket: &AddrStream| {
            let remote_addr = socket.remote_addr();
            let handler = {
                let lock = server.lock().expect("GatewayServer status error");
                lock.make_service(remote_addr)
            };
            async move {
                Ok::<_, Infallible>(handler)
            }
        });
        let server = Server::bind(&addr)
                .serve(make_svc);
        server.await.expect("Server failed to start");
    }
}
