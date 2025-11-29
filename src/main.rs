//! Beryl Router - XDP/eBPF Firewall Control Plane
//!
//! This daemon loads the XDP eBPF program and manages firewall rules
//! via configuration file watching.

use anyhow::{Context, Result};
use aya::maps::{HashMap, PerCpuArray};
use beryl_common::{FirewallConfig, PacketAction, Stats};
use beryl_config::Config;
use beryl_dhcp::{Client as DhcpClient, ClientConfig, Server as DhcpServer, database::LeaseDatabase};
use beryl_dns::DnsServer;
use beryl_ebpf::BerylEbpf;
use beryl_wifi::apply_wifi_config;
use clap::Parser;
use notify::{EventKind, RecursiveMode, Watcher};
use std::{net::Ipv4Addr, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{RwLock, mpsc},
    task::JoinHandle,
    time::interval,
};
use tracing::{Level, debug, error, info};
use tracing_subscriber::FmtSubscriber;

mod actuator;
mod api;

#[derive(Debug, Parser)]
#[command(name = "beryl-routerd", about = "XDP/eBPF Firewall for Beryl AX")]
pub struct Args {
    /// Network interface to attach XDP program to (LAN/WAN ingress)
    #[arg(short, long, default_value = "eth0")]
    pub interface: String,

    /// Configuration file path
    #[arg(short, long, default_value = "/etc/beryl/config.toml")]
    pub config: PathBuf,

    /// Use SKB mode instead of native XDP (for testing/compatibility)
    #[arg(long)]
    pub skb_mode: bool,

    /// Statistics reporting interval in seconds
    #[arg(long, default_value = "10")]
    pub stats_interval: u64,

    /// API server bind address
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub api_bind: String,
}

pub struct Router {
    ebpf: BerylEbpf,
    config_path: PathBuf,
    dhcp_handle: Option<JoinHandle<()>>,
    dhcp_client_handle: Option<JoinHandle<()>>,
    dns_handle: Option<JoinHandle<()>>,
    current_config: Option<Config>,
    // Shared state between DHCP and DNS
    lease_db: Option<Arc<RwLock<LeaseDatabase>>>,
}

impl Router {
    pub fn new(args: &Args) -> Result<Self> {
        let mut ebpf = BerylEbpf::load()?;

        // Attach XDP (Ingress)
        ebpf.attach_xdp(&args.interface, args.skb_mode)?;

        // Attach TC (Egress)
        if let Err(e) = ebpf.attach_tc_egress(&args.interface) {
            error!("Failed to attach TC egress: {}", e);
        }

        Ok(Self {
            ebpf,
            config_path: args.config.clone(),
            dhcp_handle: None,
            dhcp_client_handle: None,
            dns_handle: None,
            current_config: None,
            lease_db: None,
        })
    }

    pub async fn load_config(&mut self) -> Result<()> {
        let config = if self.config_path.exists() {
            beryl_config::load_config(&self.config_path)?
        } else {
            info!("Config file not found, using defaults");
            return Ok(());
        };

        self.apply_firewall_config(&config.firewall)?;
        self.apply_dhcp_config(&config.dhcp).await?;
        self.apply_dns_config(&config.dns).await?;
        self.apply_wifi_config(&config.wifi).await?;

        self.current_config = Some(config);
        Ok(())
    }

    pub async fn apply_wifi_config(&mut self, config: &beryl_config::WifiConfig) -> Result<()> {
        // WiFi config application is handled by the library (file generation + reload)
        if let Err(e) = apply_wifi_config(config).await {
            error!("Failed to apply WiFi config: {}", e);
        }
        Ok(())
    }

    pub fn get_current_config(&self) -> Option<Config> {
        self.current_config.clone()
    }

