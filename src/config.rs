use clap::Parser;
use std::time::Duration;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(short, long, default_value = "Logbot-HPS", help = "Service name")]
    pub name: String,
    #[arg(short, long, default_value = "60", help = "HTTP requests timeout in seconds")]
    pub timeout: u64,
    #[arg(short, long, default_value = "0", help = "Overrides the MTU size in bytes")]
    pub mtu: usize,
}

impl Config {
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout)
    }

    pub fn effective_mtu(&self, established_mtu: usize) -> usize {
        if self.mtu > 0 && self.mtu < established_mtu {
            self.mtu
        } else {
            established_mtu - crate::constants::MTU_OVERHEAD
        }
    }
}