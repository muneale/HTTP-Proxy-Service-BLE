pub mod app_state;
pub mod bluetooth;
pub mod config;
pub mod constants;
pub mod error;
pub mod http;
pub mod utils;

pub use app_state::AppState;
pub use config::Config;
pub use error::Result;
use tracing::info;
pub use std::sync::Arc;

pub async fn run(config: Config) -> Result<()> {
    // Initialize logger
    tracing_subscriber::fmt::init();

    info!(target: "hps_ble", "Starting HPS BLE server with config: {:?}", &config);

    let state = Arc::new(AppState::new());
    let session = bluetooth::setup_bluetooth().await?;
    let adapter = session.default_adapter().await?;

    let adv_handle = bluetooth::start_advertising(&adapter, &config).await?;
    let app_handle = bluetooth::serve_gatt_application(&adapter, &state, &config).await?;

    utils::handle_signals().await?;

    bluetooth::cleanup(adv_handle, app_handle).await;

    Ok(())
}