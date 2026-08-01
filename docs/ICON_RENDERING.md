# Icon Rendering

This document describes the icon rendering capabilities of each widget, including which configuration fields are available, whether icons are dynamic, and how
dynamic icons are resolved at runtime.

## Configuration Fields

| Field       | Type             | Description                                                          |
|-------------|------------------|----------------------------------------------------------------------|
| `icon`      | `Option<String>` | Optional icon name (Nerd Font or GTK icon theme)                     |
| `icon_size` | `i32`            | Icon size in pixels                                                  |
| `icon_only` | `bool`           | Show only the icon, hide the text label                              |
| `mode`      | `WidgetMode`     | Layout mode: `compact` (vertical) or `wide` (horizontal)             |
| `max_width` | `Option<i32>`    | Maximum widget width in pixels (enforced via CSS `max-width`)        |
| `show_icon` | `bool`           | Whether to show an icon at all (used by sysinfo and voice assistant) |

## Widget Icon Matrix

| Widget                 | `icon` | `icon_size` | `icon_only` | Dynamic Icon | View-dependent | State-dependent | Other Icon Config                                                                                           |
|------------------------|:------:|:-----------:|:-----------:|:------------:|:--------------:|:---------------:|-------------------------------------------------------------------------------------------------------------|
| **app-launcher**       |  yes   |     yes     |     yes     |      no      |       no       |       no        | —                                                                                                           |
| **button**             |  yes   |     yes     |     yes     |     yes      |       no       |       yes       | `state_icon` (via `state_topic`)                                                                            |
| **audio**              |   —    |     yes     |     yes     |     yes      |       no       |       yes       | `mode` (compact/wide), `max_width`                                                                          |
| **mpris**              |   —    |     yes     |     yes     |     yes      |       no       |       yes       | `mode` (compact/wide), `max_width`, album art (via `art_url`)                                               |
| **power**              |   —    |     yes     |     yes     |     yes      |      yes       |       no        | `mode` (compact/wide), `max_width`                                                                          |
| **network**            |   —    |     yes     |     yes     |     yes      |      yes       |       yes       | `mode` (compact/wide), `max_width`, 14 specific icon fields                                                 |
| **workspace-switcher** |   —    |     yes     |     yes     |     yes      |       no       |       yes       | `mode` (compact/wide), `max_width`, `icon_map`, `default_icon`, `show_scrollbar`                            |
| **wallpaper**          |   —    |     yes     |     yes     |     yes      |      yes       |       yes       | `mode` (compact/wide), `max_width`, `fallback_icon`, `show_type_icon`, `preview_image_path`, `preview_icon` |
| **notifications**      |   —    |     yes     |      —      |     yes      |       no       |       yes       | `show_icons` (bool)                                                                                         |
| **voice_assistant**    |   —    |     yes     |      —      |     yes      |       no       |       yes       | 7 state icon fields                                                                                         |
| **sysinfo** (all 7)    |  yes   |     yes     |      —      |      no      |       no       |       no        | —                                                                                                           |
| **sysinfo-multi**      |   —    |     yes     |     yes     |     yes      |      yes       |       yes       | `mode` (compact/wide), `max_width`, `views`                                                                 |
| **clock**              |   —    |     yes     |     yes     |      no      |       no       |       no        | `mode` (compact/wide), `max_width`, time display replaces icon                                              |
| **weather**            |   —    |     yes     |     yes     |     yes      |      yes       |       yes       | —                                                                                                           |

## Dynamic Icon Categories

### View-dependent Icons

Some widgets cycle through multiple views (e.g. via swipe up/down). Each view has its own icon that may be fixed or state-dependent.

- **power**: Each power action (shutdown, reboot, suspend, hibernate, lock, logout, reboot-to-firmware) is a view with a fixed icon. The icon does not change at
  runtime within a view.
