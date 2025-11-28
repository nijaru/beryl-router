pub mod server;
pub mod resolver;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DnsConfig {
    pub enabled: bool,
    pub listen: Vec<SocketAddr>,
    pub upstream: Vec<SocketAddr>,
}

pub struct DnsServer {
    config: DnsConfig,
}

impl DnsServer {
    pub fn new(config: DnsConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        tracing::info!("Starting DNS Server...");
        // TODO: Implement using hickory-server
        Ok(())
    }
}
