# power (Plugin)

Power management widget with views for shutdown, reboot, suspend, hibernate, lock, logout, and reboot-to-firmware.

## Description

The power widget communicates with the [power service](../services/power.md) via the message broker. It cycles through power actions via swipe up/down, each
with its own icon.

## Views

| View               | Icon           | Description            |
|--------------------|----------------|------------------------|
| Shutdown           | Power icon     | Power off the system   |
| Reboot             | Restart icon   | Restart the system     |
| Suspend            | Sleep icon     | Suspend to RAM         |
| Hibernate          | Snowflake icon | Suspend to disk        |
| Lock               | Lock icon      | Lock the screen        |
| Logout             | Logout icon    | Log out of the session |
| Reboot to Firmware | BIOS icon      | Reboot into UEFI/BIOS  |

## Configuration

```toml
[power_widget]
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

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Click triggers the currently displayed power action.

## Related Service

- [power (service)](../services/power.md) — systemd/logind integration, power inhibitors, MCP tools

## Crate

- **Path**: `plugins/power/`
- **Library**: `libsmearor_power_widget.so`
- **Model**: `model/power/`
