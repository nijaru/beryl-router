use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::UdpSocket;

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
        
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_broadcast(true)?;
        socket.set_reuse_address(true)?;
        
        #[cfg(target_os = "linux")]
        {
            socket.bind_device(Some(self.config.interface.as_bytes()))?;
        }
        
        socket.bind(&SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 67).into())?;
        socket.set_nonblocking(true)?;
        
        let socket = UdpSocket::from_std(socket.into())?;
        
        let mut buf = [0u8; 1500];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    tracing::debug!("Received {} bytes from {}", len, addr);
                    // TODO: Parse packet
                }
                Err(e) => {
                    tracing::error!("DHCP receive error: {}", e);
                }
            }
        }
    }
}