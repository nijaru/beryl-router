# System Design: Beryl Router

Full-featured Rust router for GL-MT3000 (Beryl AX).

## Hardware

| Component | Spec | Notes |
|-----------|------|-------|
| SoC | MediaTek MT7981 (Filogic 820) | Dual-core ARM Cortex-A53 @ 1.3GHz |
| RAM | 512MB DDR4 | Comfortable for Rust services |
| Flash | 512MB NAND | Plenty for OS + configs |
| Ethernet | 2x Gigabit (eth0=WAN, eth1=LAN) | RTL8221B PHY |
| WiFi | MT7976 (WiFi 6 AX3000) | 2x2 2.4GHz + 2x2 5GHz |

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         USER SPACE                                │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │                    beryl-routerd                          │    │
│  │  (main orchestrator: mode mgmt, eBPF loader, REST API)   │    │
│  └──────────────────────────────────────────────────────────┘    │
│           │              │              │              │          │
│           ▼              ▼              ▼              ▼          │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐   │
│  │   dhcp4d    │ │   dhcp4c    │ │    dnsd     │ │  wifid   │   │
│  │  (server)   │ │  (client)   │ │ (resolver)  │ │(hostapd) │   │
│  └─────────────┘ └─────────────┘ └─────────────┘ └──────────┘   │
│                                                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐                 │
│  │   ntpd      │ │   webui     │ │  wireguard  │                 │
│  │  (chrony?)  │ │ (optional)  │ │  (optional) │                 │
│  └─────────────┘ └─────────────┘ └─────────────┘                 │
│                                                                   │
├──────────────────────────────────────────────────────────────────┤
│                        KERNEL SPACE                               │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────────┐ │
│  │  XDP prog   │ │  TC-BPF     │ │         nftables            │ │
│  │  (ingress)  │ │  (egress)   │ │  (NAT, conntrack, mangle)   │ │
│  │             │ │             │ │                             │ │
│  │ • Blocklist │ │ • QoS mark  │ │ • Masquerade (SNAT)         │ │
│  │ • Rate limit│ │ • Egress fw │ │ • Port forwards (DNAT)      │ │
│  │ • DNS redir │ │             │ │ • Stateful firewall         │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────────┘ │
│           │              │                      │                 │
│           ▼              ▼                      ▼                 │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │                  Linux Network Stack                      │    │
│  │         (routing, bridging, conntrack, neighbor)         │    │
│  └──────────────────────────────────────────────────────────┘    │
│                              │                                    │
│                              ▼                                    │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │              MTK Flow Offload (hardware)                  │    │
│  │         (fast-path established connections)              │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                   │
├──────────────────────────────────────────────────────────────────┤
│                          HARDWARE                                 │
├──────────────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌────────────────────────────────┐  │
│  │   WAN    │  │   LAN    │  │      WiFi (MT7976)             │  │
│  │   eth0   │  │   eth1   │  │   phy0: wlan0 (5GHz AP)        │  │
│  │          │  │          │  │   phy1: wlan1 (2.4GHz AP)      │  │
│  └──────────┘  └──────────┘  └────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

## Operating Modes

### Router Mode (Default)

```
Internet ─── [eth0/WAN] ─── NAT ─── [eth1/LAN + wlan*] ─── Clients
                 │                         │
            DHCP client              DHCP server
```

- WAN: DHCP client (or static/PPPoE)
- LAN: Bridge (eth1 + wlan0 + wlan1)
- NAT: Masquerade WAN
- Services: DHCP server, DNS, firewall

### AP Mode (Bridge)

```
Upstream Router ─── [eth0/WAN] ═══ Bridge ═══ [eth1/LAN + wlan*] ─── Clients
                                    │
                              No NAT, no DHCP
```

- All interfaces bridged (br0)
- No NAT, no DHCP server
- Upstream router handles addressing
- Just WiFi AP + switch

### Repeater Mode

```
Upstream WiFi ─── [wlan0/client] ─── NAT ─── [eth1/LAN + wlan1/AP] ─── Clients
```

- wlan0: Client to upstream WiFi
- wlan1 + eth1: Bridge for downstream
- Double NAT (or use WDS if supported)

### WireGuard Client Mode

```
Internet ─── [eth0/WAN] ─── wg0 tunnel ─── VPN Server
                              │
                    [eth1/LAN + wlan*] ─── Clients (all traffic via VPN)
```

## Packet Flow

### Ingress (WAN → LAN)

```
Packet arrives at eth0
        │
        ▼
┌───────────────┐
│   XDP prog    │──── DROP (blocklist, rate limit)
└───────────────┘
        │ PASS
        ▼
┌───────────────┐
│   nftables    │──── Prerouting (DNAT for port forwards)
│   prerouting  │
└───────────────┘
        │
        ▼
┌───────────────┐
│   Routing     │──── Forward to LAN or local delivery
│   decision    │
└───────────────┘
        │
        ▼
┌───────────────┐
│   nftables    │──── Forward rules (stateful)
│   forward     │
└───────────────┘
        │
        ▼
┌───────────────┐
│ MTK Offload   │──── Fast-path for established flows
└───────────────┘
        │
        ▼
    eth1/wlan
```

