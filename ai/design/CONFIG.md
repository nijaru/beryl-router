# Configuration Schema

TOML configuration files for beryl-routerd.

## File Layout

```
/etc/beryl/
├── config.toml           # Main configuration
├── firewall.toml         # Firewall rules (optional, can be in main)
├── dhcp.toml             # DHCP settings (Phase 2)
├── dns.toml              # DNS settings (Phase 2)
└── wifi.toml             # WiFi settings (Phase 3)
```

## Main Configuration

`/etc/beryl/config.toml`:

```toml
# beryl-router configuration

[system]
hostname = "beryl"
timezone = "UTC"
log_level = "info"  # trace, debug, info, warn, error

[api]
listen = "0.0.0.0:8080"
# auth_token = "secret"  # Future: API authentication

[mode]
# Operating mode: router, ap, repeater, wireguard
type = "router"

# --- Interface Configuration ---

[interfaces.wan]
name = "eth0"
# Type: dhcp, static, pppoe
type = "dhcp"
# For static:
# address = "192.168.1.50/24"
# gateway = "192.168.1.1"
# dns = ["1.1.1.1", "8.8.8.8"]

[interfaces.lan]
name = "br-lan"
address = "192.168.8.1/24"
# Bridge members (physical interfaces)
members = ["eth1"]
# WiFi interfaces added automatically when wifi enabled

# --- Firewall Configuration ---

[firewall]
# Default policies
input = "drop"      # drop, accept
forward = "drop"
output = "accept"

# Enable connection tracking
conntrack = true

# Enable MTK hardware offload for established connections
hw_offload = true

[firewall.blocklist]
# IP addresses to block at XDP level (fast path)
ips = [
    "10.0.0.100",
]

# Ports to block (destination)
ports = [
    23,   # telnet
    25,   # smtp (outbound spam prevention)
]

# DNS-based blocklists (Phase 2, handled by DNS server)
dns_lists = [
    "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
]

[[firewall.rules]]
name = "allow-established"
action = "accept"
state = ["established", "related"]

[[firewall.rules]]
name = "allow-icmp"
action = "accept"
proto = "icmp"

[[firewall.rules]]
name = "allow-ssh-lan"
action = "accept"
proto = "tcp"
dest_port = 22
input_interface = "br-lan"

[[firewall.rules]]
name = "allow-http-lan"
action = "accept"
proto = "tcp"
dest_port = [80, 443]
input_interface = "br-lan"

[[firewall.rules]]
name = "allow-dns-lan"
action = "accept"
proto = ["tcp", "udp"]
dest_port = 53
input_interface = "br-lan"

[[firewall.rules]]
name = "allow-dhcp-lan"
action = "accept"
proto = "udp"
dest_port = [67, 68]
input_interface = "br-lan"

# --- Port Forwarding ---

[[firewall.port_forwards]]
name = "ssh-server"
proto = "tcp"
external_port = 2222
internal_ip = "192.168.8.50"
internal_port = 22

[[firewall.port_forwards]]
name = "web-server"
proto = "tcp"
external_port = 8080
internal_ip = "192.168.8.50"
internal_port = 80
```

## DHCP Configuration (Phase 2)

`/etc/beryl/dhcp.toml`:

```toml
[server]
enabled = true
interface = "br-lan"

[server.pool]
start = "192.168.8.100"
end = "192.168.8.250"
lease_time = "12h"

[server.options]
# Option 3: Router
gateway = "192.168.8.1"
# Option 6: DNS Server
dns = ["192.168.8.1"]
# Option 15: Domain Name
domain = "lan"
# Option 42: NTP Server
ntp = ["192.168.8.1"]

# Static leases (MAC -> IP binding)
[[server.static_leases]]
mac = "aa:bb:cc:dd:ee:ff"
ip = "192.168.8.50"
hostname = "server"

[[server.static_leases]]
mac = "11:22:33:44:55:66"
ip = "192.168.8.51"
hostname = "nas"

# DHCP client for WAN (when mode = router)
[client]
interface = "eth0"
# Request specific options
request_options = [1, 3, 6, 15, 28, 51]
# Send hostname
send_hostname = true
```

## DNS Configuration (Phase 2)

`/etc/beryl/dns.toml`:

