pub mod resolver;
pub mod server;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DnsConfig {
    pub enabled: bool,
    pub listen: Vec<SocketAddr>,
    pub upstream: Vec<SocketAddr>,
}

pub use server::DnsServer;
