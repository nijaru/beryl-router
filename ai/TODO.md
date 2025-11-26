# TODO

## Phase 1: Core Infrastructure

- [ ] Restructure workspace (see ai/design/SYSTEM.md)
- [ ] Main daemon skeleton (beryl-routerd)
- [ ] REST API with axum
- [ ] eBPF loader/manager
- [ ] nftables integration (NAT, port forwards)
- [ ] Mode switching framework
- [ ] TC-BPF program for egress

## Phase 2: Network Services

- [ ] DHCP server (use dhcproto crate)
- [ ] DHCP client
- [ ] DNS server (evaluate hickory-dns vs custom)
- [ ] DNS blocklist filtering
- [ ] Local hostname resolution

## Phase 3: WiFi & Advanced

- [ ] WiFi manager (hostapd wrapper)
- [ ] AP mode implementation
- [ ] Repeater mode implementation
- [ ] WireGuard integration

## Phase 4: Polish

- [ ] CLI tool (beryl-cli)
- [ ] Web UI (separate SPA)
- [ ] OpenWrt image with BTF
- [ ] Documentation

## Build System

- [ ] Install `bpf-linker`: `cargo install bpf-linker`
- [ ] Verify eBPF build on Fedora
- [ ] Cross-compilation for aarch64
