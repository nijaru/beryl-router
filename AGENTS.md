# beryl-router

XDP/eBPF firewall and full router stack for GL.iNet Beryl AX (GL-MT3000) travel router. Rust userspace + eBPF kernel programs.

## Project Structure

| Directory | Purpose |
|-----------|---------|
| beryl-router/ | Userspace control plane daemon |
| beryl-router-ebpf/ | eBPF kernel programs (XDP, TC-BPF) |
| beryl-router-common/ | Shared types between kernel/userspace |
| xtask/ | Build system for eBPF + cross-compilation |
| ai/ | **AI session context** - state tracking across sessions |

### AI Context Organization

**Session files** (read every session, <500 lines each):
- ai/STATUS.md — Current state, blockers (read FIRST)
- ai/TODO.md — Active tasks by phase
- ai/DECISIONS.md — Architecture decisions with rationale
- ai/RESEARCH.md — Research index
- ai/KNOWLEDGE.md — Hardware quirks, eBPF gotchas

**Reference files** (loaded on demand):
- ai/design/ — System architecture, flashing guide
- ai/research/ — Detailed research
- ai/decisions/ — Archived decisions
- ai/tmp/ — Temporary artifacts (gitignored)

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (nightly, edition 2024) |
| eBPF loader | aya 0.13 |
| Async runtime | tokio |
| HTTP server | axum (planned) |
| DNS server | hickory-dns (planned) |
| DHCP | dhcproto (planned) |
| Serialization | serde + toml |
| Target OS | OpenWrt (custom build with BTF) |
| Target HW | GL-MT3000 (MediaTek MT7981, ARM64) |

## Commands

```bash
# Build eBPF program (Linux only)
cargo xtask build-ebpf --release

# Build userspace for router (cross-compile)
cargo xtask build --release --target aarch64-unknown-linux-musl

# Check workspace (won't fully compile on macOS)
cargo check

# Deploy to router
scp target/aarch64-unknown-linux-musl/release/beryl-routerd root@192.168.8.1:/tmp/
```

## Verification Steps

| Check | Command | Notes |
|-------|---------|-------|
| Workspace check | `cargo check` | Fails on macOS (aya needs Linux) |
| eBPF build | `cargo xtask build-ebpf` | Linux only, needs bpf-linker |
| Cross-compile | `cargo xtask build --target aarch64-unknown-linux-musl` | Needs aarch64 linker |

## Code Standards

| Aspect | Standard |
|--------|----------|
| Edition | Rust 2024 |
| Error handling | anyhow (app), thiserror (lib) |
| Async | tokio for network, sync for files |
| eBPF | no_std, aya-ebpf macros |
| Config format | TOML files, JSON API |
| Naming | snake_case, descriptive |

## Architecture Summary

```
┌─────────────────────────────────────────────┐
│  beryl-routerd (REST API, eBPF manager)    │
├─────────────────────────────────────────────┤
│  dhcp │ dns │ wifi (hostapd) │ wireguard   │
├─────────────────────────────────────────────┤
│  XDP (ingress) │ TC-BPF │ nftables (NAT)   │
├─────────────────────────────────────────────┤
│  Linux + MTK Flow Offload                   │
├─────────────────────────────────────────────┤
│  eth0 (WAN) │ eth1 (LAN) │ wlan0/1 (WiFi) │
└─────────────────────────────────────────────┘
```

**Key design:** XDP for fast-path filtering, nftables for NAT/conntrack, coexist with MTK hardware offload.

## Hardware Notes

| Item | Value |
|------|-------|
| SoC | MediaTek MT7981 (Filogic 820) |
| CPU | Dual ARM Cortex-A53 @ 1.3GHz |
| RAM | 512MB DDR4 |
| Flash | 512MB NAND |
| WiFi | MT7976 (requires proprietary drivers, OpenWrt only) |
| Recovery | U-Boot at 192.168.1.1 (hold reset 10s) |

## Development Environment

| Machine | Role |
|---------|------|
| nick@fedora | Build host (Linux required for aya) |
| nick@apple | SSH/testing, cannot build eBPF |

## Current Focus

See ai/STATUS.md for current state, ai/TODO.md for task list.

Full architecture: ai/design/SYSTEM.md
Flashing guide: ai/design/FLASHING.md