- **network**: 7 views (WifiStatus, EthernetStatus, Throughput, WifiScan, Vpn, Airplane, QrCode), each with its own icon. Within the WifiStatus view, the
  signal-strength icon is additionally state-dependent.
- **wallpaper**: Each theme is a view. The preview image or fallback icon comes from theme data (`preview_image_path`, `preview_icon`, `wallpaper_type_icon`).
- **weather**: 15 views (Current, ForecastToday, ForecastTomorrow, Wind, Humidity, UvIndex, Sunrise, Sunset, CloudCover, Sunshine, PrecipitationProbability,
  PrecipitationAmount, Precipitation, AirPollution, Pressure). The Current and Forecast views use a state-dependent icon derived from the WMO weather code. Most
  other views use data-driven icons resolved via the `WidgetIconRendering` trait (see below).
- **sysinfo-multi**: 9 views (Cpu, CpuTemperature, Memory, Battery, Disk, NetworkDownload, NetworkUpload, Uptime, Load), each with its own icon. Within the
  Battery view, the icon is additionally state-dependent (charging/discharging/full). Within the Cpu view, the icon changes based on usage level
  (`nf-md-gauge_empty`,
  `nf-md-gauge_low`, `nf-md-gauge_full` via `UsageLevel::get_icon_name()`). Within the CpuTemperature view, the icon changes based on temperature level
  (`nf-fa-thermometer_empty`, `nf-fa-thermometer_quarter`, `nf-fa-thermometer_half`, `nf-fa-thermometer_full` via `SysinfoTemperatureLevel::get_icon_name()`).

### State-dependent Icons

The icon changes at runtime based on external data (messages from services, system state) without the user switching views.

- **button**: `state_icon` is a conditional expression evaluated against JSON state received via `state_topic`. Supports static names and ternary expressions
  like `{ison?nf-md-fan:nf-md-fan-off}`.
- **audio**: Icon selected from volume level and mute state using Nerd Font icon names (`nf-md-volume_off`, `nf-md-volume_high`, `nf-md-volume_medium`,
  `nf-md-volume_low`, `nf-md-volume_variant_off`). GTK widget resolves icons via `resolve_gtk_nerd_icon` to GResource SVGs (same path as other widgets). Atomic
  widgets resolve icon names to codepoints via `resolve_icon_codepoint`. Supports two layout modes via `WidgetMode`:
    - **Compact** (default): Vertical layout (icon, percentage, device name), matching button/weather alignment. `icon_only` hides text labels.
    - **Wide**: Horizontal layout (icon + info_box with device label and volume bar).
- **mpris**: In Wide mode, album art is loaded from `art_url` in player metadata. Falls back to `audio-x-generic-symbolic` when no art is available. In Compact
  mode, a Nerd Font playback status icon is shown (`nf-fa-play`,
  `nf-fa-pause`), resolved via `resolve_gtk_nerd_icon` to GResource SVGs. Falls back to `nf-fa-music` when no player is active. Atomic widgets use Nerd Font
  icon names resolved via `resolve_icon_codepoint`. Supports two layout modes via `WidgetMode`:
    - **Compact** (default): Vertical layout (playback icon, title, artist), matching button/weather alignment. `icon_only` hides text labels.
    - **Wide**: Horizontal layout (album art + info_box with title, artist, and progress bar).
- **network**: Within WifiStatus view, the signal-strength icon changes based on actual signal level (strength 1–4 or off). Configurable via 14 specific icon
  fields (`icon_wifi_strength_1`–`icon_wifi_strength_4`, `icon_wifi_strength_off`,
  `icon_ethernet_on`, `icon_ethernet_off`, `icon_vpn_on`, `icon_vpn_off`,
  `icon_airplane_on`, `icon_airplane_off`, `icon_throughput`, `icon_wifi_scan`,
  `icon_qr_code`).
- **workspace-switcher**: Workspace ID is looked up in `icon_map`. Falls back to
  `default_icon` when the workspace ID is not in the map.
