use crate::DnsConfig;
use crate::resolver::Forwarder;
use anyhow::Result;
use beryl_dhcp::database::LeaseDatabase;
use hickory_server::ServerFuture;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::RwLock;

pub struct DnsServer {
    config: DnsConfig,
    db: Arc<RwLock<LeaseDatabase>>,
    local_domain: Option<String>,
}

impl DnsServer {
    pub fn new(
        config: DnsConfig,
        db: Arc<RwLock<LeaseDatabase>>,
        local_domain: Option<String>,
    ) -> Self {
        Self {
            config,
            db,
            local_domain,
        }
    }

    pub async fn run(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let forwarder = Forwarder::new(
            self.config.upstream.clone(),
            self.db.clone(),
            self.local_domain.clone(),
        )?;

        let mut server = ServerFuture::new(forwarder);

        for addr in &self.config.listen {
            match UdpSocket::bind(addr).await {
                Ok(udp_socket) => {
                    server.register_socket(udp_socket);
                }
                Err(e) => {
                    tracing::error!("Failed to bind UDP {}: {}", addr, e);
                    continue;
                }
            }

            match TcpListener::bind(addr).await {
                Ok(tcp_listener) => {
                    server.register_listener(tcp_listener, Duration::from_secs(5));
                }
                Err(e) => {
                    tracing::error!("Failed to bind TCP {}: {}", addr, e);
                    continue;
                }
            }
        }

        tracing::info!(
            "DNS Server listening on {:?} with upstreams {:?}",
            self.config.listen,
            self.config.upstream
        );

        // Block forever
        server.block_until_done().await?;

        Ok(())
    }
}
