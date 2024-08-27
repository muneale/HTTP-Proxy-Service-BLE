pub mod advertisement;
pub mod application;
pub mod characteristics;

use crate::{AppState, Config, Result};
use bluer::{
    Adapter,
    adv::AdvertisementHandle,
    Session,
};
use std::sync::Arc;
use tracing::info;

pub async fn setup_bluetooth() -> Result<Session> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    info!(
        "Using Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );

    Ok(session)
}

pub async fn start_advertising(adapter: &Adapter, config: &Config) -> Result<AdvertisementHandle> {
    let handle = advertisement::create_advertisement(adapter, config).await?;
    info!("Started advertising");
    Ok(handle)
}

pub async fn serve_gatt_application(
    adapter: &Adapter,
    state: &Arc<AppState>,
    config: &Config,
) -> Result<bluer::gatt::local::ApplicationHandle> {
    let app = application::create_application(state, config);
    let handle = adapter.serve_gatt_application(app).await?;
    info!("GATT application is now being served");
    Ok(handle)
}

pub async fn cleanup(
    adv_handle: AdvertisementHandle,
    app_handle: bluer::gatt::local::ApplicationHandle,
) {
    info!("Cleaning up Bluetooth resources");
    drop(app_handle);
    drop(adv_handle);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}