- **wallpaper**: Preview image or icon comes from the selected theme's data. Falls back to `fallback_icon` when no preview is available.
- **notifications**: Each notification carries its own icon (`notification.icon`). Falls back to `dialog-information-symbolic`.
- **voice_assistant**: 7 dedicated state icon fields (`icon_idle`,
  `icon_listening`, `icon_processing`, `icon_thinking`, `icon_executing`,
  `icon_speaking`, `icon_error`). The active icon is selected based on the assistant's current state.
- **weather**: Within Current/Forecast views, the icon is derived from the WMO weather code via `weather_code_icon_day_night(code, is_day)`. Day/night
  distinction is applied for the Current view. Other views use data-driven icons via the `WidgetIconRendering` trait:
    - **Wind**: 8-direction compass icons (`nf-weather-wind_north`,
      `nf-weather-wind_north_east`, etc.) selected from wind direction. Info text shows the compass abbreviation (N, NE, E, ...).
    - **Humidity**: `nf-md-water_outline` (VeryDry), `nf-md-water_check`
      (Comfortable), `nf-md-water` (High), `nf-md-water_alert` (Muggy).
    - **Temperature**: `nf-fa-temperature_empty` (Freezing),
      `nf-fa-temperature_quarter` (Cold), `nf-fa-temperature_half` (Cool),
      `nf-fa-temperature_three_quarters` (Pleasant), `nf-fa-temperature_full`
      (Warm), `nf-fa-temperature_high` (Hot).
    - **UV Index**: `nf-weather-day_sunny` (fixed icon, color varies by level).
    - **PrecipitationIntensity**: `nf-fa-hotjar` (Dry), `nf-weather-sprinkle`
      (Light), `nf-weather-rain` (Moderate), `nf-weather-rain_wind` (Heavy),
      `nf-weather-storm_showers` (Extreme).
    - **PrecipitationAmountLevel**: `nf-fa-hotjar` (None), `nf-weather-sprinkle`
      (LightDrops), `nf-weather-rain` (Moderate), `nf-weather-showers` (Heavy).
        - **CloudCover, Sunshine, Pressure, AirPollution**: Icons and colors derived from their respective level enums via `WidgetIconRendering`.
        - Semantic icon coloring is applied via CSS classes mapped from `Color`
          via `Color::css_class()` (e.g. `icon-color-dark-blue`, `icon-color-blue`,
          `icon-color-orange`, `icon-color-red`, etc.). See the Semantic Icon Coloring section below for the full mapping.

## Widgets Without Dynamic Icons

- **app-launcher**: Icon is resolved once at startup from the `icon` config field or the `.desktop` file's `Icon` entry. Does not change at runtime.
- **sysinfo** (all 7 sub-widgets): Icon is a static config value (`icon` field). Does not change at runtime. The `show_icon` flag controls visibility.
- **clock**: No icon support.
- **weather** (Sunrise, Sunset views): Fixed icons per view, not state-dependent.

> **Note:** The **sysinfo-multi** widget (multi-view) has dynamic, view-dependent, and state-dependent icons — see the Widget Icon Matrix above. The
> single-metric
> sysinfo widgets listed here are the original static widgets.

## `WidgetMode` (Layout Modes)

The `WidgetMode` enum (`plugin-api/src/widget/mode.rs`) provides layout modes for widgets that support both compact and wide presentations. It is serialized as
lowercase strings (`compact`, `wide`) in TOML config.

- **Compact** (default): Vertical layout — icon on top, `main_text` and
  `info_text` below. Matches the layout of button and weather widgets, ensuring icons align on the same horizontal line across widgets.
- **Wide**: Horizontal layout — icon on the left, info panels (volume bar, device label) on the right.

Currently used by: **audio**, **mpris**, **power**, **network**, **wallpaper**, **clock**, **workspace-switcher**, and **sysinfo-multi** widgets. `icon_only`
only affects Compact mode.