### Egress (LAN → WAN)

```
Packet from LAN client
        │
        ▼
┌───────────────┐
│   Routing     │
│   decision    │
└───────────────┘
        │
        ▼
┌───────────────┐
│   nftables    │──── Forward rules, SNAT (masquerade)
│   forward     │
└───────────────┘
        │
        ▼
┌───────────────┐
│   TC-BPF      │──── QoS marking, egress filtering
│   (egress)    │
└───────────────┘
        │
        ▼
┌───────────────┐
│ MTK Offload   │──── Fast-path established
└───────────────┘
        │
        ▼
      eth0
```

## Services

### 1. beryl-routerd (Main Daemon)

**Responsibilities:**
- Load/manage XDP and TC-BPF programs
- Manage eBPF maps (blocklists, stats)
- Orchestrate mode switching
- Expose REST API for configuration
- Signal other services on config changes

**REST API:**
```
GET  /api/v1/status           # System status
GET  /api/v1/stats            # Packet/traffic stats
GET  /api/v1/config           # Current config
PUT  /api/v1/config           # Update config
POST /api/v1/mode             # Switch operating mode
GET  /api/v1/clients          # Connected clients
POST /api/v1/firewall/block   # Add to blocklist
```

### 2. dhcp4d (DHCP Server)

**Features:**
- IP pool management
- Static leases (MAC → IP binding)
- Lease persistence across restarts
- Hostname collection (for DNS)
- Option 6 (DNS server), Option 3 (gateway)

**Config:**
```toml
[dhcp.server]
interface = "br-lan"
pool_start = "192.168.1.100"
pool_end = "192.168.1.250"
lease_time = "12h"
gateway = "192.168.1.1"
dns = ["192.168.1.1"]

[[dhcp.static]]
mac = "aa:bb:cc:dd:ee:ff"
ip = "192.168.1.50"
hostname = "server"
```

### 3. dhcp4c (DHCP Client)

**Features:**
- Obtain WAN IP from upstream
- Handle lease renewal
- Trigger network reconfiguration on changes
- Support for vendor-specific options

### 4. dnsd (DNS Server)

**Features:**
- Forwarding resolver (upstream DNS)
- Local hostname resolution (from DHCP leases)
- Response caching
- Blocklist filtering (ads, trackers, malware)
- DNS-over-HTTPS/TLS upstream (optional)

**Config:**
```toml
[dns]
listen = "192.168.1.1:53"
upstream = ["1.1.1.1", "8.8.8.8"]
cache_size = 10000

[dns.blocking]
enabled = true
lists = [
  "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts"
]
```

### 5. wifid (WiFi Manager)

**Features:**
- Wrapper around hostapd
- Manages AP configuration
- Client mode for repeater
- Multiple SSIDs support
- WPA3 support

**Config:**
```toml
[wifi.radio0]  # 5GHz
channel = 36
bandwidth = 80

[[wifi.radio0.ssid]]
name = "MyNetwork-5G"
password = "secretpassword"
encryption = "wpa3"

[wifi.radio1]  # 2.4GHz
channel = 6
bandwidth = 20

[[wifi.radio1.ssid]]
name = "MyNetwork"
password = "secretpassword"
encryption = "wpa2"
```

### 6. Optional Services

| Service | Purpose | Implementation |
|---------|---------|----------------|
| ntpd | Time sync | Use chrony or custom |
| wireguard | VPN | Kernel module + userspace config |
| webui | Browser config | Separate SPA, talks to REST API |
| ddns | Dynamic DNS | Simple HTTP client |

## eBPF Programs

### XDP Program (Ingress)

```rust
// Maps
BLOCKLIST: HashMap<u32, u8>           // IP → drop flag
DNS_REDIRECT: HashMap<u16, u32>       // port → redirect IP
RATE_LIMIT: HashMap<u32, RateState>   // IP → rate state
STATS: PerCpuArray<Stats>             // Counters

// Actions
1. Check BLOCKLIST → DROP if matched
2. Check RATE_LIMIT → DROP if exceeded
3. Redirect DNS (port 53) to local dnsd
4. Update STATS
5. XDP_PASS to continue to netfilter
```

### TC-BPF Program (Egress)

```rust
// Maps
QOS_CLASS: HashMap<u32, u8>           // IP/port → QoS class
EGRESS_BLOCK: HashMap<u32, u8>        // Outbound blocklist

// Actions
1. Check EGRESS_BLOCK → DROP if matched
2. Set skb->priority based on QOS_CLASS
3. TC_ACT_OK to continue
```

## Configuration Files

