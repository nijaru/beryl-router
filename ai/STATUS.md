# Status

## Current State

| Aspect | Status | Notes |
|--------|--------|-------|
| System design | Complete | See ai/design/SYSTEM.md |
| Project scaffold | Complete | Workspace with 3 crates |
| XDP firewall | Code complete | Needs build verification |
| TC-BPF egress | Not started | Phase 1 |
| REST API | Not started | Phase 1 |
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
