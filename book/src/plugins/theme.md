# theme (Plugin)

Theme switcher widget with per-theme views, preview images, and swipe navigation. Cycles through themes via swipe and applies the selected theme via click.

## Description

The theme widget communicates with the [theme service](../services/theme.md). Each configured theme is a view with its own preview image or fallback Nerd Font
icon. The widget mirrors the [wallpaper widget](./wallpaper.md) pattern:

- **Swipe up/down** cycles through themes (selects without applying)
- **Click** applies the currently selected theme
- **Preview image** displayed when `preview_image_path` is set, falls back to `preview_icon` otherwise
- **✓ indicator** shown in info text when the selected theme is also the applied theme

## Configuration

```toml
[theme_widget]
icon_size = 36
fallback_icon = "nf-md-palette_outline"
#click_topic = "area.open"
#click_payload = { area_id = "theme_area" }
```

| Field           | Type     | Description                                  |
|-----------------|----------|----------------------------------------------|
| `icon_size`     | `i32`    | Icon size in pixels                          |
| `fallback_icon` | `String` | Icon when no preview image or icon available |

### Theme Icons

| Field               | Default                  | Description                       |
|---------------------|--------------------------|-----------------------------------|
| `icon_theme`        | `nf-md-palette`          | Fallback icon for themes          |
| `icon_theme_dark`   | `nf-md-weather_night`    | Dark mode theme icon (reserved)   |
| `icon_theme_light`  | `nf-md-weather_sunny`    | Light mode theme icon (reserved)  |
| `icon_theme_system` | `nf-md-theme_light_dark` | System mode theme icon (reserved) |
| `icon_no_theme`     | `nf-md-palette_outline`  | No theme available / fallback     |

## Interaction

| Gesture    | Action                                   |
|------------|------------------------------------------|
| Swipe Up   | Select next theme (without applying)     |
| Swipe Down | Select previous theme (without applying) |
| Click      | Apply the currently selected theme       |
| Long-press | Apply the selected theme (fallback)      |

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). When click/longpress actions are configured, they replace the fallback behavior.

## Related Service

- [theme (service)](../services/theme.md) — Theme management, CSS application, wallpaper coupling, MCP tools

## Crate

- **Path**: `plugins/theme/`
- **Library**: `libsmearor_theme_widget.so`
- **Model**: `model/theme/`
