use std::borrow::Borrow;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{rustls, TlsAcceptor};
use tracing;

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_keys(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    private_key(&mut BufReader::new(File::open(path)?)).map(|v| v.unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::TRACE)
        .with_current_span(false)
        .init();
    let s = generate_simple_self_signed(vec!["localhost".to_string()])?;
    s.cert.pem();
    std::fs::write("cert.pem", &s.cert.pem())?;
    std::fs::write("key.pem", &s.key_pair.serialize_pem())?;
    let addr = "0.0.0.0:8080"
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
    let certs = load_certs(PathBuf::from_str("cert.pem")?.borrow())?;
    let key = load_keys(PathBuf::from_str("key.pem")?.borrow())?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(&addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();

        let fut = async move {
            let stream = acceptor.accept(stream).await?;
            let (mut reader, mut writer) = split(stream);
            let mut buffer = vec![0; 2048];
            reader.read_buf(&mut buffer).await?;
            println!("Reqeust start ----------");
            println!("{}", String::from_utf8_lossy(&buffer));
            println!("Reqeust end ----------");
            writer
                .write_all(
                    &b"HTTP/1.0 200 ok\r\n\
                    Connection: close\r\n\
                    Content-length: 12\r\n\
                    \r\n\
                    Hello world!"[..],
                )
                .await?;
            writer.flush().await?;

            Ok(()) as io::Result<()>
        };

        tokio::spawn(async move {
            if let Err(err) = fut.await {
                eprintln!("{:?}", err);
            }
        });
    }
}
