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

| Field              | Type     | Default | Description                        |
|--------------------|----------|---------|------------------------------------|
| `rotation`         | `i32`    | `0`     | Initial rotation (0, 90, 180, 270) |
| `layer`            | `String` | `"top"` | Layer-shell layer                  |
| `namespace`        | `String` | —       | Layer-shell namespace              |
| `exclusive_zone`   | `i32`    | —       | Exclusive zone width               |
| `max_width`        | `i32`    | —       | Maximum window width               |
| `show_decorations` | `bool`   | `false` | Show window decorations            |

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
