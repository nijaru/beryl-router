# TODO

## Phase 1: Core Infrastructure

### 1.1 Workspace Restructure
- [x] Create crates/ directory structure [ai/design/CRATES.md]
- [x] Move beryl-router-common → crates/beryl-common
- [x] Create crates/beryl-config (config loading)
- [x] Create crates/beryl-ebpf (eBPF loader)
- [x] Create crates/beryl-nft (nftables integration)
- [x] Move beryl-router → src/ (main binary)
- [x] Update workspace Cargo.toml
- [x] Update xtask paths
- [x] Verify `cargo check` passes

### 1.2 Configuration System
- [x] Define Config structs [ai/design/CONFIG.md]
- [x] TOML loading with serde
- [x] Config validation
- [x] Hot-reload via notify crate
- [x] Create example config.toml

### 1.3 Main Daemon
- [x] CLI parsing (clap): --config, --interface
- [x] Daemon initialization sequence
- [x] Signal handling (SIGTERM, SIGHUP for reload)
- [x] Graceful shutdown

### 1.4 eBPF Manager
- [x] Load XDP program
- [x] Load TC-BPF program (egress)
- [x] Map management (blocklist, stats)
- [x] aya-log integration
- [x] Attach to configurable interfaces

### 1.5 TC-BPF Program
- [x] Create tc_egress.rs in beryl-router-ebpf
- [x] Egress filtering logic
- [x] QoS marking (future)
- [x] Update xtask to build TC program

### 1.6 nftables Integration
- [x] NftManager struct [ai/design/NFTABLES.md]
- [x] Generate ruleset from config
- [x] Apply rules via `nft` command
- [x] NAT (masquerade) setup
- [x] Port forwarding (DNAT)
- [x] Flowtable offload enable

### 1.7 REST API
- [x] axum router setup [ai/design/API.md]
- [x] GET /api/v1/status
- [x] GET /api/v1/stats (from eBPF maps)
- [x] GET/PUT /api/v1/config
- [x] POST /api/v1/firewall/blocklist
- [x] Shared state (Arc<RwLock<...>>)

### 1.8 Mode Switching
- [x] OperatingMode enum
- [x] Router mode setup (NAT, DHCP prep)
- [x] AP mode setup (bridge, no NAT)
- [x] Mode transition logic
- [x] POST /api/v1/mode endpoint

### 1.9 Build & Test
- [x] Verify eBPF build on Fedora
- [x] Cross-compile for aarch64
- [x] Test on x86 VM (veth pairs)
- [x] Create systemd/procd service file

## Phase 2: Network Services

### 2.1 DHCP Server
- [x] Add crates/beryl-dhcp
- [x] Use dhcproto for packet parsing
- [x] IP pool management
- [x] Lease storage (persist to file)
- [x] Static lease support
- [x] Integrate with DNS (hostname → IP)

### 2.2 DHCP Client
- [ ] WAN interface DHCP client
- [ ] Lease renewal handling
- [ ] Trigger network reconfiguration on change
- [ ] Update default route, DNS

### 2.3 DNS Server
- [x] Add crates/beryl-dns
- [x] Evaluate hickory-dns vs custom
- [x] Forwarding resolver
- [x] Response caching (Basic via Hickory)
- [x] Local hostname resolution (from DHCP)
- [ ] Blocklist filtering
- [ ] Blocklist auto-update

## Phase 3: WiFi & Advanced

### 3.1 WiFi Manager
- [x] Add crates/beryl-wifi
- [x] hostapd config generation (OpenWrt UCI)
- [x] hostapd process management (via wifi reload)
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
