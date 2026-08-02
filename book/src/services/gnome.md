# gnome (Service)

GNOME shell integration service for settings, shell extensions, and D-Bus interactions.

## Description

The GNOME service communicates with GNOME Shell and GNOME Settings via D-Bus (`zbus`). It provides access to GNOME settings, shell extensions, and system
preferences.

## Topics

| Topic                   | Direction         | Description         |
|-------------------------|-------------------|---------------------|
| `service.gnome.command` | Widget → Service  | GNOME commands      |
| `service.gnome.status`  | Service → Widgets | GNOME state updates |

## MCP Tools

| Tool                  | Description         |
|-----------------------|---------------------|
| `gnome_open_settings` | Open GNOME Settings |

## Configuration

Configured in `configs/services/services.toml`:

```toml
[[services]]
id = "gnome"
path = "target/release/libsmearor_gnome_service.so"
```

## Crate

- **Path**: `services/gnome/`
- **Library**: `libsmearor_gnome_service.so`
