# Research

Index of research topics. Details in ai/research/.

## Completed

| Topic | Finding | Details |
|-------|---------|---------|
| XDP/eBPF architecture | aya + nftables hybrid | ai/design/SYSTEM.md |
| Flashing methods | U-Boot recovery safest | ai/design/FLASHING.md |
| router7 comparison | Different goals, XDP faster for filtering | Session notes |
| nftables integration | Shell out to `nft` for Phase 1 | ai/design/NFTABLES.md |
| Config format | TOML files, JSON API | ai/design/CONFIG.md |
| REST API design | axum, /api/v1/* endpoints | ai/design/API.md |
| Crate structure | Single binary, internal crates | ai/design/CRATES.md |

## In Progress

| Topic | Status | Notes |
|-------|--------|-------|
| hickory-dns | Not started | Evaluate vs custom DNS (Phase 2) |
| dhcproto | Not started | DHCP packet parsing (Phase 2) |

## To Research

| Topic | Priority | Why | When |
|-------|----------|-----|------|
| OpenWrt BTF build | High | Blocking hardware testing | Before Phase 1 test |
| bpf-linker setup | High | Required for eBPF build | Phase 1 start |
| TC-BPF attachment | Medium | Egress filtering | Phase 1.5 |
| MT7976 hostapd | Low | WiFi manager | Phase 3 |
| nftables-rs crate | Low | Better than shell out | Future |
| WireGuard kernel module | Low | VPN mode | Phase 3 |
