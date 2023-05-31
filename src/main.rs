use std::{net::ToSocketAddrs, str::FromStr, sync::Arc};

use bytes::BytesMut;
use color_eyre::eyre::eyre;
use nom::Offset;
use rustls::{Certificate, ClientConfig, KeyLogFile, RootCertStore};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;
use tracing_subscriber::{filter::targets::Targets, layer::SubscriberExt, util::SubscriberInitExt};

use crate::h2::{Frame, FrameType, SettingsFlags};

mod h2;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // this is just a trick to get rust-analyzer to complete the body of the
    // function better. there's still issues with auto-completion within
    // functions, see https://github.com/rust-lang/rust-analyzer/issues/13355
    real_main().await
}

async fn real_main() -> color_eyre::Result<()> {
    color_eyre::install().unwrap();

    let filter_layer =
        Targets::from_str(std::env::var("RUST_LOG").as_deref().unwrap_or("info")).unwrap();
    let format_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .init();

    info!("Setting up TLS");
    let mut root_store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs()? {
        root_store.add(&Certificate(cert.0))?;
    }

    let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    client_config.key_log = Arc::new(KeyLogFile::new());
    client_config.alpn_protocols = vec![b"h2".to_vec()];

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));

    info!("Performing DNS lookup");
    let addr = "example.org:443"
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| eyre!("Failed to resolve address for example.org:443"))?;

    info!("Establishing TCP connection...");
    let stream = TcpStream::connect(addr).await?;

    info!("Establishing TLS session...");
    let mut stream = connector.connect("example.org".try_into()?, stream).await?;

    info!("Establishing HTTP/2 connection...");

    info!("Writing preface");
    stream.write_all(h2::PREFACE).await?;

    let settings = Frame::new(FrameType::Settings(Default::default()), 0);
    info!("> {settings:?}");
    settings.write(&mut stream).await?;

    let mut buf: BytesMut = Default::default();
    loop {
        info!("Reading frame ({} bytes so far)", buf.len());
        if stream.read_buf(&mut buf).await? == 0 {
            info!("connection closed!");
            return Ok(());
        }

        let slice = &buf[..];
        let frame = match Frame::parse(slice) {
            Ok((rest, frame)) => {
                buf = buf.split_off(slice.offset(rest));
                frame
            }
            Err(e) => {
                if e.is_incomplete() {
                    // keep reading!
                    continue;
                }
                panic!("parse error: {e}");
            }
        };

        info!("< {frame:?}");
        if let FrameType::Settings(flags) = &frame.frame_type {
            if !flags.contains(SettingsFlags::Ack) {
                info!("Acknowledging server settings");
                let settings = Frame::new(FrameType::Settings(SettingsFlags::Ack.into()), 0);
                info!("> {settings:?}");
                settings.write(&mut stream).await?;
            }
        }
    }
}
