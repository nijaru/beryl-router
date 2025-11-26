# TODO

## Phase 1: Core Infrastructure

### 1.1 Workspace Restructure
- [ ] Create crates/ directory structure [ai/design/CRATES.md]
- [ ] Move beryl-router-common → crates/beryl-common
- [ ] Create crates/beryl-config (config loading)
- [ ] Create crates/beryl-ebpf (eBPF loader)
- [ ] Create crates/beryl-nft (nftables integration)
- [ ] Move beryl-router → src/ (main binary)
- [ ] Update workspace Cargo.toml
- [ ] Update xtask paths
- [ ] Verify `cargo check` passes

### 1.2 Configuration System
- [ ] Define Config structs [ai/design/CONFIG.md]
- [ ] TOML loading with serde
- [ ] Config validation
- [ ] Hot-reload via notify crate
- [ ] Create example config.toml

### 1.3 Main Daemon
- [ ] CLI parsing (clap): --config, --interface
- [ ] Daemon initialization sequence
- [ ] Signal handling (SIGTERM, SIGHUP for reload)
- [ ] Graceful shutdown

### 1.4 eBPF Manager
- [ ] Load XDP program
- [ ] Load TC-BPF program (egress)
- [ ] Map management (blocklist, stats)
- [ ] aya-log integration
- [ ] Attach to configurable interfaces

### 1.5 TC-BPF Program
- [ ] Create tc_egress.rs in beryl-router-ebpf
- [ ] Egress filtering logic
- [ ] QoS marking (future)
- [ ] Update xtask to build TC program

### 1.6 nftables Integration
- [ ] NftManager struct [ai/design/NFTABLES.md]
- [ ] Generate ruleset from config
- [ ] Apply rules via `nft` command
- [ ] NAT (masquerade) setup
- [ ] Port forwarding (DNAT)
- [ ] Flowtable offload enable

### 1.7 REST API
- [ ] axum router setup [ai/design/API.md]
- [ ] GET /api/v1/status
- [ ] GET /api/v1/stats (from eBPF maps)
- [ ] GET/PUT /api/v1/config
- [ ] POST /api/v1/firewall/blocklist
- [ ] Shared state (Arc<RwLock<...>>)

### 1.8 Mode Switching
- [ ] OperatingMode enum
- [ ] Router mode setup (NAT, DHCP prep)
- [ ] AP mode setup (bridge, no NAT)
- [ ] Mode transition logic
- [ ] POST /api/v1/mode endpoint

### 1.9 Build & Test
- [ ] Verify eBPF build on Fedora
- [ ] Cross-compile for aarch64
- [ ] Test on x86 VM (veth pairs)
- [ ] Create systemd/procd service file

## Phase 2: Network Services

### 2.1 DHCP Server
- [ ] Add crates/beryl-dhcp
- [ ] Use dhcproto for packet parsing
- [ ] IP pool management
- [ ] Lease storage (persist to file)
- [ ] Static lease support
- [ ] Integrate with DNS (hostname → IP)

### 2.2 DHCP Client
- [ ] WAN interface DHCP client
- [ ] Lease renewal handling
- [ ] Trigger network reconfiguration on change
- [ ] Update default route, DNS

### 2.3 DNS Server
- [ ] Add crates/beryl-dns
- [ ] Evaluate hickory-dns vs custom
- [ ] Forwarding resolver
- [ ] Response caching
- [ ] Local hostname resolution (from DHCP)
- [ ] Blocklist filtering
- [ ] Blocklist auto-update

## Phase 3: WiFi & Advanced

### 3.1 WiFi Manager
- [ ] Add crates/beryl-wifi
- [ ] hostapd config generation
- [ ] hostapd process management
- [ ] Multiple SSID support
- [ ] Client list from hostapd_cli

### 3.2 Operating Modes
- [ ] Repeater mode (wlan client + AP)
- [ ] WireGuard mode
- [ ] Guest network (VLAN)

### 3.3 Advanced Features
- [ ] QoS / traffic shaping
- [ ] UPnP/NAT-PMP
- [ ] DDNS client

## Phase 4: Polish

### 4.1 CLI Tool
- [ ] Create tools/beryl-cli
- [ ] Status commands
- [ ] Config commands
- [ ] Client list

### 4.2 Web UI
- [ ] Create separate SPA repo (Solid or Svelte)
- [ ] Dashboard
- [ ] Network config
- [ ] Firewall rules
- [ ] Client management
- [ ] Serve static files from beryl-routerd

### 4.3 Documentation
- [ ] User guide
- [ ] API documentation
- [ ] OpenWrt build guide

### 4.4 OpenWrt Integration
- [ ] Custom OpenWrt image with BTF
- [ ] Package (ipk) creation
- [ ] procd init script
- [ ] UCI integration (optional)

## Build System

- [ ] Install bpf-linker on Fedora
- [ ] Install aarch64 cross-compiler
- [ ] Verify full build pipeline
- [ ] CI setup (GitHub Actions)
