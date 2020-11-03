use std::pin::Pin;
use std::task::{Poll, Context};
use tokio::net::{TcpListener, TcpStream};
use futures_util::{
    future::TryFutureExt,
    stream::{Stream, StreamExt, TryStreamExt},
};
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;
use rustls::internal::pemfile;


pub struct HyperAcceptor<'a> {
    acceptor: Pin<Box<dyn Stream<Item = Result<TlsStream<TcpStream>, std::io::Error>> + 'a>>,
}

impl<'a> HyperAcceptor<'a> {
    pub async fn wrap(tcp: &'a mut TcpListener, cert_file: String, key_file: String, ) -> std::io::Result<HyperAcceptor<'a>> {

        // Build TLS configuration.
        let tls_cfg = {
            // Load public certificate.
            let certs = load_certs(&cert_file)?;
            // Load private key.
            let key = load_private_key(&key_file)?;
            // Do not use client certificate authentication.
            let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());
            // Select a certificate to use.
            cfg.set_single_cert(certs, key)
                .map_err(|e| error(format!("{}", e)))?;
            // Configure ALPN to accept HTTP/2, HTTP/1.1 in that order.
            cfg.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);
            std::sync::Arc::new(cfg)
        };

        // Create a TCP listener via tokio.
        let tls_acceptor = TlsAcceptor::from(tls_cfg);
        // Prepare a long-running future stream to accept and serve cients.
        let incoming_tls_stream = tcp
            .incoming()
            .map_err(|e| error(format!("Incoming failed: {:?}", e)))
            .and_then(move |s| {
                tls_acceptor.accept(s).map_err(|e| {
                    println!("[!] Voluntary server halt due to client-connection error...");
                    // Errors could be handled here, instead of server aborting.
                    // Ok(None)
                    error(format!("TLS Error: {:?}", e))
                })
            })
            .boxed();
        Ok(HyperAcceptor {
            acceptor: incoming_tls_stream,
        })
    }

}

impl hyper::server::accept::Accept for HyperAcceptor<'_> {
    type Conn = TlsStream<TcpStream>;
    type Error = std::io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        Pin::new(&mut self.acceptor).poll_next(cx)
    }
}

// Load public certificate from file.
fn load_certs(filename: &String) -> std::io::Result<Vec<rustls::Certificate>> {
    // Open certificate file.
    let certfile = std::fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = std::io::BufReader::new(certfile);

    // Load and return certificate.
    pemfile::certs(&mut reader).map_err(|_| error("failed to load certificate".into()))
}

// Load private key from file.
fn load_private_key(filename: &String) -> std::io::Result<rustls::PrivateKey> {
    // Open keyfile.
    let keyfile = std::fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = std::io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = pemfile::rsa_private_keys(&mut reader)
        .map_err(|_| error("failed to load private key".into()))?;
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }
    Ok(keys[0].clone())
}

fn error(err: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