```toml
[server]
enabled = true
listen = ["192.168.8.1:53", "[::1]:53"]

[resolver]
# Upstream DNS servers
upstream = [
    "1.1.1.1",
    "8.8.8.8",
]
# Use DNS-over-HTTPS
# doh_upstream = ["https://cloudflare-dns.com/dns-query"]

# Query timeout
timeout_ms = 5000

# Cache settings
[cache]
enabled = true
max_entries = 10000
min_ttl = 60
max_ttl = 86400

# Local records (authoritative for .lan)
[[local_records]]
name = "router.lan"
type = "A"
value = "192.168.8.1"

[[local_records]]
name = "server.lan"
type = "A"
value = "192.168.8.50"

# Blocking
[blocking]
enabled = true
# Response for blocked queries
response = "0.0.0.0"  # or "nxdomain"

# Blocklist sources (downloaded and merged)
lists = [
    "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
    "https://adaway.org/hosts.txt",
]

# Update interval
update_interval = "24h"

# Whitelist (never block)
whitelist = [
    "example.com",
]

# Additional blocks (manual)
blocklist = [
    "ads.example.com",
]
```

## WiFi Configuration (Phase 3)

`/etc/beryl/wifi.toml`:

```toml
[general]
enabled = true
country = "US"

# 5GHz Radio
[radio.phy0]
band = "5g"
channel = 36       # or "auto"
bandwidth = 80     # 20, 40, 80, 160
txpower = 20       # dBm

[[radio.phy0.ssid]]
name = "MyNetwork-5G"
hidden = false
encryption = "wpa3"  # wpa2, wpa3, wpa2-wpa3
password = "secretpassword"
# Isolate clients from each other
client_isolation = false

# Guest network (separate VLAN)
[[radio.phy0.ssid]]
name = "Guest"
hidden = false
encryption = "wpa2"
password = "guestpassword"
client_isolation = true
vlan = 100

# 2.4GHz Radio
[radio.phy1]
band = "2.4g"
channel = 6
bandwidth = 20
txpower = 20

[[radio.phy1.ssid]]
name = "MyNetwork"
encryption = "wpa2-wpa3"
password = "secretpassword"

# Client mode (for repeater)
# [client]
# ssid = "UpstreamNetwork"
# password = "upstreampassword"
# interface = "wlan0"
```

## WireGuard Configuration (Phase 3)

`/etc/beryl/wireguard.toml`:

```toml
[interface]
enabled = false
address = "10.10.10.2/24"
private_key = "BASE64_PRIVATE_KEY"
listen_port = 51820
dns = ["10.10.10.1"]

[[peer]]
public_key = "BASE64_PUBLIC_KEY"
endpoint = "vpn.example.com:51820"
allowed_ips = ["0.0.0.0/0"]  # Route all traffic
persistent_keepalive = 25
```

## Rust Types

```rust
// crates/beryl-config/src/lib.rs

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub system: SystemConfig,
    pub api: ApiConfig,
    pub mode: ModeConfig,
    pub interfaces: InterfacesConfig,
    pub firewall: FirewallConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    pub hostname: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModeConfig {
    #[serde(rename = "type")]
    pub mode_type: OperatingMode,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperatingMode {
    Router,
    Ap,
    Repeater,
    Wireguard,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FirewallConfig {
    #[serde(default)]
    pub input: Policy,
    #[serde(default)]
    pub forward: Policy,
    #[serde(default = "default_accept")]
    pub output: Policy,
    #[serde(default)]
    pub blocklist: BlocklistConfig,
    #[serde(default)]
    pub rules: Vec<FirewallRule>,
    #[serde(default)]
    pub port_forwards: Vec<PortForward>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    #[default]
    Drop,
    Accept,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FirewallRule {
    pub name: String,
    pub action: Policy,
    #[serde(default)]
    pub proto: Option<Protocol>,
    #[serde(default)]
    pub src_ip: Option<IpAddr>,
    #[serde(default)]
    pub dest_ip: Option<IpAddr>,
    #[serde(default)]
    pub dest_port: Option<PortSpec>,
    #[serde(default)]
    pub input_interface: Option<String>,
    #[serde(default)]
    pub state: Vec<ConnState>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PortForward {
    pub name: String,
    pub proto: Protocol,
    pub external_port: u16,
    pub internal_ip: IpAddr,
    pub internal_port: u16,
}

// etc...
```

## Validation

Config loading should validate:
- IP addresses are valid
- Ports are in range (1-65535)
- Interface names exist
- No conflicting rules
- Pool range is valid (start < end)
- Required fields present for mode

```rust
impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_interfaces()?;
        self.validate_firewall()?;
        self.validate_mode_requirements()?;
        Ok(())
    }
}
```
