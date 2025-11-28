# Project Context: beryl-router

**Goal:** High-performance, Rust-based router firmware for the GL.iNet Beryl AX (GL-MT3000) travel router. Replaces shell scripts and `iptables` with a unified Rust control plane and eBPF data plane.

## Architecture

The system consists of a userspace control daemon (`beryl-routerd`) and kernel-space eBPF programs.

```
┌──────────────────────────────────────────────────────────────────┐
│  beryl-routerd (Userspace)                                      │
│  • REST API (Axum, port 8080)                                   │
│  • Config Management (TOML)                                     │
│  • eBPF Loader (Aya)                                            │
│  • Services: DHCP Server, DNS Server                            │
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
    - **DNS:** `hickory-dns` (planned)
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
- `crates/beryl-ebpf`: Logic to load/attach eBPF programs.
- `crates/beryl-nft`: `nftables` integration (NAT).

## Current Status (Phase 2 In Progress)

- **Implemented:**
    - Multi-crate workspace structure.
    - XDP Ingress firewall (Blocklist IPs/Ports).
    - TC Egress firewall (Blocklist IPs).
    - REST API (`/status`, `/stats`, `/config`).
    - **DHCP Server:**
        - Packet parsing with `dhcproto`.
        - Lease management (dynamic + static).
        - Lease persistence (JSON file).
    - **Config:** TOML based configuration system.
    - **Build System:**
        - Linux: `cargo xtask build-ebpf` works.
        - macOS (Apple Silicon): Docker/OrbStack environment (`docker-compose.yml`).

- **Pending (Phase 2):**
    - DNS Resolver implementation (`beryl-dns`).
    - Integrate DHCP hostnames into DNS.

## Handover Context (Linux → macOS/Linux)

**Last Session Date:** 2025-11-28
**Last Machine:** Linux (nick@fedora)
**Next Step:** Continue DNS implementation.

### Development Workflow

**Option A: Linux (Native)**
```bash
cargo xtask build-ebpf
cargo build --workspace
```

**Option B: macOS (Docker/OrbStack)**
1.  Start container: `docker compose up -d`
2.  Enter shell: `docker compose exec dev bash`
3.  Build: `cargo xtask build --release --target aarch64-unknown-linux-musl`

### Active Task State
- **Phase 1 (Core):** Complete.
- **Phase 2 (DHCP):** Server logic & persistence complete.
- **Phase 2 (DNS):** Skeleton created.
- **Next Task:** Implement DNS server logic in `crates/beryl-dns` using `hickory-server`.

## Key Documentation

- `AGENTS.md`: Tool usage and operational guidelines.
- `ai/STATUS.md`: Current task status and blockers.
- `ai/design/SYSTEM.md`: Detailed system architecture.
- `ai/design/CONFIG.md`: TOML config schema.
