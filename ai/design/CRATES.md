# Crate Restructure Plan

Current → Target workspace structure for single-binary architecture.

## Current Structure

```
beryl-router/
├── Cargo.toml              # Workspace root
├── beryl-router/           # Userspace daemon (basic)
├── beryl-router-common/    # Shared types
├── beryl-router-ebpf/      # eBPF programs (separate, not workspace member)
└── xtask/                  # Build system
```

## Target Structure

```
beryl-router/
├── Cargo.toml              # Workspace root
├── beryl-router-ebpf/      # eBPF programs (unchanged, separate)
│   └── src/
│       ├── main.rs         # XDP ingress program
│       └── tc_egress.rs    # TC-BPF egress program
├── crates/
│   ├── beryl-common/       # Shared types (rename)
│   │   └── src/lib.rs
│   ├── beryl-config/       # Configuration loading/validation
│   │   └── src/lib.rs
│   ├── beryl-ebpf/         # eBPF loader/manager
│   │   └── src/lib.rs
│   ├── beryl-nft/          # nftables integration
│   │   └── src/lib.rs
│   ├── beryl-dhcp/         # DHCP client + server
│   │   └── src/lib.rs
│   ├── beryl-dns/          # DNS server
│   │   └── src/lib.rs
│   └── beryl-wifi/         # WiFi/hostapd manager
│       └── src/lib.rs
├── src/
│   ├── main.rs             # Entry point, CLI parsing
│   ├── daemon.rs           # Main daemon orchestration
│   ├── api.rs              # REST API (axum)
│   └── mode.rs             # Operating mode management
├── xtask/                  # Build system (unchanged)
└── tools/
    └── beryl-cli/          # Optional CLI tool (later)
```

## Migration Steps

### Step 1: Create crates/ directory

```bash
mkdir -p crates/{beryl-common,beryl-config,beryl-ebpf,beryl-nft}
```

### Step 2: Move and rename beryl-router-common

```bash
mv beryl-router-common/* crates/beryl-common/
rm -rf beryl-router-common
```

Update `crates/beryl-common/Cargo.toml`:
```toml
[package]
name = "beryl-common"
version.workspace = true
edition.workspace = true
```

### Step 3: Create beryl-config crate

`crates/beryl-config/Cargo.toml`:
```toml
[package]
name = "beryl-config"
version.workspace = true
edition.workspace = true

[dependencies]
beryl-common = { path = "../beryl-common" }
serde.workspace = true
toml = "0.8"
thiserror = "1"
```

### Step 4: Create beryl-ebpf crate (loader, not programs)

`crates/beryl-ebpf/Cargo.toml`:
```toml
[package]
name = "beryl-ebpf"
version.workspace = true
edition.workspace = true

[dependencies]
beryl-common = { path = "../beryl-common" }
aya.workspace = true
aya-log.workspace = true
anyhow.workspace = true
tracing.workspace = true
```

### Step 5: Create beryl-nft crate

`crates/beryl-nft/Cargo.toml`:
```toml
[package]
name = "beryl-nft"
version.workspace = true
edition.workspace = true

[dependencies]
anyhow.workspace = true
tracing.workspace = true
tokio = { workspace = true, features = ["process"] }
```

### Step 6: Convert beryl-router to main binary

Move `beryl-router/` contents to `src/`:
```bash
mv beryl-router/src/main.rs src/main.rs
rm -rf beryl-router
```

Update root `Cargo.toml`:
```toml
[package]
name = "beryl-routerd"
version.workspace = true
edition.workspace = true

[[bin]]
name = "beryl-routerd"
path = "src/main.rs"

[dependencies]
beryl-common = { path = "crates/beryl-common" }
beryl-config = { path = "crates/beryl-config" }
beryl-ebpf = { path = "crates/beryl-ebpf" }
beryl-nft = { path = "crates/beryl-nft" }
# ... workspace deps
```

### Step 7: Update workspace members

```toml
[workspace]
resolver = "2"
members = [
    ".",
    "crates/beryl-common",
    "crates/beryl-config",
    "crates/beryl-ebpf",
    "crates/beryl-nft",
    "xtask",
]
```

### Step 8: Update eBPF crate path

In `beryl-router-ebpf/Cargo.toml`:
```toml
beryl-common = { path = "../crates/beryl-common", features = ["ebpf"] }
```

### Step 9: Update xtask

Update paths in `xtask/src/main.rs` to reflect new structure.

## Verification

After restructure:
```bash
# Should compile (on Linux)
cargo check

# eBPF should still build
cargo xtask build-ebpf

# Workspace members correct
cargo metadata --format-version 1 | jq '.workspace_members'
```

## Phase 2 Additions

When implementing DHCP/DNS, add:
```bash
mkdir -p crates/{beryl-dhcp,beryl-dns}
```

And add to workspace members.

## Phase 3 Additions

When implementing WiFi:
```bash
mkdir -p crates/beryl-wifi
```
