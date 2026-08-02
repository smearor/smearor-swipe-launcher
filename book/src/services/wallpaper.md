# wallpaper (Service)

Wallpaper management service that scans for wallpaper themes and applies them.

## Description

The wallpaper service scans configured directories for wallpaper themes, manages the current wallpaper, and applies changes via the compositor (Hyprland) or
GNOME settings. It broadcasts the list of available themes and the current selection to all [wallpaper widgets](../plugins/wallpaper.md).

## Topics

| Topic                       | Direction         | Description                               |
|-----------------------------|-------------------|-------------------------------------------|
| `service.wallpaper.command` | Widget → Service  | Set wallpaper, list themes, next/previous |
| `service.wallpaper.status`  | Service → Widgets | Current wallpaper, available themes       |

## MCP Tools

| Tool                    | Description                         |
|-------------------------|-------------------------------------|
| `wallpaper_set`         | Set the wallpaper by theme name     |
| `wallpaper_list_themes` | List all available wallpaper themes |
| `wallpaper_next`        | Switch to the next theme            |
| `wallpaper_previous`    | Switch to the previous theme        |

## Configuration

```toml
[[services]]
id = "wallpaper"
path = "target/release/libsmearor_wallpaper_service.so"

[wallpaper]
wallpaper_dir = "~/Pictures/Wallpapers"
```

## Crate

- **Path**: `services/wallpaper/`
- **Library**: `libsmearor_wallpaper_service.so`
- **Model**: `model/wallpaper/`