    pub fn apply_firewall_config(&mut self, config: &FirewallConfig) -> Result<()> {
        // Update IP blocklist (XDP Ingress)
        if let Some(map) = self.ebpf.get_map_mut("BLOCKLIST") {
            let mut blocklist: HashMap<_, u32, u32> = HashMap::try_from(map)?;

            // Clear existing entries
            let keys: Vec<u32> = blocklist.keys().filter_map(|k| k.ok()).collect();
            for key in keys {
                let _ = blocklist.remove(&key);
            }

            // Add new blocked IPs
            for ip_str in &config.blocked_ips {
                if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                    let ip_u32 = u32::from(ip);
                    blocklist.insert(ip_u32, PacketAction::Drop as u32, 0)?;
                    debug!(ip = %ip_str, "Added IP to ingress blocklist");
                }
            }
        }

        // Update Port blocklist (XDP Ingress)
        if let Some(map) = self.ebpf.get_map_mut("PORT_BLOCKLIST") {
            let mut port_blocklist: HashMap<_, u16, u32> = HashMap::try_from(map)?;

            let port_keys: Vec<u16> = port_blocklist.keys().filter_map(|k| k.ok()).collect();
            for key in port_keys {
                let _ = port_blocklist.remove(&key);
            }

            for port in &config.blocked_ports {
                port_blocklist.insert(*port, PacketAction::Drop as u32, 0)?;
                debug!(port, "Added port to ingress blocklist");
            }
        }

