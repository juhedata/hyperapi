use tokio;
use clap::{App, Arg};
use hyper::{Server, server::conn::AddrStream};
use hyper::service::{make_service_fn};
use serde_yaml;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};

use hyperapi::config::GatewayConfig;
use hyperapi::proxy::GatewayServer;

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfigFile {
    pub apihub: GatewayConfig,
}


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
    let config_file = matches.value_of("config").unwrap();
    let is_testing = matches.is_present("test");

    if is_testing {
        // todo
        println!("Validating config file");
    } else {
        let content = tokio::fs::read_to_string(config_file).await.unwrap();
        let config_file = serde_yaml::from_str::<ServerConfigFile>(&content).unwrap();
        let config = config_file.apihub;
        let addr = config.listen.parse().unwrap();
        let server = GatewayServer::new(config);
        
        let make_svc = make_service_fn(|socket: &AddrStream| {
            let remote_addr = socket.remote_addr();
            let handler = server.make_service(remote_addr);
            async move {
                Ok::<_, Infallible>(handler)
            }
        });

        // start listening
        let server = Server::bind(&addr).serve(make_svc);

        server.await.unwrap();
    }
}
