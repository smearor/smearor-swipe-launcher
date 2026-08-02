# app-launcher (Plugin)

Launches applications from `.desktop` files. Each instance loads a single desktop entry and displays its icon, name, and description.

## Description

The app-launcher widget reads a `.desktop` file path from its config, parses it with `freedesktop_entry_parser`, and displays the application icon and name. On
click, it launches the application via `smearor-wrot` with the current rotation parameter.

## Configuration

```toml
[my_app]
defaults = "app_launcher"
desktop_file_path = "/usr/share/applications/myapp.desktop"
icon = "nf-md-apps"
info_text = "My App"
[my_app.wrapper]
follows_rotation = true
```

| Field                      | Type             | Description                                    |
|----------------------------|------------------|------------------------------------------------|
| `desktop_file_path`        | `String`         | Path to the `.desktop` file                    |
| `icon`                     | `Option<String>` | Override icon (defaults to desktop entry icon) |
| `icon_size`                | `i32`            | Icon size in pixels                            |
| `icon_only`                | `bool`           | Show only the icon                             |
| `info_text`                | `Option<String>` | Override description text                      |
| `mode`                     | `WidgetMode`     | `compact` or `wide`                            |
| `wrapper.follows_rotation` | `bool`           | Pass rotation to `smearor-wrot`                |

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). The default fallback for click launches the application. Use `supplement` mode to send a
message in addition to launching.

## Related Service

- [app-launcher (service)](../services/app-launcher.md) — Scans `.desktop` files, provides search, registers MCP tools

## Crate

- **Path**: `plugins/app-launcher/`
- **Library**: `libsmearor_app_launcher_widget.so`
- **Model**: `model/app-launcher/`
