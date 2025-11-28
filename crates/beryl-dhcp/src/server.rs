use crate::database::LeaseDatabase;
use dhcproto::{
    v4, Decodable, Encodable, Encoder,
};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::UdpSocket;

use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub enabled: bool,
    pub interface: String,
    pub pool: PoolConfig,
    pub options: OptionsConfig,
    #[serde(default)]
    pub static_leases: Vec<StaticLease>,
    pub lease_file: Option<PathBuf>,
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
    db: LeaseDatabase,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        let db = LeaseDatabase::new(&config.pool, &config.static_leases, config.lease_file.clone());
        Self { config, db }
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
                    if let Err(e) = self.handle_packet(&buf[..len], &socket).await {
                        tracing::error!("Failed to handle packet: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("DHCP receive error: {}", e);
                }
            }
        }
    }

    async fn handle_packet(&mut self, buf: &[u8], socket: &UdpSocket) -> anyhow::Result<()> {
        let mut decoder = dhcproto::Decoder::new(buf);
        let msg = v4::Message::decode(&mut decoder)?;
        
        // Only handle BootRequest (client to server)
        if msg.opcode() != v4::Opcode::BootRequest {
            return Ok(());
        }

        // Extract message type option
        let msg_type = match msg.opts().get(v4::OptionCode::MessageType) {
            Some(v4::DhcpOption::MessageType(t)) => t,
            _ => return Ok(()), // Ignore if no message type
        };

        tracing::debug!("DHCP Message Type: {:?}", msg_type);

        match msg_type {
            v4::MessageType::Discover => self.handle_discover(&msg, socket).await,
            v4::MessageType::Request => self.handle_request(&msg, socket).await,
            _ => Ok(()), // Ignore Release/Decline for now
        }
    }

    async fn handle_discover(&mut self, msg: &v4::Message, socket: &UdpSocket) -> anyhow::Result<()> {
        let mac = msg.chaddr(); // Client MAC
        let req_ip = msg.opts().get(v4::OptionCode::RequestedIpAddress).and_then(|opt| {
            if let v4::DhcpOption::RequestedIpAddress(ip) = opt {
                Some(*ip)
            } else {
                None
            }
        });

        if let Some(lease) = self.db.allocate_ip(mac, req_ip) {
            tracing::info!("Offering IP {} to {:x?}", lease.ip, mac);
            
            let mut offer = v4::Message::default();
            offer.set_opcode(v4::Opcode::BootReply);
            offer.set_xid(msg.xid());
            offer.set_yiaddr(lease.ip);
            offer.set_chaddr(mac);
            // offer.set_flags(msg.flags()); // Keep broadcast flag if present
            
            // Options
            offer.opts_mut().insert(v4::DhcpOption::MessageType(v4::MessageType::Offer));
            offer.opts_mut().insert(v4::DhcpOption::ServerIdentifier(self.get_server_ip()));
            offer.opts_mut().insert(v4::DhcpOption::AddressLeaseTime(self.db.get_duration().as_secs() as u32));
            offer.opts_mut().insert(v4::DhcpOption::SubnetMask(Ipv4Addr::new(255, 255, 255, 0))); // TODO: Configurable mask
            
            if let Some(gw) = self.config.options.gateway {
                offer.opts_mut().insert(v4::DhcpOption::Router(vec![gw]));
            }
            if !self.config.options.dns.is_empty() {
                offer.opts_mut().insert(v4::DhcpOption::DomainNameServer(self.config.options.dns.clone()));
            }

            self.send_response(offer, socket).await?;
        }

        Ok(())
    }

    async fn handle_request(&mut self, msg: &v4::Message, socket: &UdpSocket) -> anyhow::Result<()> {
        let mac = msg.chaddr();
        // Check if this is a request for a specific IP
        let req_ip = msg.opts().get(v4::OptionCode::RequestedIpAddress).and_then(|opt| {
             if let v4::DhcpOption::RequestedIpAddress(ip) = opt {
                Some(*ip)
            } else {
                None
            }
        });
        
        // Or verify 'ciaddr' (client IP) for renewals
        let target_ip = req_ip.or_else(|| {
             if !msg.ciaddr().is_unspecified() {
                 Some(msg.ciaddr())
             } else {
                 None
             }
        });

        if let Some(ip) = target_ip {
             // Verify we actually leased this IP to this MAC
             // Simple check: re-allocate returns the same lease if exists
             if let Some(lease) = self.db.allocate_ip(mac, Some(ip)) {
                 if lease.ip == ip {
                    tracing::info!("ACKing IP {} for {:x?}", ip, mac);

                    let mut ack = v4::Message::default();
                    ack.set_opcode(v4::Opcode::BootReply);
                    ack.set_xid(msg.xid());
                    ack.set_yiaddr(ip);
                    ack.set_chaddr(mac);
                    
                    ack.opts_mut().insert(v4::DhcpOption::MessageType(v4::MessageType::Ack));
                    ack.opts_mut().insert(v4::DhcpOption::ServerIdentifier(self.get_server_ip()));
                    ack.opts_mut().insert(v4::DhcpOption::AddressLeaseTime(self.db.get_duration().as_secs() as u32));
                    ack.opts_mut().insert(v4::DhcpOption::SubnetMask(Ipv4Addr::new(255, 255, 255, 0)));

                    if let Some(gw) = self.config.options.gateway {
                        ack.opts_mut().insert(v4::DhcpOption::Router(vec![gw]));
                    }
                     if !self.config.options.dns.is_empty() {
                        ack.opts_mut().insert(v4::DhcpOption::DomainNameServer(self.config.options.dns.clone()));
                    }
                    
                    self.send_response(ack, socket).await?;
                 } else {
                     // NAK?
                     tracing::warn!("Request for {} from {:x?} invalid (got {})", ip, mac, lease.ip);
                     // Send NAK logic here
                 }
             }
        }

        Ok(())
    }

    async fn send_response(&self, msg: v4::Message, socket: &UdpSocket) -> anyhow::Result<()> {
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        msg.encode(&mut encoder)?;
        
        // Broadcast to 255.255.255.255 port 68
        let dest = SocketAddr::new(Ipv4Addr::BROADCAST.into(), 68);
        socket.send_to(&buf, dest).await?;
        
        Ok(())
    }
    
    fn get_server_ip(&self) -> Ipv4Addr {
        // TODO: Get actual IP from interface or config
        self.config.options.gateway.unwrap_or(Ipv4Addr::new(192, 168, 8, 1))
    }
}