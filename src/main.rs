use clap::Parser;
use hps_ble::{run, Config, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    run(config).await
}