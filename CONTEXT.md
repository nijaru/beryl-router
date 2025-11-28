# Project Context: beryl-router

**Goal:** High-performance, Rust-based router firmware for the GL.iNet Beryl AX (GL-MT3000) travel router. Replaces shell scripts and `iptables` with a unified Rust control plane and eBPF data plane.

## Architecture

The system consists of a userspace control daemon (`beryl-routerd`) and kernel-space eBPF programs.

```
┌──────────────────────────────────────────────────────────────────┐
│  beryl-routerd (Userspace)                                      │
│  • REST API (Axum, port 8080)                                   │
│  • Config Management (TOML/JSON)                                │
│  • eBPF Loader (Aya)                                            │
│  • Mode Manager (Router/AP/Repeater)                            │
└──────────────────────────────────────────────────────────────────┘
          │                       │
          ▼                       ▼
┌─────────────────────┐   ┌──────────────────────┐
│ XDP Program (Ingress)│   │ TC Program (Egress)  │
│ • Blocklist (IP/Port)│   │ • Egress Filtering   │
│ • Rate Limiting      │   │ • QoS Marking        │
└─────────────────────┘   └──────────────────────┘
          │                       │
          ▼                       ▼
┌──────────────────────────────────────────────────────────────────┐
│  Linux Kernel / Hardware (MT7981)                               │
│  • nftables (NAT/Conntrack)                                     │
│  • MTK Flow Offload (Hardware Fast-Path)                        │
└──────────────────────────────────────────────────────────────────┘
```

## Tech Stack

- **Language:** Rust (2024 Edition)
- **Userspace:**
    - **Runtime:** `tokio`
    - **API:** `axum` + `tower-http`
    - **Config:** `serde` + `toml`
    - **Logging:** `tracing`
- **Kernel (eBPF):**
    - **Loader:** `aya`
    - **Programs:** `aya-ebpf` (no_std)
    - **Requirement:** Rust Nightly (for `-Z build-std`)

## Workspace Structure

- `beryl-routerd` (root `src/`): Main control plane binary.
- `beryl-router-ebpf`: Kernel programs (XDP/TC).
- `crates/beryl-common`: Shared types (config, stats) between kernel/user.
- `crates/beryl-config`: Configuration parsing logic.
- `crates/beryl-ebpf`: Logic to load/attach eBPF programs.
- `crates/beryl-nft`: `nftables` integration (NAT).

## Current Status (Phase 1 Complete)

- **Implemented:**
    - Multi-crate workspace structure.
    - XDP Ingress firewall (Blocklist IPs/Ports).
    - TC Egress firewall (Blocklist IPs).
    - REST API (`/status`, `/stats`, `/config`).
    - `xtask` build system for cross-compilation.

- **Pending (Phase 2):**
    - DHCP Server/Client implementation.
    - DNS Resolver implementation.

## Handover Context (macOS → Linux)

**Last Session Date:** 2025-11-28
**Last Machine:** macOS (nick@apple)
**Next Machine:** Linux (nick@fedora)

### Immediate Actions Required

1.  **Sync Code:**
    ```bash
    git pull
    ```
2.  **Verify Build (Linux):**
    The eBPF build was skipped on macOS due to missing kernel headers. You must verify it compiles on Linux.
    ```bash
    cargo xtask build-ebpf
    cargo check --workspace
    ```
3.  **Resume Work:**
    We are starting **Phase 2: DHCP Server/Client**.
    - Task ID: `beryl-router-j82` (in beads)
    - Goal: Implement `dhcproto` based server/client in `crates/beryl-dhcp`.
    - Reference: `ai/design/SYSTEM.md` (Service #2 and #3).

### Active Task State
- **Restructure:** Complete & Merged.
- **TC-BPF:** Complete & Merged.
- **API:** Complete & Merged.
- **Next Task:** `beryl-router-j82` (Phase 2 DHCP) is currently **OPEN**.

## Development Constraints

1.  **eBPF Build:** Requires **Linux** (for `aya` linking) and **Rust Nightly**.
    - Command: `cargo xtask build-ebpf`
2.  **Userspace Build:** Can use **Rust Stable**.
    - Command: `cargo build --package beryl-routerd`
3.  **Cross-Compilation:** Target `aarch64-unknown-linux-musl` for the device.

## Key Documentation

- `AGENTS.md`: Tool usage and operational guidelines.
- `ai/STATUS.md`: Current task status and blockers.
- `ai/design/SYSTEM.md`: Detailed system architecture.
- `ai/design/API.md`: REST API specification.