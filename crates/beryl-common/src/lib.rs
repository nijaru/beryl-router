//! Shared types between eBPF and userspace.
//!
//! These types are used in eBPF maps and must be `#[repr(C)]` for correct memory layout.

#![cfg_attr(feature = "ebpf", no_std)]

/// Packet action in blocklist maps.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketAction {
    Pass = 0,
    Drop = 1,
}

impl From<u32> for PacketAction {
    fn from(v: u32) -> Self {
        match v {
            1 => PacketAction::Drop,
            _ => PacketAction::Pass,
        }
    }
}

/// Per-CPU statistics.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub packets_total: u64,
    pub packets_passed: u64,
    pub packets_dropped: u64,
}

/// Configuration for a firewall rule.
#[cfg(not(feature = "ebpf"))]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct FirewallConfig {
    /// Blocked IP addresses (IPv4 as dotted-decimal strings)
    #[serde(default)]
    pub blocked_ips: Vec<String>,
    /// Blocked destination ports
    #[serde(default)]
    pub blocked_ports: Vec<u16>,
    /// Blocked egress IP addresses (LAN -> WAN)
    #[serde(default)]
    pub blocked_egress_ips: Vec<String>,
}

#[cfg(not(feature = "ebpf"))]
impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            blocked_ips: Vec::new(),
            blocked_ports: Vec::new(),
            blocked_egress_ips: Vec::new(),
        }
    }
}
