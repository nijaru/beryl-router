# Decisions

## Architecture

| Decision | Context | Rationale | Tradeoffs |
|----------|---------|-----------|-----------|
| XDP over nftables | Need fast packet processing | XDP processes at driver level, ~0 CPU for drops | More complex than nftables, limited to ingress |
| Rust + aya | Need safe eBPF development | Type-safe, good tooling, active community | Requires nightly, less documentation than C |
| Separate eBPF crate | eBPF compiles to different target | Cannot be workspace member (bpfel-unknown-none) | Extra build step via xtask |
| MUSL target | Static linking for OpenWrt | No glibc dependency, simpler deployment | Slightly larger binary |
| File-based config | Simple config management | Hot-reload via inotify, easy to edit | No auth, local only |

## OpenWrt

| Decision | Context | Rationale | Tradeoffs |
|----------|---------|-----------|-----------|
| Custom image build | Need BTF support | Stock images lack `CONFIG_DEBUG_INFO_BTF` | Must maintain custom build |
| Keep MTK wifi drivers | Need wifi functionality | Proprietary drivers only in OpenWrt | Locked to OpenWrt kernel |
