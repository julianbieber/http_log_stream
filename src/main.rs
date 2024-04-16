use std::borrow::Borrow;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use clap::Parser;
use http::StatusCode;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_rustls::{rustls, TlsAcceptor};

#[derive(Parser)]
struct Opts {
    #[arg(long, default_value = "localhost")]
    hostname: String,
    #[arg(long)]
    files: Vec<PathBuf>,
    #[arg(long)]
    status: Vec<u16>,
    #[arg(long, default_value_t = 8080)]
    port: u16,
}

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_keys(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    private_key(&mut BufReader::new(File::open(path)?)).map(|v| v.unwrap())
}

async fn process_stream(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    counter: Arc<Mutex<usize>>,
    responses: Arc<Mutex<Vec<(String, u16)>>>,
) -> Result<(), Box<dyn Error>> {
    let stream = acceptor.accept(stream).await?;
    let (mut reader, mut writer) = split(stream);
    let mut buffer = vec![0; 2048];
    reader.read_buf(&mut buffer).await?;
    println!("Reqeust start ----------");
    println!("{}", String::from_utf8_lossy(&buffer));
    println!("Reqeust end ----------");
    let c = counter.lock().await;
    let lc = *c;
    drop(c);

    let r = responses.lock().await;
    let (l_r, status) = if r.len() == 0 {
        ("Hello".to_string(), 200)
    } else {
        r.get(lc % r.len()).unwrap().clone()
    };
    drop(r);
    let mut c = counter.lock().await;
    *c += 1;
    drop(c);

    // let response_content = &opts.files[*c];

    writer.write_all(b"HTTP/1.0 ").await?;
    writer.write_all(format!("{status} ").as_bytes()).await?;
    writer
        .write_all(
            StatusCode::from_u16(status)
                .unwrap()
                .canonical_reason()
                .unwrap()
                .as_bytes(),
        )
        .await?;
    // 200
    writer
        .write_all(
            b"\r\n\
                    Connection: close\r\n\
                    Content-length: ",
        )
        .await?;
    writer
        .write_all(format!("{}", l_r.len()).as_bytes())
        .await?;
    writer.write_all(b"\r\n\r\n").await?;
    writer.write_all(l_r.as_bytes()).await?;
    writer.flush().await?;

    Ok(()) as Result<(), Box<dyn Error>>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::parse();
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::TRACE)
        .with_current_span(false)
        .init();
    let s = generate_simple_self_signed(vec![opts.hostname])?;
    s.cert.pem();
    std::fs::write("cert.pem", s.cert.pem())?;
    std::fs::write("key.pem", s.key_pair.serialize_pem())?;
    let port = opts.port;
    let addr = format!("0.0.0.0:{port}")
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
    let counter = Arc::new(Mutex::new(0));
    let r: Vec<(String, u16)> = opts
        .files
        .into_iter()
        .map(|p| String::from_utf8_lossy(&std::fs::read(p).unwrap()).to_string())
        .zip(opts.status.into_iter())
        .collect();
    let responses = Arc::new(Mutex::new(r));

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();

        let local_counter = counter.clone();
        let local_responses = responses.clone();

        tokio::spawn(async move {
            if let Err(err) = process_stream(acceptor, stream, local_counter, local_responses).await
            {
                eprintln!("{:?}", err);
            }
        });
    }
}
