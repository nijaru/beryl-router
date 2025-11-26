# REST API Design

HTTP API served by beryl-routerd on port 8080.

## Authentication

Phase 1: None (local network only)
Future: API key header or session token

## Endpoints

### System

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/status | System status and health |
| GET | /api/v1/stats | Packet statistics from eBPF |
| POST | /api/v1/reboot | Reboot router |
| POST | /api/v1/restart | Restart beryl-routerd |

#### GET /api/v1/status

```json
{
  "version": "0.1.0",
  "uptime_secs": 3600,
  "mode": "router",
  "interfaces": {
    "wan": {
      "name": "eth0",
      "status": "up",
      "ip": "192.168.1.50",
      "mac": "aa:bb:cc:dd:ee:ff"
    },
    "lan": {
      "name": "br-lan",
      "status": "up",
      "ip": "192.168.8.1",
      "mac": "aa:bb:cc:dd:ee:00"
    }
  },
  "services": {
    "dhcp_server": "running",
    "dns_server": "running",
    "wifi": "running"
  }
}
```

#### GET /api/v1/stats

```json
{
  "packets": {
    "total": 1000000,
    "passed": 999000,
    "dropped": 1000
  },
  "bytes": {
    "rx": 1073741824,
    "tx": 536870912
  },
  "connections": {
    "active": 42,
    "total": 1000
  }
}
```

### Configuration

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/config | Full configuration |
| PUT | /api/v1/config | Update full configuration |
| PATCH | /api/v1/config/{section} | Update config section |

#### GET /api/v1/config

```json
{
  "system": {
    "hostname": "beryl",
    "timezone": "UTC"
  },
  "mode": {
    "type": "router"
  },
  "interfaces": {
    "wan": {
      "type": "dhcp"
    },
    "lan": {
      "address": "192.168.8.1/24"
    }
  },
  "firewall": {
    "blocked_ips": ["10.0.0.100"],
    "blocked_ports": [23, 25]
  }
}
```

### Mode

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/mode | Current operating mode |
| POST | /api/v1/mode | Switch operating mode |

#### POST /api/v1/mode

Request:
```json
{
  "mode": "ap"
}
```

Response:
```json
{
  "previous": "router",
  "current": "ap",
  "status": "switching",
  "message": "Mode change in progress, network will restart"
}
```

### Firewall

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/firewall/rules | List firewall rules |
| POST | /api/v1/firewall/rules | Add firewall rule |
| DELETE | /api/v1/firewall/rules/{id} | Delete firewall rule |
| GET | /api/v1/firewall/blocklist | IP blocklist |
| POST | /api/v1/firewall/blocklist | Add to blocklist |
| DELETE | /api/v1/firewall/blocklist/{ip} | Remove from blocklist |
| GET | /api/v1/firewall/portforwards | Port forwarding rules |
| POST | /api/v1/firewall/portforwards | Add port forward |
| DELETE | /api/v1/firewall/portforwards/{id} | Delete port forward |

#### POST /api/v1/firewall/blocklist

Request:
```json
{
  "ip": "10.0.0.100",
  "reason": "malicious"
}
```

#### POST /api/v1/firewall/portforwards

Request:
```json
{
  "name": "ssh-server",
  "proto": "tcp",
  "external_port": 2222,
  "internal_ip": "192.168.8.50",
  "internal_port": 22
}
```

### DHCP (Phase 2)

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/dhcp/leases | Active DHCP leases |
| GET | /api/v1/dhcp/static | Static lease mappings |
| POST | /api/v1/dhcp/static | Add static lease |
| DELETE | /api/v1/dhcp/static/{mac} | Remove static lease |

#### GET /api/v1/dhcp/leases

```json
{
  "leases": [
    {
      "ip": "192.168.8.100",
      "mac": "aa:bb:cc:dd:ee:ff",
      "hostname": "laptop",
      "expires": "2024-01-15T12:00:00Z"
    }
  ]
}
```

### DNS (Phase 2)

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/dns/stats | DNS query statistics |
| GET | /api/v1/dns/cache | Cache contents |
| POST | /api/v1/dns/cache/flush | Flush DNS cache |
| GET | /api/v1/dns/blocklist | DNS blocklist status |
| POST | /api/v1/dns/blocklist/update | Update blocklists |

### WiFi (Phase 3)

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/wifi/status | WiFi status |
| GET | /api/v1/wifi/clients | Connected clients |
| GET | /api/v1/wifi/config | WiFi configuration |
| PUT | /api/v1/wifi/config | Update WiFi config |
| POST | /api/v1/wifi/scan | Scan for networks |

### Clients

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/clients | All connected clients |
| GET | /api/v1/clients/{mac} | Specific client info |
| POST | /api/v1/clients/{mac}/block | Block client |
| DELETE | /api/v1/clients/{mac}/block | Unblock client |

#### GET /api/v1/clients

```json
{
  "clients": [
    {
      "mac": "aa:bb:cc:dd:ee:ff",
      "ip": "192.168.8.100",
      "hostname": "laptop",
      "interface": "wlan0",
      "connected_at": "2024-01-15T10:00:00Z",
      "rx_bytes": 1000000,
      "tx_bytes": 500000,
      "signal": -45
    }
  ]
}
```

## Error Responses

All errors return:
```json
{
  "error": {
    "code": "INVALID_CONFIG",
    "message": "Invalid IP address format",
    "details": {
      "field": "ip",
      "value": "not-an-ip"
    }
  }
}
```

HTTP status codes:
- 200: Success
- 400: Bad request (invalid input)
- 404: Not found
- 409: Conflict (e.g., duplicate rule)
- 500: Internal error

## Implementation Notes

### Axum Router Structure

```rust
fn api_router() -> Router {
    Router::new()
        .route("/status", get(status_handler))
        .route("/stats", get(stats_handler))
        .route("/config", get(get_config).put(put_config))
        .route("/mode", get(get_mode).post(set_mode))
        .nest("/firewall", firewall_router())
        .nest("/dhcp", dhcp_router())
        .nest("/dns", dns_router())
        .nest("/wifi", wifi_router())
        .nest("/clients", clients_router())
        .layer(TraceLayer::new_for_http())
}
```

### Shared State

```rust
struct AppState {
    config: Arc<RwLock<Config>>,
    ebpf: Arc<EbpfManager>,
    nft: Arc<NftManager>,
    dhcp: Arc<DhcpService>,  // Phase 2
    dns: Arc<DnsService>,    // Phase 2
    wifi: Arc<WifiManager>,  // Phase 3
}
```
