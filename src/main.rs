//! Beryl Router - XDP/eBPF Firewall Control Plane
//!
//! This daemon loads the XDP eBPF program and manages firewall rules
//! via configuration file watching.

use anyhow::{Context, Result};
use aya::maps::{HashMap, PerCpuArray};
use beryl_common::{FirewallConfig, PacketAction, Stats};
use beryl_config::Config;
use beryl_dhcp::Server as DhcpServer;
use beryl_ebpf::BerylEbpf;
use clap::Parser;
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    net::Ipv4Addr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::interval,
};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

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
    current_config: Option<Config>,
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
            current_config: None,
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
        
        self.current_config = Some(config);
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
        // Stop existing DHCP server if running
        if let Some(handle) = self.dhcp_handle.take() {
            handle.abort(); // Simple cancellation
            info!("Stopped existing DHCP server");
        }

        if let Some(server_config) = &config.server {
            if server_config.enabled {
                info!("Starting DHCP server on {}", server_config.interface);
                let mut server = DhcpServer::new(server_config.clone());
                let handle = tokio::spawn(async move {
                    if let Err(e) = server.run().await {
                        error!("DHCP Server failed: {}", e);
                    }
                });
                self.dhcp_handle = Some(handle);
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