use tokio;
use clap::{App, Arg};
use async_std::fs;
use hyper::{Server, service::make_service_fn, server::conn::AddrStream};
use std::sync::Arc;
use anyhow::Error;
use apihub_gateway::proxy::GatewayServer;
use apihub_gateway::config::GatewayConfig;


#[tokio::main]
async fn main() {
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
    let config_file = matches.value_of("c").unwrap();
    let is_testing = matches.is_present("t");

    if is_testing {
        println!("Validating config file");
    } else {
        let content = fs::read_to_string(config_file).await.unwrap();
        let config = serde_yaml::from_str::<GatewayConfig>(&content).unwrap();
        let addr = config.listen.parse().unwrap();
        let gateway = Arc::new(GatewayServer::new(config).await);

        let gwconf = Arc::clone(gateway);
        tokio::spawn(async move {
            gwconf.watch_config_update().await
        });

        let make_svc = make_service_fn(|conn: &AddrStream| {
            let gw = Arc::clone(&gateway);
            let addr = conn.remote_addr().clone();
            async move {
                Ok::<_, Error>(gw.make_service(addr))
            }
        });

        // start listening
        let server = Server::bind(&addr).serve(make_svc);
        server.await.unwrap();
    }
}
