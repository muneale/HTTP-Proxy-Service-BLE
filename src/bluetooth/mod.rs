pub mod characteristics;

use crate::app_state::AppState;
use bluer::gatt::local::{Application, Service};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

pub fn create_application(state: &Arc<AppState>, timeout: u64, mtu: usize) -> Application {
    Application {
        services: vec![Service {
            uuid: *crate::constants::SERVICE_UUID,
            primary: true,
            characteristics: vec![
                characteristics::create_http_headers_body_mtu_sizes_characteristic(state),
                characteristics::create_http_headers_body_chunk_idx_characteristic(state),
                characteristics::create_http_uri_characteristic(state),
                characteristics::create_http_headers_characteristic(state, mtu),
                characteristics::create_http_status_code_characteristic(state),
                characteristics::create_http_entity_body_characteristic(state, mtu),
                characteristics::create_https_security_characteristic(state),
                characteristics::create_http_control_point_characteristic(state, timeout, mtu),
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}

pub async fn handle_signals() -> Result<(), crate::error::AppError> {
    let mut signal_terminate = signal(SignalKind::terminate())?;
    let mut signal_interrupt = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = signal_terminate.recv() => info!(target="handle_signals", "Received SIGTERM"),
        _ = signal_interrupt.recv() => info!(target="handle_signals", "Received SIGINT"),
    };
    Ok(())
}