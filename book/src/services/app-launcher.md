# app-launcher (Service)

Scans `.desktop` files, provides application search, and launches applications via `smearor-wrot` with rotation support.

## Description

The app-launcher service scans standard application directories for `.desktop` files, builds a searchable index, and handles launch requests from
the [app-launcher widget](../plugins/app-launcher.md). It uses `smearor-wrot` to launch applications with the correct rotation.

## Topics

| Topic                         | Direction         | Description                    |
|-------------------------------|-------------------|--------------------------------|
| `service.app_launcher.search` | Widget → Service  | Search for applications        |
| `service.app_launcher.launch` | Widget → Service  | Launch an application          |
| `service.app_launcher.list`   | Widget → Service  | List all applications          |
| `service.app_launcher.status` | Service → Widgets | Search results / launch status |

## MCP Tools

| Tool         | Description                                   |
|--------------|-----------------------------------------------|
| `app_launch` | Launch an application by name or desktop file |
| `app_search` | Search for applications by query              |

## Configuration

```toml
[app_launcher]
smearor_wrot_path = "smearor-wrot"
rotation = 0
```

| Field               | Type          | Description                                                                                              |
|---------------------|---------------|----------------------------------------------------------------------------------------------------------|
| `smearor_wrot_path` | `String`      | Path to the `smearor-wrot` binary                                                                        |
| `rotation`          | `Option<f32>` | Fallback rotation in degrees (used only when `follows_rotation = true` and no `wrapper.rotation` is set) |

## Crate

- **Path**: `services/app-launcher/`
- **Library**: `libsmearor_app_launcher_service.so`
- **Model**: `model/app-launcher/`
