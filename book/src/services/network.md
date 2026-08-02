# network (Service)

NetworkManager integration service for WiFi, Ethernet, VPN, and airplane mode control.

## Description

The network service communicates with NetworkManager via D-Bus (`zbus`). It provides WiFi scanning, connection management, VPN control, and airplane mode
toggling. It broadcasts status updates when network state changes.

## Topics

| Topic                     | Direction         | Description                                |
|---------------------------|-------------------|--------------------------------------------|
| `service.network.command` | Widget → Service  | Scan, connect, disconnect, toggle airplane |
| `service.network.status`  | Service → Widgets | WiFi, Ethernet, VPN status                 |

## MCP Tools

| Tool                      | Description                |
|---------------------------|----------------------------|
| `network_scan`            | Scan for WiFi networks     |
| `network_connect`         | Connect to a WiFi network  |
| `network_disconnect`      | Disconnect from a network  |
| `network_toggle_airplane` | Toggle airplane mode       |
| `network_get_status`      | Get current network status |

## Configuration

```toml
[[services]]
id = "network"
path = "target/release/libsmearor_network_service.so"
```

## Crate

- **Path**: `services/network/`
- **Library**: `libsmearor_network_service.so`
- **Model**: `model/network/`
