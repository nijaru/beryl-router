use beryl_common::FirewallConfig;
use beryl_dhcp::ServerConfig as DhcpServerConfig;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub system: SystemConfig,
    pub api: ApiConfig,
    pub mode: ModeConfig,
    pub interfaces: InterfacesConfig,
    #[serde(default)]
    pub firewall: FirewallConfig,
    #[serde(default)]
    pub dhcp: DhcpConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemConfig {
    pub hostname: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_timezone() -> String { "UTC".to_string() }
fn default_log_level() -> String { "info".to_string() }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub listen: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModeConfig {
    #[serde(rename = "type")]
    pub mode_type: OperatingMode,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperatingMode {
    Router,
    Ap,
    Repeater,
    Wireguard,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterfacesConfig {
    pub wan: InterfaceConfig,
    pub lan: InterfaceConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterfaceConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub iface_type: Option<String>, // dhcp, static, pppoe (WAN only)
    pub address: Option<String>, // CIDR (LAN only usually)
    pub members: Option<Vec<String>>, // For bridge (LAN)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DhcpConfig {
    #[serde(default)]
    pub server: Option<DhcpServerConfig>,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}