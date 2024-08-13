use crate::Result;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

pub async fn handle_signals() -> Result<()> {
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => info!("Received SIGTERM"),
        _ = sigint.recv() => info!("Received SIGINT"),
    }

    Ok(())
}