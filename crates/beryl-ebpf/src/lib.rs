use anyhow::{Context, Result};
use aya::{
    Ebpf, include_bytes_aligned,
    programs::{SchedClassifier, TcAttachType, tc},
};
use aya_log::EbpfLogger;
use tracing::{info, warn};

pub struct BerylEbpf {
    ebpf: Ebpf,
}

impl BerylEbpf {
    pub fn load() -> Result<Self> {
        // Load eBPF bytecode
        #[cfg(debug_assertions)]
        let mut ebpf = Ebpf::load(include_bytes_aligned!(
            "../../../beryl-router-ebpf/target/bpfel-unknown-none/debug/beryl-router-ebpf"
        ))?;

        #[cfg(not(debug_assertions))]
        let mut ebpf = Ebpf::load(include_bytes_aligned!(
            "../../../beryl-router-ebpf/target/bpfel-unknown-none/release/beryl-router-ebpf"
        ))?;

        // Initialize eBPF logging
        if let Err(e) = EbpfLogger::init(&mut ebpf) {
            warn!("Failed to initialize eBPF logger: {}", e);
        }

        Ok(Self { ebpf })
    }

    pub fn attach_xdp(&mut self, iface: &str, skb_mode: bool) -> Result<()> {
        let program: &mut aya::programs::Xdp = self
            .ebpf
            .program_mut("xdp_firewall")
            .context("XDP program not found")?
            .try_into()?;
        program.load()?;

        let flags = if skb_mode {
            aya::programs::XdpFlags::SKB_MODE
        } else {
            aya::programs::XdpFlags::default()
        };

        program
            .attach(iface, flags)
            .context("Failed to attach XDP program")?;

        info!(
            iface,
            mode = if skb_mode { "SKB" } else { "Native" },
            "XDP program attached"
        );
        Ok(())
    }

    pub fn attach_tc_egress(&mut self, iface: &str) -> Result<()> {
        // Ensure qdisc exists (usually clsact)
        let _ = tc::qdisc_add_clsact(iface); // Ignore error if already exists

        let program: &mut SchedClassifier = self
            .ebpf
            .program_mut("tc_egress")
            .context("TC egress program not found")?
            .try_into()?;
        program.load()?;

        program
            .attach(iface, TcAttachType::Egress)
            .context("Failed to attach TC egress program")?;

        info!(iface, "TC egress program attached");
        Ok(())
    }

    pub fn get_map_mut(&mut self, name: &str) -> Option<&mut aya::maps::Map> {
        self.ebpf.map_mut(name)
    }

    pub fn get_map(&self, name: &str) -> Option<&aya::maps::Map> {
        self.ebpf.map(name)
    }
}
