# nftables Integration

How beryl-routerd manages nftables rules for NAT and stateful firewall.

## Why nftables (not pure XDP)

| Task | XDP | nftables | Winner |
|------|-----|----------|--------|
| Stateless drop | ✓ Fast | ✓ Slower | XDP |
| NAT (SNAT/DNAT) | Complex | ✓ Built-in | nftables |
| Connection tracking | ✗ | ✓ conntrack | nftables |
| Port forwarding | ✗ | ✓ DNAT | nftables |
| Stateful firewall | ✗ | ✓ ct state | nftables |

**Hybrid approach:** XDP for fast-path drops, nftables for stateful operations.

## nftables Table Structure

```nft
#!/usr/sbin/nft -f

# beryl-router nftables rules
# Managed by beryl-routerd - DO NOT EDIT MANUALLY

table inet beryl {
    # Blocklist set (synced from eBPF for redundancy)
    set blocklist_v4 {
        type ipv4_addr
        flags interval
    }

    set blocklist_v6 {
        type ipv6_addr
        flags interval
    }

    # Port forward targets
    set portfwd_tcp {
        type inet_service : ipv4_addr . inet_service
        flags interval
    }

    chain input {
        type filter hook input priority filter; policy drop;

        # Allow established/related
        ct state established,related accept

        # Allow loopback
        iif lo accept

        # Allow ICMP
        ip protocol icmp accept
        ip6 nexthdr icmpv6 accept

        # Allow from LAN
        iifname "br-lan" jump input_lan

        # Drop from WAN (default policy)
    }

    chain input_lan {
        # SSH
        tcp dport 22 accept
        # HTTP API
        tcp dport 8080 accept
        # DNS
        tcp dport 53 accept
        udp dport 53 accept
        # DHCP
        udp dport { 67, 68 } accept
    }

    chain forward {
        type filter hook forward priority filter; policy drop;

        # Allow established/related
        ct state established,related accept

        # LAN to WAN (outbound)
        iifname "br-lan" oifname "eth0" accept

        # Port forwards (DNAT'd traffic)
        ct status dnat accept
    }

    chain output {
        type filter hook output priority filter; policy accept;
    }

    # NAT chains
    chain prerouting {
        type nat hook prerouting priority dstnat;

        # Port forwards
        iifname "eth0" tcp dport 2222 dnat to 192.168.8.50:22
    }

    chain postrouting {
        type nat hook postrouting priority srcnat;

        # Masquerade outbound
        oifname "eth0" masquerade
    }
}

# Flowtable for hardware offload
table inet beryl_offload {
    flowtable ft {
        hook ingress priority filter
        devices = { eth0, eth1 }
        flags offload  # Hardware offload if supported
    }

    chain forward {
        type filter hook forward priority filter - 1;

        # Offload established connections
        ct state established flow add @ft
    }
}
```

## Rust Integration

### Option 1: Shell out to `nft` (Simple, Phase 1)

```rust
// crates/beryl-nft/src/lib.rs

use anyhow::Result;
use tokio::process::Command;

pub struct NftManager {
    table_name: String,
}

impl NftManager {
    pub fn new() -> Self {
        Self {
            table_name: "beryl".to_string(),
        }
    }

    /// Apply full ruleset
    pub async fn apply_ruleset(&self, rules: &str) -> Result<()> {
        let output = Command::new("nft")
            .args(["-f", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "nft failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Add IP to blocklist
    pub async fn block_ip(&self, ip: &str) -> Result<()> {
        Command::new("nft")
            .args([
                "add", "element", "inet", &self.table_name,
                "blocklist_v4", &format!("{{ {} }}", ip)
            ])
            .status()
            .await?;
        Ok(())
    }

    /// Add port forward
    pub async fn add_port_forward(&self, pf: &PortForward) -> Result<()> {
        let rule = format!(
            "add rule inet {} prerouting iifname \"eth0\" {} dport {} dnat to {}:{}",
            self.table_name,
            pf.proto,
            pf.external_port,
            pf.internal_ip,
            pf.internal_port
        );
        Command::new("nft").args(rule.split_whitespace()).status().await?;
        Ok(())
    }

    /// Generate full ruleset from config
    pub fn generate_ruleset(&self, config: &FirewallConfig) -> String {
        let mut out = String::new();

        out.push_str("flush table inet beryl\n");
        out.push_str("table inet beryl {\n");

        // Sets
        out.push_str("  set blocklist_v4 { type ipv4_addr; flags interval; }\n");

        // Input chain
        out.push_str(&format!(
            "  chain input {{ type filter hook input priority filter; policy {}; }}\n",
            match config.input {
                Policy::Drop => "drop",
                Policy::Accept => "accept",
            }
        ));

        // ... generate rest

        out.push_str("}\n");
        out
    }
}
```

### Option 2: libnftables bindings (Future, more robust)

```rust
// Using nftables-rs crate (if it matures)
use nftables::Nftables;

let nft = Nftables::new()?;
nft.add_table("inet", "beryl")?;
nft.add_chain("inet", "beryl", "input", ChainType::Filter, Hook::Input, Priority::Filter)?;
// ...
```

**Recommendation:** Start with Option 1 (shell out), migrate to bindings if needed.

## Synchronization with eBPF

eBPF maps and nftables sets should stay in sync:

```rust
impl Daemon {
    async fn sync_blocklist(&self, ips: &[IpAddr]) -> Result<()> {
        // Update eBPF map (fast path)
        for ip in ips {
            self.ebpf.add_to_blocklist(*ip)?;
        }

        // Update nftables set (backup/redundancy)
        self.nft.set_blocklist(ips).await?;

        Ok(())
    }
}
```

## Mode-Specific Rules

### Router Mode

```nft
# Full NAT, firewall
chain postrouting {
    oifname "eth0" masquerade
}
```

### AP Mode

```nft
# No NAT, bridge only
# Delete masquerade rule
# Forward all bridged traffic
chain forward {
    iifname "br-lan" oifname "br-lan" accept
}
```

### Repeater Mode

```nft
# NAT on wlan0 (upstream)
chain postrouting {
    oifname "wlan0" masquerade
}
```

## Initialization Sequence

1. Flush existing beryl tables
2. Create base table structure
3. Load config rules
4. Enable flowtable offload
5. Start monitoring for config changes

```rust
impl NftManager {
    pub async fn initialize(&self, config: &Config) -> Result<()> {
        // Flush any existing
        self.flush().await?;

        // Apply base rules
        let ruleset = self.generate_ruleset(&config.firewall);
        self.apply_ruleset(&ruleset).await?;

        // Enable offload if supported
        if config.firewall.hw_offload {
            self.enable_offload().await?;
        }

        Ok(())
    }
}
```

## Testing

```bash
# View current rules
nft list ruleset

# Test syntax without applying
nft -c -f rules.nft

# Monitor changes
nft monitor

# Check flowtable offload
nft list flowtables
cat /proc/net/nf_conntrack | grep OFFLOAD
```
