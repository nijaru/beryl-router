use anyhow::{Context, Result};
use beryl_dhcp::DhcpLease;
use std::process::Command;
use tracing::{info, warn};

/// Applies a DHCP lease to the system (IP, Route, DNS)
pub struct NetworkActuator;

impl NetworkActuator {
    pub fn apply_lease(interface: &str, lease: &DhcpLease) -> Result<()> {
        info!("Applying DHCP lease to {}: IP {}/{}", interface, lease.ip, lease.netmask);

        // 1. Apply IP Address
        // ip addr flush dev {interface}
        // ip addr add {ip}/{mask} dev {interface}
        
        // Calculate CIDR prefix from netmask
        let prefix = ip_mask_to_prefix(lease.netmask);
        let cidr = format!("{}/{}", lease.ip, prefix);

        run_cmd("ip", &["addr", "flush", "dev", interface])?;
        run_cmd("ip", &["addr", "add", &cidr, "dev", interface])?;

        // 2. Apply Default Gateway
        if let Some(gw) = lease.gateway {
            info!("Setting default gateway: {}", gw);
            // ip route add default via {gw} dev {interface}
            // First delete existing default to be safe
            let _ = run_cmd("ip", &["route", "del", "default"]); 
            run_cmd("ip", &["route", "add", "default", "via", &gw.to_string(), "dev", interface])?;
        }

        // 3. Apply DNS
        // Write to /tmp/resolv.conf.auto (OpenWrt style) or /etc/resolv.conf
        // For now, we assume we control /etc/resolv.conf
        if !lease.dns.is_empty() {
            let mut resolv_content = String::new();
            for dns in &lease.dns {
                resolv_content.push_str(&format!("nameserver {}
", dns));
            }
            info!("Updating /etc/resolv.conf with DNS: {:?}", lease.dns);
            std::fs::write("/etc/resolv.conf", resolv_content)
                .context("Failed to write /etc/resolv.conf")?;
        }

        Ok(())
    }
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .context(format!("Failed to execute {} {:?}", cmd, args))?;

    if !status.success() {
        warn!("Command {} {:?}" returned non-zero exit code", cmd, args);
        // Don't hard fail on commands like "ip route del" if route doesn't exist
        if cmd == "ip" && args.first() == Some(&"addr") {
             return Err(anyhow::anyhow!("Command failed: {} {:?}", cmd, args));
        }
    }
    Ok(())
}

fn ip_mask_to_prefix(mask: std::net::Ipv4Addr) -> u32 {
    u32::from(mask).count_ones()
}
