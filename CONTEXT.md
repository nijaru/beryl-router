# Project Context: beryl-router

**Goal:** High-performance, Rust-based router firmware for the GL.iNet Beryl AX (GL-MT3000) travel router. Replaces shell scripts and `iptables` with a unified Rust control plane and eBPF data plane.

## Architecture

The system consists of a userspace control daemon (`beryl-routerd`) and kernel-space eBPF programs.

```
┌──────────────────────────────────────────────────────────────────┐
│  beryl-routerd (Userspace)                                      │
│  • REST API (Axum, port 8080)                                   │
│  • Config Management (TOML, notify)                             │
│  • eBPF Loader (Aya)                                            │
│  • DHCP Server (dhcproto, shared Lease DB)                      │
│  • DNS Server (hickory-dns, forwarding + local resolution)      │
│  • WiFi Manager (Generates /etc/config/wireless + reload)       │
└──────────────────────────────────────────────────────────────────┘
          │                       │
          ▼                       ▼
┌─────────────────────┐   ┌──────────────────────┐
│ XDP Program (Ingress)│   │ TC Program (Egress)  │
│ • Blocklist (IP/Port)│   │ • Egress Filtering   │
│ • Rate Limiting      │   │ • QoS Marking        │
└─────────────────────┘   └──────────────────────┘
```

## Tech Stack

- **Language:** Rust (2024 Edition)
- **Userspace:**
    - **Runtime:** `tokio`
    - **API:** `axum`
    - **Config:** `serde` + `toml`
    - **DHCP:** `dhcproto` + `socket2`
    - **DNS:** `hickory-dns`
    - **WiFi:** OpenWrt UCI generation (`/etc/config/wireless`)
- **Kernel (eBPF):**
    - **Loader:** `aya`
    - **Programs:** `aya-ebpf` (no_std)

## Workspace Structure

- `beryl-routerd` (root `src/`): Main control plane binary.
- `beryl-router-ebpf`: Kernel programs (XDP/TC).
- `crates/beryl-common`: Shared types (config, stats).
- `crates/beryl-config`: Configuration parsing logic (TOML).
- `crates/beryl-dhcp`: DHCP server/client implementation.
- `crates/beryl-dns`: DNS resolver implementation.
- `crates/beryl-wifi`: WiFi configuration management.
- `crates/beryl-ebpf`: Logic to load/attach eBPF programs.
- `crates/beryl-nft`: `nftables` integration (NAT).

## Current Status (End of Phase 3)

- **Complete:**
    - Multi-crate workspace structure.
    - XDP Ingress firewall & TC Egress firewall.
    - REST API (`/status`, `/stats`, `/config`).
    - **DHCP Server:** Full implementation with persistence.
    - **DNS Server:** Forwarding + Local hostname resolution (integrated with DHCP).
    - **WiFi Manager:** UCI config generation and system reload integration.
    - **Dev Environment:** Docker-based build for macOS (aarch64).

- **Pending (Phase 4):**
    - **Build OpenWrt Image:** Requires building a custom OpenWrt image with `CONFIG_DEBUG_INFO_BTF=y` on Linux.

## Handover Context (macOS → Linux)

**Last Session Date:** 2025-11-28
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

### Key Documentation

- `AGENTS.md`: Tool usage and operational guidelines.
- `ai/STATUS.md`: Current task status and blockers.
- `ai/design/WIFI.md`: WiFi architecture decisions.
