# wallpaper (Plugin)

Wallpaper management widget with theme preview. Cycles through wallpaper themes via swipe and shows a preview image or fallback icon.

## Description

The wallpaper widget communicates with the [wallpaper service](../services/wallpaper.md). Each theme is a view with its own preview image or fallback icon. The
widget supports setting wallpapers and browsing available themes.

## Configuration

```toml
[wallpaper_widget]
icon_size = 32
icon_only = false
mode = "compact"
max_width = 200
show_type_icon = true
fallback_icon = "nf-md-image"
```

| Field                | Type             | Description                          |
|----------------------|------------------|--------------------------------------|
| `icon_size`          | `i32`            | Icon size in pixels                  |
| `icon_only`          | `bool`           | Show only the icon                   |
| `mode`               | `WidgetMode`     | `compact` or `wide`                  |
| `max_width`          | `Option<i32>`    | Maximum widget width                 |
| `show_type_icon`     | `bool`           | Show wallpaper type icon overlay     |
| `fallback_icon`      | `String`         | Icon when no preview image available |
| `preview_image_path` | `Option<String>` | Custom preview image path            |
| `preview_icon`       | `Option<String>` | Custom preview icon                  |

## Dynamic Icons

Each theme is a view. The preview image or fallback icon comes from theme data (`preview_image_path`, `preview_icon`, `wallpaper_type_icon`).

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Click applies the current theme.

## Related Service

- [wallpaper (service)](../services/wallpaper.md) — Wallpaper management, theme scanning, MCP tools

## Crate

- **Path**: `plugins/wallpaper/`
- **Library**: `libsmearor_wallpaper_widget.so`
- **Model**: `model/wallpaper/`
