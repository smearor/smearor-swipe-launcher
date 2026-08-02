# wayland (Service)

Wayland compositor integration service for layer-shell, monitor events, and workspace lifecycle.

## Description

The wayland service connects to the Wayland compositor via `wayland-client` and `wayland-protocols`. It handles monitor configuration changes, layer-shell
surface management, and workspace lifecycle events. It broadcasts compositor events to all instances.

## Topics

| Topic                            | Direction     | Description                   |
|----------------------------------|---------------|-------------------------------|
| `compositor::monitor_changed`    | Service → All | Monitor configuration changed |
| `compositor::workspace_changed`  | Service → All | Active workspace changed      |
| `compositor::instance_lifecycle` | Service → All | Instance lifecycle event      |

## Configuration

```toml
[[services]]
id = "wayland"
path = "target/release/libsmearor_wayland_service.so"
```

## Crate

- **Path**: `services/wayland/`
- **Library**: `libsmearor_wayland_service.so`