### Unified 4-Line Layout

All widgets (button, weather, audio, mpris, power, network, wallpaper, clock, workspace-switcher, sysinfo-multi) use the same vertical structure in their inner
content box, ensuring consistent icon alignment and total height across widgets:

| Line | Height      | Button             | Weather            | Audio (Wide)                | MPRIS (Wide)                | Power (Wide)                   | Network                    | Wallpaper                   | Clock                        | WorkspaceSwitcher          | Sysinfo Multi              |
|------|-------------|--------------------|--------------------|-----------------------------|-----------------------------|--------------------------------|----------------------------|-----------------------------|------------------------------|----------------------------|----------------------------|
| 0    | `icon_size` | Icon               | Icon               | Icon                        | Album Art                   | Icon                           | Icon                       | Preview/Fallback            | Time (text)                  | Icon                       | Icon                       |
| 1    | 20px        | `widget-main-text` | `widget-main-text` | `widget-main-text` (device) | `widget-main-text` (title)  | `widget-main-text` (action)    | `widget-main-text` (value) | `widget-main-text` (theme)  | `widget-main-text` (date)    | `widget-main-text` (name)  | `widget-main-text` (value) |
| 2    | 16px        | `widget-info-text` | `widget-info-text` | `widget-info-text` (empty)  | `widget-info-text` (artist) | `widget-info-text` (countdown) | `widget-info-text` (info)  | `widget-info-text` (status) | `widget-info-text` (weekday) | `widget-info-text` (index) | `widget-info-text` (label) |
| 3    | 16px        | spacer             | spacer             | volume bar                  | progress bar                | timeout bar                    | spacer/QR                  | spacer                      | spacer                       | scrollbar                  | `LevelBar` or spacer       |

In Compact mode with `icon_only = true`, lines 1–3 are empty but retain their `height_request` to preserve icon alignment.

## `max_width` (Maximum Widget Width)

