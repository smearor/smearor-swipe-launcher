# Launcher Configuration

The main launcher configuration is in `configs/launcher/config.toml`. It defines the layout, areas, plugins, and per-plugin settings.

## Structure

```toml
# Area layout order
areas = ["left_area", "scroll_band", "right_area"]

# Per-workspace layout profiles
[[profiles]]
trigger = { Workspace = 2 }
areas = ["development_band"]

# Global default templates
[defaults.menu_button]
click_topic = "area.open"
enabled = true

# Application settings
[launcher]
rotation = 0
layer = "top"
namespace = "smearor-swipe-launcher"
exclusive_zone = 105
max_width = 1080

# Layout settings
[layout]
orientation = "horizontal"
spacing = 0

# Area definitions
[scroll_band]
area_type = "scroll"
plugins = [
    { id = "clock_widget", path = "target/release/libsmearor_clock_widget.so" },
    { id = "audio", path = "target/release/libsmearor_audio_widget.so", widget = "audio" }
]

# Plugin-specific configurations
[clock_widget]
mode = "compact"
timezone = "local"
```

## Sections

### `[launcher]`

| Field              | Type     | Default | Description                                                          |
|--------------------|----------|---------|----------------------------------------------------------------------|
| `rotation`         | `i32`    | `0`     | Initial rotation (0, 90, 180, 270)                                   |
| `layer`            | `String` | `"top"` | Layer-shell layer                                                    |
| `namespace`        | `String` | —       | Layer-shell namespace                                                |
| `exclusive_zone`   | `i32`    | —       | Exclusive zone width                                                 |
| `max_width`        | `i32`    | —       | Maximum window width                                                 |
| `show_decorations` | `bool`   | `false` | Show window decorations                                              |
| `scale`            | `f32`    | `1.0`   | Global widget scaling factor (see [Widget Scaling](#widget-scaling)) |

### `[layout]`

| Field         | Type     | Default        | Description                |
|---------------|----------|----------------|----------------------------|
| `orientation` | `String` | `"horizontal"` | `horizontal` or `vertical` |
| `spacing`     | `i32`    | `0`            | Spacing between areas      |

### `[[profiles]]`

| Field     | Type             | Description                                  |
|-----------|------------------|----------------------------------------------|
| `trigger` | `ProfileTrigger` | Workspace trigger (e.g. `{ Workspace = 2 }`) |
| `areas`   | `Vec<String>`    | Area IDs for this profile                    |

### `[defaults.{name}]`

Default templates that plugins can inherit from with `defaults = "{name}"`. Instance-specific values override the template.

## Plugin Entries

Plugins are listed in an area's `plugins` array:

```toml
# Using path= (development / from-source)
plugins = [
    { id = "clock_widget", path = "target/release/libsmearor_clock_widget.so" },
    { id = "sysinfo", path = "target/release/libsmearor_sysinfo_widget.so", widget = "cpu", disabled = false }
]

# Using name= (system installation / Debian packages)
plugins = [
    { id = "clock", name = "clock_widget" },
    { id = "battery", name = "sysinfo_widget", widget = "battery" }
]
```

| Field      | Type             | Description                                                        |
|------------|------------------|--------------------------------------------------------------------|
| `id`       | `String`         | Unique plugin instance ID                                          |
| `path`     | `Option<String>` | Path to the `.so` file (mutually exclusive with `name`)            |
| `name`     | `Option<String>` | Short name for library resolution (mutually exclusive with `path`) |
| `widget`   | `Option<String>` | Sub-widget selector (e.g. `"cpu"` for sysinfo)                     |
| `disabled` | `bool`           | Whether the plugin is disabled                                     |

### `path` vs `name`

Either `path` or `name` must be specified for each plugin entry:

- **`path`** — explicit file path to the `.so` file. Used in development configs. Relative paths are resolved from the working directory; `~` is expanded to the
  home directory.
- **`name`** — short name used for library resolution. The host searches for `libsmearor_<name>.so` in:
    1. `~/.local/lib/smearor/` (user-local)
    2. `/usr/lib/smearor/` (system-wide, e.g. Debian package installation)

### Config Discovery

The launcher discovers config files in this fallback order:

1. CLI `--config` argument
2. `*.toml` in the working directory (excluding `services.toml`, `wallpaper.toml`)
3. `~/.config/smearor/launcher/*.toml` (user config)
4. `/usr/share/smearor/launcher/*.toml` (system default)

On first run, the launcher copies default configs from `/usr/share/smearor/` to `~/.config/smearor/` if they don't already exist.

### Per-Instance CSS

Each config file can have an accompanying CSS file. Given `my-launcher.toml`, the launcher looks for `my-launcher.css` in the same directory. If found, it is
loaded with higher priority than the global user CSS, allowing per-instance style overrides.

See [Design and CSS](./design-css.md) for the full CSS layer system and hot-reload details.

See [Area Configuration](./area-config.md) for area-specific settings, and individual [plugin pages](../plugins/app-launcher.md) for plugin-specific config
fields.

## Widget Scaling

The `scale` field in `[launcher]` controls a global scaling factor for all GTK widget dimensions and font sizes. This is useful for high-DPI displays or
accessibility.

### How It Works

- **Pixel dimensions** (`width`, `height`, `icon_size`, spacing, label heights) are multiplied by the scale factor.
- **CSS font sizes** are scaled via a global CSS provider (`.widget-main-text`, `.widget-info-text`, `.nerd-icon`, `.clock-time`, `.sysinfo-icon`).
- The scale is clamped to `[0.5, 3.0]`. Values outside this range are clamped; `NaN` or infinity falls back to `1.0`.
- Atomic widgets (Stream Deck, Loupedeck) are **not** affected — they use physical device dimensions.

### Per-Widget Override

Individual widgets can override the global scale by setting `scale` directly in their config section:

```toml
[launcher]
scale = 1.5

[mpris]
scale = 1.0  # this widget uses 1.0, not the global 1.5

[clock_widget]
scale = 2.0  # this widget uses 2.0
```

The per-widget value **replaces** the global value (it is not multiplied on top of it).

### CSS Provider Lifecycle

CSS rules are registered exactly once per unique scale value via `register_css_once`. Rebuilding widgets (layout changes, config reloads) does not accumulate
duplicate CSS providers. Per-widget scaling uses a CSS class (e.g. `.scale-150`) applied at `STYLE_PROVIDER_PRIORITY_APPLICATION + 2`, which takes precedence
over the global provider at `APPLICATION + 1`.
