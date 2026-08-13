# theme (Service)

Theme management service that loads theme definitions, applies CSS, and optionally couples with the wallpaper service.

## Description

The theme service loads theme definitions from `themes.toml`, applies CSS providers to the GTK display, and broadcasts status to
all [theme widgets](../plugins/theme.md). It supports:

- **Named themes** with CSS files, colors, and metadata
- **CSS custom properties** (`--theme-color-1` through `--theme-color-5`) injected per mode
- **Mode-aware switching** (Dark, Light, System)
- **Wallpaper coupling** — applying a theme can also switch the wallpaper
- **Personalization integration** — System-mode themes react to color scheme changes
- **Hot-reload** — CSS file changes are picked up without restarting

## Topics

| Topic                   | Direction         | Description                           |
|-------------------------|-------------------|---------------------------------------|
| `service.theme.command` | Widget → Service  | Select theme, apply selected, refresh |
| `service.theme.status`  | Service → Widgets | Current theme, available themes, mode |

## MCP Tools

| Tool        | Parameters     | Description                                    |
|-------------|----------------|------------------------------------------------|
| `get_theme` | —              | Get current theme status (applied theme, mode) |
| `set_theme` | `name: String` | Select and apply a theme by name immediately   |

## Configuration

```toml
[[services]]
id = "theme"
path = "target/release/libsmearor_theme_service.so"

[theme]
default_theme = "default"
auto_apply = true
follow_system_color_scheme = true
#config_path = "configs/services/themes.toml"
```

| Field                        | Type     | Default | Description                                         |
|------------------------------|----------|---------|-----------------------------------------------------|
| `default_theme`              | `String` | `""`    | Theme to apply on startup                           |
| `auto_apply`                 | `bool`   | `false` | Apply default theme automatically on startup        |
| `follow_system_color_scheme` | `bool`   | `true`  | Re-apply CSS for System-mode themes on color change |
| `config_path`                | `String` | `""`    | Path to themes.toml (auto-discovered if empty)      |

### Config File Discovery

1. Working directory → `themes.toml`
2. `~/.config/smearor/services/themes.toml`
3. `/usr/share/smearor/services/themes.toml`

## Theme Definitions (`themes.toml`)

```toml
[[themes]]
name = "default"
description = "Default Smearor theme"
mode = "System"
css_files_dark = ["~/.config/smearor/themes/default-dark.css"]
css_files_light = ["~/.config/smearor/themes/default-light.css"]
preview_icon = "nf-md-palette"
preview_image_path = ""
wallpaper_theme = "Smearor"

[[themes]]
name = "Halloween"
description = "Spooky Halloween theme"
mode = "Dark"
css_files_dark = ["~/.config/smearor/themes/halloween.css"]
preview_icon = "nf-md-ghost"
preview_image_path = "~/Bilder/Themes/halloween-preview.png"

[themes.colors.dark]
color_1 = "#ff6b00ff"
color_2 = "#8b00ffff"
color_3 = "#00ff00ff"
color_4 = "#ff0000ff"
color_5 = "#fff200ff"

wallpaper_theme = "Halloween"
```

### Theme Fields

| Field                | Type           | Description                                            |
|----------------------|----------------|--------------------------------------------------------|
| `name`               | `String`       | Human-readable theme name                              |
| `description`        | `String`       | Theme description                                      |
| `mode`               | `String`       | `Dark`, `Light`, or `System`                           |
| `css_files_dark`     | `[String]`     | CSS files for Dark mode                                |
| `css_files_light`    | `[String]`     | CSS files for Light mode (falls back to dark if empty) |
| `preview_icon`       | `String`       | Nerd Font icon name for widget display                 |
| `preview_image_path` | `String`       | Optional preview image path (overrides icon)           |
| `colors.dark`        | `ThemePalette` | 5 colors for Dark mode (`color_1`–`color_5`)           |
| `colors.light`       | `ThemePalette` | 5 colors for Light mode                                |
| `wallpaper_theme`    | `String?`      | Optional wallpaper theme name to couple                |

### CSS Custom Properties

Each theme defines 5 colors per mode, exported as CSS variables:

| CSS Variable      | Default     | Color Name       |
|-------------------|-------------|------------------|
| `--theme-color-1` | `#04e762ff` | malachite        |
| `--theme-color-2` | `#f5b700ff` | selective-yellow |
| `--theme-color-3` | `#00a1e4ff` | celestial-blue   |
| `--theme-color-4` | `#dc0073ff` | mexican-pink     |
| `--theme-color-5` | `#89fc00ff` | chartreuse       |

CSS files can reference these via `var(--theme-color-1)` etc.

## CSS Provider Priority

Theme CSS is registered at `STYLE_PROVIDER_PRIORITY_USER + 2`, above all other CSS sources.

## Wallpaper Coupling

When a theme has `wallpaper_theme` set, applying it also:

1. Selects the wallpaper theme (`WallpaperCommandMessage::SelectTheme`)
2. Starts the wallpaper process (`WallpaperCommandMessage::StartSelected`)

## Personalization Integration

The service subscribes to `service.personalization.status`. When the color scheme changes and the current theme is `System` mode, the service re-applies CSS
with the appropriate dark/light files.

## Crate

- **Path**: `services/theme/`
- **Library**: `libsmearor_theme_service.so`
- **Model**: `model/theme/`
