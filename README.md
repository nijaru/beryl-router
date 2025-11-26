# beryl-router

XDP/eBPF firewall for GL.iNet Beryl AX (GL-MT3000) travel router.

## Architecture

- **Data Plane (Kernel):** eBPF/XDP program processes packets at driver level
- **Control Plane (Userspace):** Rust daemon manages config and eBPF maps
- **OS:** Minimal OpenWrt with BTF support

## Requirements

### Build Host

**Must build on Linux** - aya requires Linux kernel headers (netlink, bpf syscalls).

- Rust nightly (`rustup default nightly`)
- `bpf-linker` (`cargo install bpf-linker`)
- Cross-compiler for aarch64 (`aarch64-linux-gnu-gcc` or zig)
- Build on Linux (e.g., Fedora) or use Docker/Podman

### Router

- OpenWrt with `CONFIG_DEBUG_INFO_BTF=y`
- MediaTek MT7981 (Filogic 820)

## Build

```bash
# Build eBPF program
cargo xtask build-ebpf --release

# Build userspace binary for router
cargo xtask build --release --target aarch64-unknown-linux-musl
```

## Deploy

```bash
scp target/aarch64-unknown-linux-musl/release/beryl-router root@192.168.8.1:/root/
scp config.example.json root@192.168.8.1:/etc/beryl-router/config.json
ssh root@192.168.8.1 "./beryl-router --interface eth0"
```

## Configuration

Edit `/etc/beryl-router/config.json`:

```json
{
  "blocked_ips": ["10.0.0.100"],
  "blocked_ports": [23, 25]
}
```

Changes are hot-reloaded automatically.

## License

MIT
