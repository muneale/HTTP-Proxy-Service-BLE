mod app_state;
mod bluetooth;
mod constants;
mod error;
mod http_handler;

use app_state::AppState;
use bluetooth::create_application;
use clap::Parser;
use error::AppError;
use std::sync::Arc;
use tokio::time::sleep;
use tracing::{info, debug};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "Logbot-HPS", help = "Service name")]
    name: String,
    #[arg(short, long, default_value = "60", help = "HTTP requests timeout in seconds")]
    timeout: u64,
    #[arg(short, long, default_value = "0", help = "Overrides the MTU size in bytes. When set to 0, it uses the established MTU size between client and server. Ignored when the value is greater than established MTU size.")]
    mtu: usize,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!(target="main", "Service started with arguments: {:?}", args);

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    info!(
        target="main",
        "Advertising on Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );

    let le_advertisement = bluer::adv::Advertisement {
        service_uuids: vec![*constants::SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some(args.name.to_string()),
        ..Default::default()
    };
    let adv_handle = adapter.advertise(le_advertisement).await?;

    info!(
        target="main",
        "Serving GATT echo service on Bluetooth adapter {}",
        adapter.name()
    );

    let state = Arc::new(AppState::new());

    let app = create_application(&state, args.timeout, args.mtu);
    let app_handle = adapter.serve_gatt_application(app).await?;

    info!(target="main", "Service ready.");

    bluetooth::handle_signals().await?;

    info!(target="main", "Removing service and advertisement");
    drop(app_handle);
    drop(adv_handle);
    sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}