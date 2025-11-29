# Status

## Handover Context (macOS → Linux)

**Last Session Date:** 2025-11-29
**Last Machine:** macOS (nick@apple)
**Next Step:** Build the custom OpenWrt image on Fedora.

### Instructions for `nick@fedora`

1.  **Pull changes:** `git pull`
2.  **Setup Buildroot:** Clone OpenWrt (or GL.iNet SDK) for MT7981.
3.  **Configure Kernel:** Run `make kernel_menuconfig` and enable:
    - `CONFIG_DEBUG_INFO_BTF=y`
    - `CONFIG_BPF_SYSCALL=y`
4.  **Build:** `make -j$(nproc)`
5.  **Flash:** Flash the resulting image to the GL-MT3000.
6.  **Deploy:** `scp` the `beryl-routerd` binary (built via `cargo xtask`) to the router and test.

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
| XDP firewall | Code complete | Ingress filtering (Blocklist/Ports) |
| TC-BPF egress | Code complete | Egress filtering (Phase 1.5) |
| REST API | Code complete | Status, Stats, Config endpoints (Phase 1.7) |
| DHCP server/client | Code complete | Server logic, persistence, integration |
| DNS server | Code complete | Forwarding + Local Hostname resolution |
| Local DNS | Code complete | Resolves names from DHCP leases |
| WiFi manager | Code complete | Generates OpenWrt config + reload |
| DHCP Client | Complete | Integrated with netlink/ip commands |
| OpenWrt image | Complete | Built with BTF. Artifact ready. |

## Blockers

| Blocker | Impact | Resolution |
|---------|--------|------------|
| Testing Hardware | Cannot verify eBPF | Flash custom image to router |

## Recent Commits

- build: Compiled OpenWrt 23.05 image with CONFIG_DEBUG_INFO_BTF=y
- feat: Integrated DHCP Client with system actuator
- infra: Setup Docker/Podman build environment

## Artifacts

- **Firmware Image:** `build/openwrt/bin/targets/mediatek/filogic/openwrt-mediatek-filogic-glinet_gl-mt3000-squashfs-sysupgrade.bin`
- **Flash Method:**
    1. **Web UI:** `http://192.168.8.1` -> Upgrade -> Local Upgrade (Uncheck 'Keep Settings').
    2. **U-Boot:** Hold Reset 10s on boot -> `http://192.168.1.1` (PC IP: 192.168.1.2).

## Active Work
Next: Cross-compile `beryl-routerd` and deploy.

## User Context

- Home network: Xfinity → Asus AP → devices
- Beryl will sit behind existing network (double NAT for testing)
- Primary use: travel router, may run without WAN connection
- Build on Fedora (nick@fedora) OR Docker on Mac (nick@apple)

## Learnings

- XDP is ingress-only; need TC-BPF for egress filtering
- MediaTek HW flow offload should coexist with XDP (don't replace NAT)
- Use nftables for NAT/conntrack, XDP for fast-path filtering
- WiFi requires hostapd + proprietary MT7976 drivers (OpenWrt only)
- U-Boot recovery (hold reset 10s) is safety net for bricking
- Nightly Rust is required for `beryl-router-ebpf` (build-std).
- `bpf-linker` requires specific rustc version matching the linker; updated Dockerfile to use nightly default.
- Docker on Apple Silicon with `rust:nightly` is effective for building aarch64 binaries.
- OpenWrt WiFi control is best done via generating `/etc/config/wireless` and calling `/sbin/wifi reload` rather than fighting netifd.

## Active Work
Next Phase: Building the OpenWrt Image (Phase 4).