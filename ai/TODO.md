# TODO

## Build System

- [ ] Install `bpf-linker`: `cargo install bpf-linker`
- [ ] Install cross-compiler: `brew install aarch64-elf-gcc` or use zig
- [ ] Verify eBPF build: `cargo xtask build-ebpf`
- [ ] Verify cross-compilation: `cargo xtask build --release`

## OpenWrt Image

- [ ] Clone OpenWrt: `git clone https://git.openwrt.org/openwrt/openwrt.git`
- [ ] Configure for MT7981 with BTF (`CONFIG_DEBUG_INFO_BTF=y`)
- [ ] Strip LuCI and unnecessary packages
- [ ] Build and flash to Beryl AX

## Features

- [ ] Add TC-BPF program for egress filtering
- [ ] Add rate limiting map
- [ ] Add connection tracking (simple)
- [ ] Add REST API for config (axum)
- [ ] Add systemd/procd service file

## Testing

- [ ] Test on x86 VM with veth pairs
- [ ] Test on actual Beryl AX hardware
- [ ] Benchmark vs stock nftables
