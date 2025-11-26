# Status

## Current State

| Aspect | Status | Notes |
|--------|--------|-------|
| Project scaffold | Complete | Workspace with 3 crates |
| eBPF program | Code complete | Needs build verification |
| Userspace daemon | Code complete | Needs build verification |
| OpenWrt image | Not started | Requires custom build with BTF |
| Hardware testing | Not started | Need Beryl AX device |

## Blockers

| Blocker | Impact | Resolution |
|---------|--------|------------|
| aya requires Linux | Cannot build on macOS | Build on Fedora (nick@fedora) or Docker |
| No BTF-enabled OpenWrt | Cannot test on device | Build custom image |
| Cross-compiler needed | Cannot build for aarch64 | Install `aarch64-linux-gnu-gcc` |

## Recent Commits

- Initial project scaffold with XDP/eBPF architecture

## Learnings

- XDP is ingress-only; need TC-BPF for egress filtering
- MediaTek HW flow offload may outperform XDP for simple NAT
- `bpf-linker` required for eBPF compilation
