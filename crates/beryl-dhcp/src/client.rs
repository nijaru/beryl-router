use anyhow::{Context, Result};
use dhcproto::{v4, Decodable, Decoder, Encodable, Encoder};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub interface: String,
    pub mac: [u8; 6],
}

#[derive(Debug)]
pub struct DhcpLease {
    pub ip: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub gateway: Option<Ipv4Addr>,
    pub dns: Vec<Ipv4Addr>,
    pub lease_time: u32,
    pub server_id: Ipv4Addr,
}

pub struct Client {
    config: ClientConfig,
    xid: u32,
}

impl Client {
    #[must_use]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            xid: rand::random(),
        }
    }

    /// Run the DHCP handshake to acquire a lease
    ///
    /// # Errors
    ///
    /// Returns an error if the socket cannot be bound, or if any part of the DHCP
    /// handshake (DISCOVER, OFFER, REQUEST, ACK) fails or times out.
    pub async fn acquire(&mut self) -> Result<DhcpLease> {
        // In a real implementation, we need a RAW socket to broadcast 255.255.255.255
        // from 0.0.0.0 before we have an IP. 
        // For this MVP, we'll assume we can bind to 0.0.0.0:68 and broadcast.
        // Note: In production, this often requires SO_BINDTODEVICE or raw sockets 
        // if multiple interfaces are present.
        
        let socket = UdpSocket::bind("0.0.0.0:68").await
            .context("Failed to bind to DHCP client port 68")?;
        
        socket.set_broadcast(true).context("Failed to set broadcast")?;
        
        // 1. Send DISCOVER
        self.send_discover(&socket).await?;
        debug!("Sent DHCP DISCOVER on {}", self.config.interface);

        // 2. Wait for OFFER
        let offer = self.wait_for_offer(&socket).await?;
        info!("Received DHCP OFFER: {} from {}", offer.yiaddr(), offer.siaddr());

        // 3. Send REQUEST
        self.send_request(&socket, &offer).await?;
        debug!("Sent DHCP REQUEST for {}", offer.yiaddr());

        // 4. Wait for ACK
        let lease = self.wait_for_ack(&socket).await?;
        info!("Received DHCP ACK. Lease acquired for {}", lease.ip);

        Ok(lease)
    }

    async fn send_discover(&self, socket: &UdpSocket) -> Result<()> {
        let mut packet = v4::Message::default();
        packet.set_xid(self.xid);
        packet.set_flags(v4::Flags::default().set_broadcast());
        packet.set_chaddr(&self.config.mac);
        
        packet.opts_mut()
            .insert(v4::DhcpOption::MessageType(v4::MessageType::Discover));
        
        // Request specific parameters
        packet.opts_mut().insert(v4::DhcpOption::ParameterRequestList(vec![
            v4::OptionCode::SubnetMask,
            v4::OptionCode::Router,
            v4::OptionCode::DomainNameServer,
            v4::OptionCode::DomainName,
        ]));

        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        packet.encode(&mut encoder)?;

        socket.send_to(&buf, "255.255.255.255:67").await?;
        Ok(())
    }

    async fn wait_for_offer(&self, socket: &UdpSocket) -> Result<v4::Message> {
        let mut buf = [0u8; 1500];
        loop {
            let (n, _addr) = socket.recv_from(&mut buf).await?;
            let mut decoder = Decoder::new(&buf[..n]);
            match v4::Message::decode(&mut decoder) {
                Ok(msg) if msg.xid() == self.xid => {
                    if let Some(v4::DhcpOption::MessageType(v4::MessageType::Offer)) = msg.opts().get(v4::OptionCode::MessageType) {
                        return Ok(msg);
                    }
                }
                Ok(_) => debug!("Ignored DHCP message with mismatched XID"),
                Err(e) => warn!("Failed to decode packet: {}", e),
            }
        }
    }

    async fn send_request(&self, socket: &UdpSocket, offer: &v4::Message) -> Result<()> {
        let mut packet = v4::Message::default();
        packet.set_xid(self.xid);
        packet.set_flags(v4::Flags::default().set_broadcast());
        packet.set_chaddr(&self.config.mac);
        
        packet.opts_mut()
            .insert(v4::DhcpOption::MessageType(v4::MessageType::Request));
        packet.opts_mut().insert(v4::DhcpOption::RequestedIpAddress(offer.yiaddr()));
        
        // Server Identifier is required in REQUEST
        if let Some(v4::DhcpOption::ServerIdentifier(si)) = offer.opts().get(v4::OptionCode::ServerIdentifier) {
            packet.opts_mut().insert(v4::DhcpOption::ServerIdentifier(*si));
        }

        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        packet.encode(&mut encoder)?;

        socket.send_to(&buf, "255.255.255.255:67").await?;
        Ok(())
    }

    async fn wait_for_ack(&self, socket: &UdpSocket) -> Result<DhcpLease> {
        let mut buf = [0u8; 1500];
        loop {
            let (n, _addr) = socket.recv_from(&mut buf).await?;
            let mut decoder = Decoder::new(&buf[..n]);
            match v4::Message::decode(&mut decoder) {
                Ok(msg) if msg.xid() == self.xid => {
                     match msg.opts().get(v4::OptionCode::MessageType) {
                        Some(v4::DhcpOption::MessageType(v4::MessageType::Ack)) => {
                            return Ok(Self::parse_lease(&msg));
                        }
                        Some(v4::DhcpOption::MessageType(v4::MessageType::Nak)) => {
                            return Err(anyhow::anyhow!("Received DHCP NAK"));
                        }
                        _ => {}
                     }
                }
                _ => {}
            }
        }
    }

    fn parse_lease(msg: &v4::Message) -> DhcpLease {
        let ip = msg.yiaddr();
        
        let netmask = match msg.opts().get(v4::OptionCode::SubnetMask) {
            Some(v4::DhcpOption::SubnetMask(mask)) => *mask,
            _ => Ipv4Addr::new(255, 255, 255, 0), // Default fallback
        };

        let gateway = match msg.opts().get(v4::OptionCode::Router) {
            Some(v4::DhcpOption::Router(routers)) => routers.first().copied(),
            _ => None,
        };

        let dns = match msg.opts().get(v4::OptionCode::DomainNameServer) {
            Some(v4::DhcpOption::DomainNameServer(servers)) => servers.clone(),
            _ => Vec::new(),
        };

        let lease_time = match msg.opts().get(v4::OptionCode::AddressLeaseTime) {
            Some(v4::DhcpOption::AddressLeaseTime(secs)) => *secs,
            _ => 3600,
        };

        let server_id = match msg.opts().get(v4::OptionCode::ServerIdentifier) {
            Some(v4::DhcpOption::ServerIdentifier(id)) => *id,
            _ => Ipv4Addr::UNSPECIFIED,
        };

        DhcpLease {
            ip,
            netmask,
            gateway,
            dns,
            lease_time,
            server_id,
        }
    }
}