use std::{str::FromStr, sync::Arc};

use hyper::{Client, Request};
use rustls::{Certificate, ClientConfig, KeyLogFile, RootCertStore};
use tracing::info;
use tracing_subscriber::{filter::targets::Targets, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install().unwrap();

    let filter_layer =
        Targets::from_str(std::env::var("RUST_LOG").as_deref().unwrap_or("info")).unwrap();
    let format_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .init();

    info!("Setting up TLS root certificate store");
    let mut root_store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs()? {
        root_store.add(&Certificate(cert.0))?;
    }

    let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    client_config.key_log = Arc::new(KeyLogFile::new());

    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(client_config)
        .https_only()
        .enable_http2()
        .build();

    let client = Client::builder()
        .http2_only(true)
        .build::<_, hyper::Body>(connector);

    let req = Request::get("https://example.org").body(hyper::Body::empty())?;
    info!("Performing HTTP/2 request...");
    let res = client.request(req).await?;
    info!("Response header: {:?}", res);

    info!("Reading response body...");
    let body = hyper::body::to_bytes(res.into_body()).await?;
    info!("Response body is {} bytes", body.len());

    Ok(())
}
