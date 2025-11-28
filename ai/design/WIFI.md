# WiFi Management Architecture

## Overview

The WiFi management subsystem in Beryl Router is responsible for controlling the wireless interfaces (`wlan0`, `wlan1` on GL-MT3000) to provide Access Point (AP) functionality.

On the GL-MT3000 (MediaTek MT7981), the WiFi hardware requires proprietary drivers (`mt7976`) and the OpenWrt-patched `hostapd` daemon. We cannot replace `hostapd` entirely. Instead, `beryl-routerd` will act as a controller for `hostapd`.

## Hardware Context

- **2.4GHz Radio:** `wlan0` (MT7981 internal?)
- **5GHz Radio:** `wlan1` (MT7976)
- **Drivers:** OpenWrt `kmod-mt7981-firmware`, `kmod-mt76`, etc.
- **Daemon:** `hostapd` running in userspace.

## Control Strategy

We have two primary options for controlling `hostapd` on OpenWrt:

1.  **UBUS (OpenWrt Micro Bus):** OpenWrt patches `hostapd` to expose a `ubus` interface (`hostapd.wlan0`, `hostapd.wlan1`).
    *   **Pros:** Native to OpenWrt, allows dynamic reload, status querying, client kick/ban.
    *   **Cons:** Requires `ubus` crate or FFI bindings.
2.  **Unix Socket (Standard hostapd control interface):** The standard `hostapd_cli` mechanism.
    *   **Pros:** Standard, portable, well-supported by crates like `wifi-ctrl`.
    *   **Cons:** OpenWrt configuration (`/etc/config/wireless` -> `netifd` -> `hostapd.sh` -> `hostapd.conf`) manages the daemon lifecycle. Editing `hostapd.conf` directly fights `netifd`.

### Decision: Hybrid Approach

We will use **UBUS** for dynamic control (status, client management) and **OpenWrt Config Files** (`/etc/config/wireless`) for persistent configuration (SSID, password, channel), triggering `reload_config` via `ubus` or `wifi reload`.

Actually, for a "full replacement" feel, we might want to generate `/etc/config/wireless` from our own `config.toml` and then tell OpenWrt to reload.

## Component Design

```rust
pub struct WifiManager {
    // Handle to UBUS or method to invoke commands
}

impl WifiManager {
    pub fn apply_config(config: &WifiConfig) -> Result<()> {
        // 1. Generate /etc/config/wireless content
        // 2. Write file
        // 3. Trigger 'wifi reload'
    }

    pub fn get_status() -> Result<WifiStatus> {
        // Use UBUS to query hostapd state
    }
}
```

## Beryl-Wifi Crate

New crate `crates/beryl-wifi`:
- `config.rs`: Types for WiFi config (SSID, encryption, etc.) mapping to OpenWrt UCI.
- `manager.rs`: Logic to write UCI files and trigger reloads.
- `ubus.rs`: (Optional) Wrapper around `ubus` CLI or bindings for status.

## OpenWrt Wireless Config Structure

```uci
config wifi-device 'radio0'
    option type 'mac80211'
    option path 'platform/soc/18000000.wifi'
    option channel '36'
    option band '5g'
    option htmode 'HE80'

config wifi-iface 'default_radio0'
    option device 'radio0'
    option network 'lan'
    option mode 'ap'
    option ssid 'Beryl-5G'
    option encryption 'psk2'
    option key 'password'
```

## Plan

1.  Define `WifiConfig` in `beryl-config`.
2.  Implement `beryl-wifi` to:
    *   Read `WifiConfig`.
    *   Generate UCI compatible `/etc/config/wireless`.
    *   Execute `/sbin/wifi reload` to apply.
3.  Integrate into `beryl-routerd` startup/config reload.

## Dependencies

- `rust-uci` (maybe, or just write text files since UCI is just a format). Writing raw files is safer/simpler if we own the file.
- `std::process::Command` for reloading.

