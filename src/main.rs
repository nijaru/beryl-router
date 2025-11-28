//! Beryl Router - XDP/eBPF Firewall Control Plane
//!
//! This daemon loads the XDP eBPF program and manages firewall rules
//! via configuration file watching.

use anyhow::{Context, Result};
use aya::maps::{HashMap, PerCpuArray};
use beryl_common::{FirewallConfig, PacketAction, Stats};
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
    fs,
    sync::{mpsc, RwLock},
    time::interval,
};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Parser)]
#[command(name = "beryl-routerd", about = "XDP/eBPF Firewall for Beryl AX")]
struct Args {
    /// Network interface to attach XDP program to (LAN/WAN ingress)
    #[arg(short, long, default_value = "eth0")]
    interface: String,

    /// Configuration file path
    #[arg(short, long, default_value = "/etc/beryl-router/config.json")]
    config: PathBuf,

    /// Use SKB mode instead of native XDP (for testing/compatibility)
    #[arg(long)]
    skb_mode: bool,

    /// Statistics reporting interval in seconds
    #[arg(long, default_value = "10")]
    stats_interval: u64,
}

struct Router {
    ebpf: BerylEbpf,
    config_path: PathBuf,
}

impl Router {
    fn new(args: &Args) -> Result<Self> {
        let mut ebpf = BerylEbpf::load()?;

        // Attach XDP (Ingress)
        ebpf.attach_xdp(&args.interface, args.skb_mode)?;
        
        // Attach TC (Egress) - usually same interface for single-NIC testing, 
        // or eth0 (WAN) for router mode. For now, attach to same interface.
        if let Err(e) = ebpf.attach_tc_egress(&args.interface) {
            error!("Failed to attach TC egress: {}", e);
            // Don't fail hard, maybe kernel doesn't support it or qdisc issue
        }

        Ok(Self {
            ebpf,
            config_path: args.config.clone(),
        })
    }

    async fn load_config(&mut self) -> Result<()> {
        let config: FirewallConfig = if self.config_path.exists() {
            let contents = fs::read_to_string(&self.config_path).await?;
            serde_json::from_str(&contents)?
        } else {
            info!("Config file not found, using defaults");
            FirewallConfig::default()
        };

        self.apply_config(&config)?;
        Ok(())
    }

    fn apply_config(&mut self, config: &FirewallConfig) -> Result<()> {
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
            "Configuration applied"
        );

        Ok(())
    }

    fn get_stats(&self) -> Result<Stats> {
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
