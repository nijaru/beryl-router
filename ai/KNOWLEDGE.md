# Knowledge

Permanent quirks and gotchas for this project.

## Hardware (GL-MT3000)

| Area | Knowledge | Impact | Source |
|------|-----------|--------|--------|
| WiFi drivers | MT7976 requires proprietary blobs from MediaTek | Must use OpenWrt (has patches), cannot use mainline | OpenWrt wiki |
| Ethernet | eth0=WAN, eth1=LAN (RTL8221B PHY) | Interface names are fixed | GL-iNet docs |
| Flash layout | 512MB NAND, UBI filesystem | Plenty of space, use squashfs+overlayfs | OpenWrt |
| U-Boot | Recovery at 192.168.1.1, hold reset 10s | Safety net for bricked firmware | GL-iNet docs |
| LED | Single RGB LED, controllable via GPIO | Can indicate status | OpenWrt |
| Reset button | GPIO, can trigger events | Use for factory reset | OpenWrt |

## eBPF/XDP

| Area | Knowledge | Impact | Source |
|------|-----------|--------|--------|
| BTF requirement | aya uses CO-RE, needs kernel BTF | Must build OpenWrt with CONFIG_DEBUG_INFO_BTF | aya docs |
| XDP modes | Native (fastest), SKB (compatible), offload (HW) | MT7981 may not support native on all interfaces | Testing needed |
| XDP + bridge | XDP doesn't work on bridge interfaces | Attach to physical interfaces before bridging | Kernel docs |
| Map size limits | Per-CPU arrays limited by CPU count (2 on MT7981) | Design maps accordingly | eBPF docs |

## Network Stack

| Area | Knowledge | Impact | Source |
|------|-----------|--------|--------|
| MTK flow offload | Hardware NAT acceleration in MT7981 | Don't fight it, let it handle established flows | OpenWrt wiki |
| nftables flowtables | Software fast-path for established connections | Complements XDP, enable for throughput | Kernel docs |
| conntrack | Required for NAT, stateful firewall | XDP can't do stateful, use nftables | Kernel docs |
| hostapd | Only way to manage MT7976 WiFi | Must wrap/manage it, can't replace | OpenWrt |

## Development Environment

| Area | Knowledge | Impact | Source |
|------|-----------|--------|--------|
| aya on macOS | Doesn't compile - needs Linux headers | Build on Fedora or Docker | Testing |
| Cross-compile | aarch64-unknown-linux-musl target | Need linker: aarch64-linux-gnu-gcc or zig | Rust docs |
| bpf-linker | Required for eBPF compilation | cargo install bpf-linker (Linux only) | aya docs |
| Docker (Mac) | Use `rust-slim-bookworm` based image on Apple Silicon | Allows native aarch64 builds without cross-compiling | User |

## User Context

| Area | Knowledge | Impact | Source |
|------|-----------|--------|--------|
| Home network | Xfinity all-in-one → Asus AP → devices | Beryl will be behind existing NAT | User |
| Dev machines | Mac M3 (nick@apple), Fedora i9+4090 (nick@fedora) | Build on Fedora, test from Mac | User |
| Use case | Travel router, may run without WAN | Must work in AP mode, offline capable | User |