```
/etc/beryl/
├── config.toml              # Main config (mode, interfaces)
├── dhcp/
│   ├── server.toml          # DHCP server config
│   └── static-leases.toml   # Static MAC→IP mappings
├── dns/
│   ├── server.toml          # DNS config
│   ├── local-records.toml   # Custom A/CNAME records
│   └── blocklists/          # Downloaded blocklists
├── firewall/
│   ├── rules.toml           # Firewall rules
│   ├── port-forwards.toml   # DNAT rules
│   └── blocklist.txt        # IP blocklist
├── wifi/
│   └── config.toml          # WiFi settings
└── wireguard/
    └── wg0.conf             # WireGuard config
```

### Main Config Example

```toml
[system]
hostname = "beryl"
timezone = "UTC"

[mode]
type = "router"  # router, ap, repeater, wireguard

[interfaces.wan]
name = "eth0"
type = "dhcp"  # dhcp, static, pppoe

[interfaces.lan]
name = "br-lan"
address = "192.168.1.1/24"
members = ["eth1", "wlan0", "wlan1"]

[firewall]
default_input = "drop"
default_forward = "drop"
default_output = "accept"
```

## Crate Structure

```
beryl-router/
├── Cargo.toml                    # Workspace
├── beryl-router-ebpf/            # eBPF programs (no_std)
│   └── src/
│       ├── main.rs               # XDP program
│       └── tc.rs                 # TC-BPF program
├── beryl-router-common/          # Shared types
│   └── src/lib.rs
├── crates/
│   ├── beryl-routerd/            # Main daemon
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api.rs            # REST API (axum)
│   │       ├── ebpf.rs           # eBPF loader/manager
│   │       ├── mode.rs           # Mode switching
│   │       └── nftables.rs       # nftables integration
│   ├── beryl-dhcp/               # DHCP client + server
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs
│   │       ├── server.rs
│   │       └── packet.rs
│   ├── beryl-dns/                # DNS server
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       ├── resolver.rs
│   │       ├── cache.rs
│   │       └── blocker.rs
│   └── beryl-wifi/               # WiFi manager
│       └── src/
│           ├── lib.rs
│           └── hostapd.rs
├── xtask/                        # Build system
└── tools/
    └── beryl-cli/                # CLI tool for config
```

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Async runtime | tokio | Standard, well-supported |
| HTTP framework | axum | Fast, ergonomic, tower ecosystem |
| eBPF loader | aya | Best Rust eBPF library |
| Serialization | serde + toml | Human-readable config |
| Logging | tracing | Structured, async-compatible |
| CLI | clap | Standard |
| WiFi | hostapd | Only option for MT7976 |
| Firewall | nftables (nft CLI) | Modern, supports flowtables |

## Build & Deploy

### Build (on Linux)

```bash
# Build eBPF
cargo xtask build-ebpf --release

# Build all userspace binaries for router
cargo xtask build --release --target aarch64-unknown-linux-musl
```

### Deploy

```bash
# Create release tarball
tar -czvf beryl-router.tar.gz \
  target/aarch64-unknown-linux-musl/release/beryl-routerd \
  target/aarch64-unknown-linux-musl/release/beryl-cli \
  etc/

# Deploy to router
scp beryl-router.tar.gz root@192.168.8.1:/tmp/
ssh root@192.168.8.1 "cd /tmp && tar xzf beryl-router.tar.gz && ./install.sh"
```

## Development Phases

### Phase 1: Core Infrastructure
- [ ] Main daemon with eBPF loader
- [ ] XDP firewall (blocklist, stats)
- [ ] REST API skeleton
- [ ] Mode switching framework
- [ ] nftables integration (NAT)

### Phase 2: Network Services
- [ ] DHCP server
- [ ] DHCP client
- [ ] DNS server with caching
- [ ] DNS blocklist filtering

### Phase 3: WiFi & Advanced
- [ ] WiFi manager (hostapd wrapper)
- [ ] Multiple operating modes
- [ ] WireGuard integration
- [ ] QoS / traffic shaping

### Phase 4: Polish
- [ ] Web UI
- [ ] CLI tool
- [ ] Documentation
- [ ] OpenWrt image builder integration

## Open Questions

1. **Single binary vs multiple?**
   - Option A: Single `beryl-routerd` with all services (simpler deploy)
   - Option B: Separate binaries per service (router7 style, process isolation)
   - **Recommendation:** Single binary with feature flags, can split later

2. **Config format?**
   - TOML (readable), JSON (web-friendly), or both?
   - **Recommendation:** TOML for files, JSON for API

3. **Web UI technology?**
   - Embedded (Rust templates) or separate SPA (React/Svelte)?
   - **Recommendation:** Separate SPA, served as static files

4. **IPv6 support?**
   - Full dual-stack from start, or IPv4 first?
   - **Recommendation:** IPv4 first, design for IPv6

5. **Use existing crates or build from scratch?**
   - DHCP: `dhcproto` crate exists
   - DNS: `trust-dns` / `hickory-dns` are mature
   - **Recommendation:** Use existing crates where mature
