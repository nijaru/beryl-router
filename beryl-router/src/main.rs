//! Beryl Router - XDP/eBPF Firewall Control Plane
//!
//! This daemon loads the XDP eBPF program and manages firewall rules
//! via configuration file watching.

use anyhow::{Context, Result};
use aya::{
    include_bytes_aligned,
    maps::{HashMap, PerCpuArray},
    programs::{Xdp, XdpFlags},
    Ebpf,
};
use aya_log::EbpfLogger;
use beryl_router_common::{FirewallConfig, PacketAction, Stats};
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
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Parser)]
#[command(name = "beryl-router", about = "XDP/eBPF Firewall for Beryl AX")]
struct Args {
    /// Network interface to attach XDP program to
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
    ebpf: Ebpf,
    config_path: PathBuf,
}

impl Router {
    fn new(args: &Args) -> Result<Self> {
        // Load eBPF bytecode compiled from beryl-router-ebpf
        #[cfg(debug_assertions)]
        let mut ebpf = Ebpf::load(include_bytes_aligned!(
            "../../target/bpfel-unknown-none/debug/beryl-router-ebpf"
        ))?;

        #[cfg(not(debug_assertions))]
        let mut ebpf = Ebpf::load(include_bytes_aligned!(
            "../../target/bpfel-unknown-none/release/beryl-router-ebpf"
        ))?;

        // Initialize eBPF logging
        if let Err(e) = EbpfLogger::init(&mut ebpf) {
            warn!("Failed to initialize eBPF logger: {}", e);
        }

        // Load and attach XDP program
        let program: &mut Xdp = ebpf
            .program_mut("xdp_firewall")
            .context("XDP program not found")?
            .try_into()?;
        program.load()?;

        let flags = if args.skb_mode {
            XdpFlags::SKB_MODE
        } else {
            XdpFlags::default()
        };

        program
            .attach(&args.interface, flags)
            .context("Failed to attach XDP program")?;

        info!(
            interface = %args.interface,
            mode = if args.skb_mode { "SKB" } else { "Native" },
            "XDP program attached"
        );

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
        // Update IP blocklist
        let mut blocklist: HashMap<_, u32, u32> =
            HashMap::try_from(self.ebpf.map_mut("BLOCKLIST").context("BLOCKLIST map not found")?)?;

        // Clear existing entries by iterating and removing
        let keys: Vec<u32> = blocklist.keys().filter_map(|k| k.ok()).collect();
        for key in keys {
            let _ = blocklist.remove(&key);
        }

        // Add new blocked IPs
        for ip_str in &config.blocked_ips {
            if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                let ip_u32 = u32::from(ip);
                blocklist.insert(ip_u32, PacketAction::Drop as u32, 0)?;
                debug!(ip = %ip_str, "Added IP to blocklist");
            } else {
                warn!(ip = %ip_str, "Invalid IP address in config");
            }
        }

        // Update port blocklist
        let mut port_blocklist: HashMap<_, u16, u32> = HashMap::try_from(
            self.ebpf
                .map_mut("PORT_BLOCKLIST")
                .context("PORT_BLOCKLIST map not found")?,
        )?;

        let port_keys: Vec<u16> = port_blocklist.keys().filter_map(|k| k.ok()).collect();
        for key in port_keys {
            let _ = port_blocklist.remove(&key);
        }

        for port in &config.blocked_ports {
            port_blocklist.insert(*port, PacketAction::Drop as u32, 0)?;
            debug!(port, "Added port to blocklist");
        }

        info!(
            blocked_ips = config.blocked_ips.len(),
            blocked_ports = config.blocked_ports.len(),
            "Configuration applied"
        );

        Ok(())
    }

    fn get_stats(&self) -> Result<Stats> {
        let stats: PerCpuArray<_, Stats> =
            PerCpuArray::try_from(self.ebpf.map("STATS").context("STATS map not found")?)?;

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
