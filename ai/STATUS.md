# Status

## Current State

| Aspect | Status | Notes |
|--------|--------|-------|
| System design | Complete | ai/design/SYSTEM.md |
| Crate restructure plan | Complete | ai/design/CRATES.md |
| API design | Complete | ai/design/API.md |
| Config schema | Complete | ai/design/CONFIG.md |
| nftables design | Complete | ai/design/NFTABLES.md |
| Flashing guide | Complete | ai/design/FLASHING.md |
| Project scaffold | Complete | Multi-crate structure implemented |
| XDP firewall | Code complete | Needs build verification |
| TC-BPF egress | Not started | Phase 1.5 |
| REST API | Not started | Phase 1.7 |
| DHCP server/client | Not started | Phase 2 |
| DNS server | Not started | Phase 2 |
| WiFi manager | Not started | Phase 3 |
| OpenWrt image | Not started | Requires custom build with BTF |

## Blockers

| Blocker | Impact | Resolution |
|---------|--------|------------|
| aya requires Linux | Cannot build on macOS | Build on Fedora (nick@fedora) or Docker |
| No BTF-enabled OpenWrt | Cannot test on device | Build custom image |
| Cross-compiler needed | Cannot build for aarch64 | Install `aarch64-linux-gnu-gcc` |

## Recent Commits

- refactor: Restructure workspace per ai/design/CRATES.md
- chore: Initialize beads task tracking
- System design document for full router stack
- Initial project scaffold with XDP/eBPF architecture

## User Context

- Home network: Xfinity → Asus AP → devices
- Beryl will sit behind existing network (double NAT for testing)
- Primary use: travel router, may run without WAN connection
- Build on Fedora (nick@fedora), SSH from Mac (nick@apple)

## Learnings

- XDP is ingress-only; need TC-BPF for egress filtering
- MediaTek HW flow offload should coexist with XDP (don't replace NAT)
- Use nftables for NAT/conntrack, XDP for fast-path filtering
- WiFi requires hostapd + proprietary MT7976 drivers (OpenWrt only)
- U-Boot recovery (hold reset 10s) is safety net for bricking

## Active Work
TC-BPF egress filtering (Phase 1.5).