The `max_width` field (`Option<i32>`, part of `WidgetDimensions`) caps the widget's rendered width. When set, the widget's effective width is
`min(width, max_width)`, and a CSS `max-width` rule is dynamically injected to enforce the limit hard (GTK's `width_request` is only a minimum). Additionally,
`hexpand(false)` and `halign(Start)` are set to prevent the widget from expanding into available space.

When `max_width` is `None` (not set), the widget uses mode-dependent defaults via `max_width_or_default(mode)`:

- **Compact mode**: `DEFAULT_WIDGET_WIDTH` (100px)
- **Wide mode**: `DEFAULT_WIDE_MODE_WIDGET_WIDTH` (300px)

These defaults are **not** enforced via CSS — they only serve as fallbacks for `width_request` and internal calculations (e.g. progress/volume bar width). Hard
enforcement only happens when `max_width` is explicitly set in config.

Currently used by: **audio**, **mpris**, **power**, **network**, **wallpaper**, **clock**, and **workspace-switcher** widgets.

## Icon Rendering Architecture

### `ViewData` Struct

The `ViewData` struct (`plugin-api/src/widget/view_data.rs`) is the generic data container for view rendering across GTK, graphic, atomic, and HTML renderers.
It contains:

- `icon_name: String` — Nerd Font icon name (e.g. `nf-fa-volume_up`)
- `main_text: String` — primary text line
- `info_text: String` — secondary text line
- `icon_color: Option<Color>` — optional semantic icon color
- `is_error: bool` — whether this represents an error/loading state

Constructors: `new()`, `with_color()`, `error()`, and `TypedBuilder` pattern (`ViewData::builder().icon_name(...).main_text(...).info_text(...).build()`).

Used by: weather widget (`render_view`, `render_view_graphic`,
`render_view_html`, `render_atomic_view`), audio atomic (`render_atomic_view`), mpris atomic (`render_atomic_view`).

### `AtomicGraphicData` Struct

The `AtomicGraphicData` struct (`plugin-api/src/atomic/graphic_data.rs`) is the data container for pixel-based atomic widget rendering. It contains:

- `icon_char: char` — resolved Nerd Font codepoint
- `main_text: String` — primary text line
- `info_text: String` — secondary text line
- `is_error: bool` — whether this represents an error/loading state
- `icon_color: Option<[u8; 4]>` — optional RGBA icon color

Constructors: `new()`, `with_color()`, `error()`.

Used by: all atomic widgets via `render_atomic_graphic_data()`. The macro
`atomic_widget_impl!` destructures this struct and passes fields to
`render_atomic_graphic_default()`.

### `WidgetIconRendering` Trait

The `WidgetIconRendering` trait (`plugin-api/src/widget/icon.rs`) provides data-driven icon resolution for weather level enums:

- `get_icon_name() -> Option<String>` — returns a Nerd Font icon name
- `get_icon_color() -> Option<Color>` — returns a semantic color
- `get_icon_name_or_default(&str) -> String` — default method that returns
  `get_icon_name()` or the provided fallback string

Implemented by: `TemperatureLevel`, `HumidityLevel`, `WindDirection`,
`PrecipitationIntensity`, `UvIndexLevel`, `CloudCoverLevel`, `PressureLevel`,
`AirQualityLevel`, `SunshineLevel`, `PrecipitationProbabilityLevel`,
`PrecipitationAmountLevel`, `WindSpeedLevel`.

### Icon Resolution Pipeline

1. **Icon name** (e.g. `nf-fa-volume_up`) stored in `ViewData.icon_name`
2. **GTK path**: `resolve_gtk_nerd_icon()` normalizes the name to GTK symbolic form (e.g. `nf-fa-volume-up-symbolic`). The caller constructs the full GResource
   path (`/com/nerd/icons/nf-fa-volume-up-symbolic.svg`) and loads it via
   `Image::from_resource()`, falling back to `Image::from_icon_name()` if the resource is not found.
3. **Pixel path**: `resolve_icon_codepoint()` looks up the Unicode codepoint in `nerd_gtk_icons::codepoint_map::ICONS` and stores it in
   `AtomicGraphicData.icon_char`

### Semantic Icon Coloring

Weather icons can have semantic colors applied via CSS classes. The `Color`
struct in `plugin-api/src/widget/icon.rs` provides named color constants and a
`css_class()` method that maps each color to a CSS class name. CSS classes are defined in `resources/style.css`. For pixel-based rendering, `Color::to_rgba()`
converts to `[u8; 4]` for direct color application.

| Color Constant       | CSS Class                |
|----------------------|--------------------------|
| `Color::GREEN`       | `icon-color-green`       |
| `Color::LIGHT_GREEN` | `icon-color-light-green` |
| `Color::YELLOW`      | `icon-color-yellow`      |
| `Color::ORANGE`      | `icon-color-orange`      |
| `Color::RED`         | `icon-color-red`         |
| `Color::DARK_RED`    | `icon-color-dark-red`    |
| `Color::DARK_BLUE`   | `icon-color-dark-blue`   |
| `Color::BLUE`        | `icon-color-blue`        |
| `Color::LIGHT_BLUE`  | `icon-color-light-blue`  |
| `Color::BLACK`       | `icon-color-black`       |
| `Color::WHITE`       | `icon-color-white`       |
| (other)              | `icon-color-default`     |

## Resolved Inconsistencies

The following inconsistencies have been addressed:

- **`icon_size` for `wallpaper`, `notifications`, and `weather`**: These widgets previously rendered dynamic icons without exposing an `icon_size`
  configuration field. Icon size was either hardcoded (`48` for wallpaper,
  `16` for notifications) or derived from widget dimensions (`(min(w, h) *
  0.5).min(40)` for weather). All three now expose `icon_size` using the centralized `DEFAULT_ICON_SIZE` (36px) from `plugin-api`.
- **Centralized `DEFAULT_ICON_SIZE`**: All GTK-based widgets now import
  `DEFAULT_ICON_SIZE` from `smearor_swipe_launcher_plugin_api` instead of defining their own local constants. This ensures consistent default icon sizes across
  all widgets.
- **Atomic widgets**: `AtomicWidgetConfig` gained an optional `icon_size`
  field for headless graphic rendering. When unset, the icon size is derived from the physical button dimensions (`(min(width, height) * 0.5).min(40)`)
  to ensure room for `main_text` and `info_text`. This is intentionally **not** based on `DEFAULT_ICON_SIZE` because atomic widgets must share button space with
  text labels.

## Remaining Inconsistencies

The following inconsistencies still exist across widgets and may be candidates for further unification:

- **`show_icon` vs `icon`**: `sysinfo` and `voice_assistant` use a `show_icon`
  boolean to toggle icon visibility, while other widgets rely on the presence or absence of the `icon` field.
- **State icon mechanisms**: `button` uses a generic `state_icon` expression evaluated against arbitrary JSON state. `voice_assistant` uses 7 hardcoded
  state-specific fields. `network` uses 14 view/state-specific fields. A unified state-expression mechanism could replace all three approaches.

## `icon_only` Support

The `icon_only` flag (default: `false`, centralized via `DEFAULT_ICON_ONLY` in
`plugin-api`) hides all text labels and shows only the icon. When `icon` is
`None` or empty, text labels are shown regardless of `icon_only`.

### Widgets with `icon_only`

All widgets with `icon_only` use the centralized `WidgetIcon` struct from
`plugin-api` (flattened via `#[serde(flatten)]`), which bundles `icon_size`
and `icon_only` with defaults from `DEFAULT_ICON_SIZE` and `DEFAULT_ICON_ONLY`.

| Widget                 |       Config Field        | Behavior                                                 |
|------------------------|:-------------------------:|----------------------------------------------------------|
| **button**             | `icon_config: WidgetIcon` | Hides `main_text` and `info_text` labels                 |
| **app-launcher**       | `icon_config: WidgetIcon` | Hides the app name label                                 |
| **weather**            | `icon_config: WidgetIcon` | Hides `temp_label` and `info_label`                      |
| **audio**              | `icon_config: WidgetIcon` | Hides `main_label` and `info_label` (compact mode only)  |
| **mpris**              | `icon_config: WidgetIcon` | Hides `main_label` and `info_label` (compact mode only)  |
| **power**              | `icon_config: WidgetIcon` | Hides `main_label` and `info_label` (compact mode only)  |
| **network**            | `icon_config: WidgetIcon` | Hides `value_label` and `info_label`                     |
| **sysinfo-multi**      | `icon_config: WidgetIcon` | Hides `value_label` and `info_label` (compact mode only) |
| **wallpaper**          | `icon_config: WidgetIcon` | Hides `theme_label` and `status_label`                   |
| **clock**              | `icon_config: WidgetIcon` | Hides `date_label` and `weekday_label`                   |
| **workspace-switcher** | `icon_config: WidgetIcon` | Hides `main_label` and `info_label` (compact mode only)  |

### Widgets where `icon_only` would be sensible

These widgets render an icon plus text labels but do not yet support `icon_only`:

| Widget | Labels Hidden | Notes |
|--------|---------------|-------|
| (none) | —             | —     |

### Widgets where `icon_only` is not applicable

These widgets already have an inverse `show_icon` flag, use the icon as a secondary element, or have dynamic content that doesn't benefit from icon-only mode:

- **sysinfo**: Uses `show_icon` (inverse semantics). (Applies to the original single-metric widgets only; the **sysinfo-multi** widget supports `icon_only`.)
- **voice_assistant**: Uses `show_icon` (inverse semantics)
- **notifications**: Dynamic content, icon is a header element
- **wallpaper**: Preview image is the primary element, icon is fallback only
