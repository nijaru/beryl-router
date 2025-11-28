use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub enabled: bool,
    pub interface: String,
    pub pool: PoolConfig,
    pub options: OptionsConfig,
    #[serde(default)]
    pub static_leases: Vec<StaticLease>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PoolConfig {
    pub start: Ipv4Addr,
    pub end: Ipv4Addr,
    pub lease_time: String, // e.g. "12h"
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OptionsConfig {
    pub gateway: Option<Ipv4Addr>,
    #[serde(default)]
    pub dns: Vec<Ipv4Addr>,
    pub domain: Option<String>,
    #[serde(default)]
    pub ntp: Vec<Ipv4Addr>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StaticLease {
    pub mac: String,
    pub ip: Ipv4Addr,
    pub hostname: Option<String>,
}

pub struct Server {
    config: ServerConfig,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        if !self.config.enabled {
            tracing::info!("DHCP Server disabled");
            return Ok(());
        }
        
        tracing::info!("Starting DHCP Server on {}", self.config.interface);
        
        // TODO: Bind socket and listen loop
        
        Ok(())
    }
}