        // Update Egress blocklist (TC Egress)
        if let Some(map) = self.ebpf.get_map_mut("EGRESS_BLOCK") {
            let mut egress_block: HashMap<_, u32, u32> = HashMap::try_from(map)?;

            let keys: Vec<u32> = egress_block.keys().filter_map(|k| k.ok()).collect();
            for key in keys {
                let _ = egress_block.remove(&key);
            }

            for ip_str in &config.blocked_egress_ips {
                if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                    let ip_u32 = u32::from(ip);
                    egress_block.insert(ip_u32, PacketAction::Drop as u32, 0)?;
                    debug!(ip = %ip_str, "Added IP to egress blocklist");
                }
            }
        }

        info!(
            ingress_ips = config.blocked_ips.len(),
            ingress_ports = config.blocked_ports.len(),
            egress_ips = config.blocked_egress_ips.len(),
            "Firewall configuration applied"
        );

        Ok(())
    }

    pub async fn apply_dhcp_config(&mut self, config: &beryl_config::DhcpConfig) -> Result<()> {
        // --- DHCP Server Handling ---
        if let Some(handle) = self.dhcp_handle.take() {
            handle.abort();
            info!("Stopped existing DHCP server");
        }

        if let Some(server_config) = &config.server {
            if server_config.enabled {
                let db = Arc::new(RwLock::new(LeaseDatabase::new(
                    &server_config.pool,
                    &server_config.static_leases,
                    server_config.lease_file.clone(),
                )));
                self.lease_db = Some(db.clone());

                info!("Starting DHCP server on {}", server_config.interface);
                let mut server = DhcpServer::new(server_config.clone(), db);
                let handle = tokio::spawn(async move {
                    if let Err(e) = server.run().await {
                        error!("DHCP Server failed: {}", e);
                    }
                });
                self.dhcp_handle = Some(handle);
            } else {
                self.lease_db = None;
            }
        }

        // --- DHCP Client Handling ---
        if let Some(handle) = self.dhcp_client_handle.take() {
            handle.abort();
            info!("Stopped existing DHCP client");
        }

        if let Some(client_config) = &config.client {
            info!("Starting DHCP client on {}", client_config.interface);
            let config = client_config.clone();
            let handle = tokio::spawn(async move {
                let mut client = DhcpClient::new(config.clone());
                loop {
                    match client.acquire().await {
                        Ok(lease) => {
                            info!("DHCP Lease acquired: {}/{}", lease.ip, lease.netmask);
                            if let Err(e) = actuator::NetworkActuator::apply_lease(&config.interface, &lease) {
                                error!("Failed to apply DHCP lease: {}", e);
                            }
                            
                            // Renewal logic (simple sleep for 50% of lease time)
                            let sleep_time = Duration::from_secs((lease.lease_time / 2).into());
                            debug!("Sleeping for {:?} before renewal", sleep_time);
                            tokio::time::sleep(sleep_time).await;
                        }
                        Err(e) => {
                            error!("DHCP acquire failed: {}. Retrying in 5s...", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            });
            self.dhcp_client_handle = Some(handle);
        }

        Ok(())
    }

    pub async fn apply_dns_config(
        &mut self,
        config: &beryl_config::DnsConfigWrapper,
    ) -> Result<()> {
        if let Some(handle) = self.dns_handle.take() {
            handle.abort();
            info!("Stopped existing DNS server");
        }

        if let Some(server_config) = &config.server {
            if server_config.enabled {
                // Need lease DB for local resolution
                if let Some(db) = &self.lease_db {
                    info!("Starting DNS server...");
                    // Determine local domain from DHCP config if available?
                    // Ideally DNS config should have it or we grab from DHCP options.
                    // For now, pass None or "lan"
                    let local_domain = Some("lan".to_string());

                    let server = DnsServer::new(server_config.clone(), db.clone(), local_domain);
                    let handle = tokio::spawn(async move {
                        if let Err(e) = server.run().await {
                            error!("DNS Server failed: {}", e);
                        }
                    });
                    self.dns_handle = Some(handle);
                } else {
                    tracing::warn!(
                        "DNS Server enabled but DHCP (and Lease DB) is not initialized. Local resolution will fail."
                    );
                    // We could start it without local resolution, but for now let's skip or start with empty DB?
                    // Or just don't start.
                }
            }
        }
        Ok(())
    }

    pub fn get_stats(&self) -> Result<Stats> {
        let map = self.ebpf.get_map("STATS").context("STATS map not found")?;
        let stats: PerCpuArray<_, Stats> = PerCpuArray::try_from(map)?;

        let per_cpu_stats = stats.get(&0, 0)?;
        let mut total = Stats::default();

        for cpu_stats in per_cpu_stats.iter() {
            total.packets_total += cpu_stats.packets_total;
            total.packets_passed += cpu_stats.packets_passed;
            total.packets_dropped += cpu_stats.packets_dropped;
        }

        Ok(total)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    info!("Starting Beryl Router");

    // Create router instance
    let router = Arc::new(RwLock::new(Router::new(&args)?));

    // Load initial config
    router.write().await.load_config().await?;

    // Channel for config reload signals
    let (tx, mut rx) = mpsc::channel::<()>(1);

    // Set up file watcher for config changes
    let config_path = args.config.clone();
    let tx_watcher = tx.clone();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                let _ = tx_watcher.blocking_send(());
            }
        }
    })?;

    if let Some(parent) = config_path.parent() {
        if parent.exists() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
            info!(path = ?config_path, "Watching config file");
        }
    }

    // Stats reporting task
    let router_stats = router.clone();
    let stats_interval_secs = args.stats_interval;
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(stats_interval_secs));
        loop {
            interval.tick().await;
            let router = router_stats.read().await;
            match router.get_stats() {
                Ok(stats) => {
                    info!(
                        total = stats.packets_total,
                        passed = stats.packets_passed,
                        dropped = stats.packets_dropped,
                        "Packet statistics"
                    );
                }
                Err(e) => {
                    error!("Failed to get stats: {}", e);
                }
            }
        }
    });

    // API Server
    let api_router = router.clone();
    let api_bind = args.api_bind.clone();
    tokio::spawn(async move {
        let app_state = api::AppState { router: api_router };
        let app = api::app(app_state);

        info!("API server listening on {}", api_bind);
        let listener = tokio::net::TcpListener::bind(api_bind).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // Config reload handler
    let router_reload = router.clone();
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            info!("Config change detected, reloading...");
            let mut router = router_reload.write().await;
            if let Err(e) = router.load_config().await {
                error!("Failed to reload config: {}", e);
            }
        }
    });

    // Wait for shutdown signal
    info!("Router running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
