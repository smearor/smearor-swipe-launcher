# sysinfo (Plugin)

System information widgets providing real-time monitoring of CPU, memory, disk, network, temperature, uptime, and load. Ships as multiple sub-widgets from a
single crate.

## Sub-Widgets

| Widget      | `widget` config value | Description                     |
|-------------|-----------------------|---------------------------------|
| CPU         | `cpu`                 | CPU usage percentage with gauge |
| Memory      | `memory`              | RAM and swap usage              |
| Disk        | `disks`               | Disk space usage                |
| Network     | `network`             | Network I/O rates               |
| Uptime      | `uptime`              | System uptime                   |
| Load        | `load`                | System load average             |
| Temperature | `temperature`         | CPU/GPU temperatures            |

Additionally, a **sysinfo-multi** widget (`widget = "sysinfo"`) combines all views into a single widget with swipe cycling.

## Configuration

```toml
[cpu_widget]
path = "target/release/libsmearor_sysinfo_widget.so"
widget = "cpu"
icon = "nf-fae-chip"
show_icon = true

[cpu_package_temp_widget]
path = "target/release/libsmearor_sysinfo_widget.so"
widget = "temperature"
components = ["asusec CPU Package"]
format = "{temperature:.0}°C"
show_label = true
gauge_size = 120
```

| Field        | Type             | Description                                   |
|--------------|------------------|-----------------------------------------------|
| `icon`       | `Option<String>` | Icon name                                     |
| `icon_size`  | `i32`            | Icon size                                     |
| `show_icon`  | `bool`           | Whether to show an icon                       |
| `components` | `Vec<String>`    | Temperature sensor names (temperature widget) |
| `format`     | `String`         | Display format string                         |
| `show_label` | `bool`           | Show sensor label                             |
| `gauge_size` | `i32`            | Gauge diameter in pixels                      |
| `mode`       | `WidgetMode`     | `compact` or `wide` (sysinfo-multi)           |
| `max_width`  | `Option<i32>`    | Maximum widget width (sysinfo-multi)          |
| `views`      | `Vec<String>`    | Which views to include (sysinfo-multi)        |

## Action Bindings

Supports all [action binding types](../features/action-bindings.md).

## Related Service

- [sysinfo (service)](../services/sysinfo.md) — System metrics collection, MCP tools

## Crate

- **Path**: `plugins/sysinfo/`
- **Library**: `libsmearor_sysinfo_widget.so`
- **Model**: `model/sysinfo/`
