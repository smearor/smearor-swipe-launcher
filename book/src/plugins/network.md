# network (Plugin)

Network status and control widget with 7 views: WiFi status, Ethernet status, Throughput, WiFi scan, VPN, Airplane mode, and QR code.

## Description

The network widget communicates with the [network service](../services/network.md) via the message broker. It cycles through views via swipe up/down gestures,
each showing different network information.

## Views

| View            | Description                             |
|-----------------|-----------------------------------------|
| WiFi Status     | SSID, signal strength, connection state |
| Ethernet Status | Connection state, IP address            |
| Throughput      | Download/upload rates                   |
| WiFi Scan       | Available networks list                 |
| VPN             | VPN profiles and connection state       |
| Airplane        | Airplane mode toggle                    |
| QR Code         | QR code for WiFi sharing                |

## Configuration

```toml
[network_menu_widget]
icon_size = 32
icon_only = false
mode = "compact"
max_width = 200
```

| Field       | Type          | Description          |
|-------------|---------------|----------------------|
| `icon_size` | `i32`         | Icon size in pixels  |
| `icon_only` | `bool`        | Show only the icon   |
| `mode`      | `WidgetMode`  | `compact` or `wide`  |
| `max_width` | `Option<i32>` | Maximum widget width |

## Dynamic Icons

Each view has its own icon. The WiFi status view additionally has state-dependent icons based on signal strength.

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Swipe up/down cycles views.

## Related Service

- [network (service)](../services/network.md) — NetworkManager integration, WiFi scanning, VPN, MCP tools

## Crate

- **Path**: `plugins/network/`
- **Library**: `libsmearor_network_widget.so`
- **Model**: `model/network/`
