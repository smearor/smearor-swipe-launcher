# Concept: Sysinfo Multi-View Widget

This document describes the concept for a **Multi-View Sysinfo Widget** in the *Smearor Swipe Launcher*. The current Sysinfo plugin consists of separate
single-metric widgets (CPU, Memory, Battery, Disk, Temperature, Network, Uptime) that each occupy a tile. This concept introduces a single **compact, view-based
tile** that cycles through all system metrics via swipe gestures — mirroring the Network Widget's multi-view pattern.

The existing `model/sysinfo` and `services/sysinfo` crates remain unchanged. Only the widget crate (`plugins/sysinfo`) is extended with a new multi-view widget
type.

---

## 1. Motivation

### 1.1 Current State

The Sysinfo plugin currently exposes **seven independent GTK widgets** and **nine atomic widget variants**, each rendering a single metric:

| Widget Type               | Metric             | Rendering Path       |
|---------------------------|--------------------|----------------------|
| `cpu`                     | CPU usage %        | GTK (Bar/Gauge)      |
| `memory`                  | Memory usage %     | GTK (Bar/Gauge)      |
| `battery`                 | Battery level %    | GTK (Bar/Gauge)      |
| `disks`                   | Disk usage %       | GTK (Bar/Gauge)      |
| `network`                 | Network throughput | GTK (text)           |
| `temperature`             | CPU temperature    | GTK (text)           |
| `uptime`                  | System uptime      | GTK (text)           |
| `sysinfo_cpu` (atomic)    | CPU usage %        | Headless / Web / GTK |
| `sysinfo_cpu_temperature` | CPU temperature    | Headless / Web / GTK |
| ...                       | ...                | ...                  |

Each widget subscribes to its own topic and renders independently. There is no view-switching mechanism — each metric requires a separate tile in the launcher
layout.

### 1.2 What Is Missing

- A **single tile** that can display all system metrics by cycling through views.
- **Swipe Up / Swipe Down** gesture support to switch between metrics (like the Network Widget).
- A unified `SysinfoView` enum in the model crate for view selection.
- A `SysinfoMultiWidget` struct that aggregates all status message types and renders the current view.
- Headless (GraphicRenderer) and Web (WebRenderer) support for the multi-view widget.
- Semantic icon colors per view (already implemented via `UsageLevel`, `BatteryLevel`, `SysinfoTemperatureLevel`).

### 1.3 Comparison: Current vs Multi-View

| Aspect             | Current (Separate Widgets)          | Multi-View Widget                          |
|--------------------|-------------------------------------|--------------------------------------------|
| Tiles required     | One per metric (up to 9)            | Single tile for all metrics                |
| Navigation         | None (each widget is static)        | Swipe Up / Swipe Down to cycle views       |
| Config             | One widget config per metric        | Single config with `views` list            |
| Data sources       | Each widget subscribes to one topic | Widget subscribes to all sysinfo topics    |
| Semantic colors    | Per widget (already implemented)    | Per view (reuses existing level enums)     |
| GTK rendering      | Bar/Gauge/Text per widget           | Icon + text labels (like Network Widget)   |
| Headless rendering | Per atomic widget                   | Single GraphicRenderer with view switching |
| Web rendering      | Per atomic widget                   | Single WebRenderer with view switching     |

---

## 2. Goals

- Provide a single `SysinfoMultiWidget` that cycles through configurable system metric views.
- Support **Swipe Up / Swipe Down** (and Scroll Up / Scroll Down) for view navigation.
- Support **all three rendering paths**: GTK, Headless (GraphicRenderer), and Web (WebRenderer).
- Reuse existing semantic color enums (`UsageLevel`, `BatteryLevel`, `SysinfoTemperatureLevel`).
- Allow users to configure which views to include and in what order via TOML `views` list.
- Follow the **Unified 4-Line Layout** (see `docs/ICON_RENDERING.md`) for consistent icon alignment and widget height across all widgets.
- Support `WidgetMode` (compact/wide) and `icon_only` via `WidgetIcon`.
- Keep existing single-metric widgets unchanged — the multi-view widget is an addition, not a replacement.

## 3. Non-Goals

- Removing or deprecating the existing single-metric widgets.
- Adding new sysinfo metrics (e.g. GPU, fan speed) — those are separate features.
- Supporting per-view display modes (Bar/Gauge) — the multi-view widget uses icon + text labels only.
- Supporting interactive actions (e.g. killing processes) — this is a display-only widget.

---

## 4. Architecture

### 4.1 System Architecture & Data Flow

```
+----------------------------+                +----------------------------+
| Sysinfo Multi-View Widget  |                | Sysinfo Service            |
| (subscribed to all         |                | (Singleton, unchanged)     |
|  service.sysinfo.* topics) |                |                            |
+----------------------------+                +----------------------------+
             |                                              |
             |  1. Status Broadcasts (all topics)            |
             | <============================================|
             |     Topic: "service.sysinfo.cpu"             |
             |     Payload: CpuStatusMessage                |
             |     Topic: "service.sysinfo.memory"          |
             |     Payload: MemoryStatusMessage             |
             |     Topic: "service.sysinfo.battery"         |
             |     Payload: BatteryStatusMessage            |
             |     Topic: "service.sysinfo.disks"           |
             |     Payload: DisksStatusMessage               |
             |     Topic: "service.sysinfo.network"         |
             |     Payload: NetworkStatusMessage            |
             |     Topic: "service.sysinfo.uptime"          |
             |     Payload: UptimeStatusMessage              |
             |                                              |
+------------+--------------+                                 |
| View Engine               |                                 |
| - current_view index      |                                 |
| - swipe up/down cycling   |                                 |
| - render_view() dispatch  |                                 |
| - semantic icon colors   |                                 |
+---------------------------+
            |
            v
+----------------------------+
| Render Output              |
| - GTK: icon + text labels  |
| - Headless: FfiGraphic     |
| - Web: HTML string         |
+----------------------------+
```

### 4.2 View Model

A new `SysinfoView` enum is added to `model/sysinfo` to define the available views. This mirrors `NetworkView` in `model/network`.

```rust
/// Available sysinfo views that the multi-view widget can display.
/// Each variant corresponds to a system metric rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum SysinfoView {
    /// CPU usage percentage.
    #[default]
    Cpu,
    /// CPU temperature.
    CpuTemperature,
    /// Memory usage percentage.
    Memory,
    /// Battery level percentage and charging state.
    Battery,
    /// Disk usage percentage (first mount or root).
    Disk,
    /// Network download throughput.
    NetworkDownload,
    /// Network upload throughput.
    NetworkUpload,
    /// System uptime.
    Uptime,
    /// 1-minute load average.
    Load,
}
```

The enum derives `Serialize` and `Deserialize` for TOML config support, and `Default` (defaults to `Cpu`). It follows the same pattern as `NetworkView`.

### 4.3 Unified 4-Line Layout

The widget follows the **Unified 4-Line Layout** defined in `docs/ICON_RENDERING.md`, ensuring consistent icon alignment and total height across all widgets:

| Line | Height      | Sysinfo Multi-View (Compact)            | Sysinfo Multi-View (Wide)               |
|------|-------------|-----------------------------------------|-----------------------------------------|
| 0    | `icon_size` | Icon                                    | Icon                                    |
| 1    | 20px        | `widget-main-text` (value, e.g. "42%")  | `widget-main-text` (value, e.g. "42%")  |
| 2    | 16px        | `widget-info-text` (metric label)       | `widget-info-text` (metric label)       |
| 3    | 16px        | `LevelBar` (percentage views) or spacer | `LevelBar` (percentage views) or spacer |

For **percentage-based views** (Cpu, Memory, Battery, Disk, Load), Line 3 contains a `LevelBar` (progress bar) instead of a spacer — mirroring the existing
single-metric GTK widgets which draw a Bar or Gauge. For **non-percentage views** (CpuTemperature, NetworkDownload, NetworkUpload, Uptime), Line 3 remains a
spacer.

This matches the pattern used by the **audio** widget (volume bar in Line 3) and the **power** widget (timeout bar in Line 3).

In Compact mode with `icon_only = true`, lines 1–3 are empty/hidden but retain their `height_request` to preserve icon alignment.

### 4.4 `WidgetMode` Support

The widget supports two layout modes via `WidgetMode` (`plugin-api/src/widget/mode.rs`):

- **Compact** (default): Vertical layout — icon on top, `main_text` and `info_text` below. Matches the layout of button, weather, network, and other widgets,
  ensuring icons align on the same horizontal line across widgets.
- **Wide**: Horizontal layout — icon on the left, `main_text` and `info_text` on the right. Useful for wider tiles (e.g. scroll band).

`icon_only` only affects Compact mode — in Wide mode, text labels are always shown.

### 4.5 Progress Bar (Line 3)

The existing single-metric GTK widgets support two display modes: `DisplayMode::Bar` (using `LevelBar`) and `DisplayMode::Gauge` (using `DrawingArea` with a
circular gauge). The multi-view widget replaces the Gauge with a simpler **horizontal `LevelBar`** in Line 3 of the 4-line layout, since the compact tile size
does not accommodate a circular gauge overlay.

The `LevelBar` is shown only for **percentage-based views** — views that have a natural 0–100% range:

| View              | Has Progress Bar | Value Source                          |
|-------------------|:----------------:|---------------------------------------|
| `Cpu`             |       yes        | `cpu_usage` (0–100%)                  |
| `CpuTemperature`  |        no        | —                                     |
| `Memory`          |       yes        | `memory_usage` (0–100%)               |
| `Battery`         |       yes        | `level` (0–100%)                      |
| `Disk`            |       yes        | `mounts[0].usage` (0–100%)            |
| `NetworkDownload` |        no        | —                                     |
| `NetworkUpload`   |        no        | —                                     |
| `Uptime`          |        no        | —                                     |
| `Load`            |       yes        | `load_average_1m / num_cpus` (0–100%) |

The `LevelBar` uses the same CSS classes as the existing sysinfo widgets (`sysinfo-bar`, `sysinfo-normal`, `sysinfo-warning`, `sysinfo-critical`) for consistent
color thresholds. The CSS class is updated dynamically based on the current value:

```rust
fn update_bar_css(bar: &LevelBar, value: f32, warning: f32, critical: f32) {
    bar.remove_css_class("sysinfo-normal");
    bar.remove_css_class("sysinfo-warning");
    bar.remove_css_class("sysinfo-critical");
    let class = if value >= critical {
        "sysinfo-critical"
    } else if value >= warning {
        "sysinfo-warning"
    } else {
        "sysinfo-normal"
    };
    bar.add_css_class(class);
}
```

When switching to a non-percentage view, the `LevelBar` is hidden (`set_visible(false)`) and the spacer is shown instead. When switching to a percentage view,
the spacer is hidden and the `LevelBar` is shown.

### 4.6 Color Priority Model

The multi-view widget reuses the existing semantic color system:

```
1. Error state          → error color (red/white, hardcoded)
2. Semantic color       → WidgetIconRendering::get_icon_color() (runtime)
3. Configured color     → WidgetIcon.icon_color (from TOML)
4. Default text color   → white or theme-dependent
```

Each view maps to a semantic level enum:

| View              | Level Enum                | Color Logic                                  | State-dependent Icon |
|-------------------|---------------------------|----------------------------------------------|----------------------|
| `Cpu`             | `UsageLevel`              | Green (<50%) → Yellow (<75%) → Orange → Red  | yes (gauge icons)    |
| `CpuTemperature`  | `SysinfoTemperatureLevel` | Blue (<40°C) → Green → Yellow → Orange → Red | yes (thermometer)    |
| `Memory`          | `UsageLevel`              | Same as CPU                                  | no                   |
| `Battery`         | `BatteryLevel`            | Red (<15%) → Orange → Yellow → Green         | yes (charging state) |
| `Disk`            | `UsageLevel`              | Same as CPU                                  | no                   |
| `NetworkDownload` | None                      | Default text color                           | no                   |
| `NetworkUpload`   | None                      | Default text color                           | no                   |
| `Uptime`          | None                      | Default text color                           | no                   |
| `Load`            | `UsageLevel`              | Based on load average percentage             | no                   |

### 4.7 State-dependent Icons

Two views have **state-dependent icons** that change based on the current metric value, in addition to semantic colors:

**Cpu view** — icon depends on CPU usage level (`UsageLevel`):

| Usage Level | Icon                | Condition |
|-------------|---------------------|-----------|
| Low         | `nf-md-gauge_empty` | 0–49%     |
| Moderate    | `nf-md-gauge_low`   | 50–74%    |
| High        | `nf-md-gauge_full`  | 75–100%   |

This requires updating `UsageLevel::get_icon_name()` in `model/sysinfo/src/model/usage_level.rs` to return the gauge icon instead of `None`.

**CpuTemperature view** — icon depends on temperature level (`SysinfoTemperatureLevel`):

| Temperature Level | Icon                        | Condition |
|-------------------|-----------------------------|-----------|
| Cool              | `nf-fa-thermometer_empty`   | <40°C     |
| Normal            | `nf-fa-thermometer_quarter` | 40–59°C   |
| Warm              | `nf-fa-thermometer_half`    | 60–74°C   |
| Hot               | `nf-fa-thermometer_full`    | 75–100°C  |
| Critical          | `nf-fa-thermometer_full`    | >85°C     |

This requires updating `SysinfoTemperatureLevel::get_icon_name()` in `model/sysinfo/src/model/temperature_level.rs` to return the thermometer icon instead of
`None`.

> **Note:** The `Critical` temperature level reuses `nf-fa-thermometer_full` since there is no higher variant. The semantic color (red) distinguishes it from
> `Hot` (orange).

> **Note:** These changes to `get_icon_name()` also benefit the existing atomic widgets, which already call `get_icon_name_or_default()` on the level enums.
> Currently they fall back to a hardcoded default icon; after this change, the level enum provides the state-dependent icon directly.

---

## 5. Implementation Plan

### Phase 1: Model (`model/sysinfo`)

#### 5.1 New File: `model/sysinfo/src/messages/view.rs`

```rust
use serde::Deserialize;
use serde::Serialize;

/// Available sysinfo views that the multi-view widget can display.
/// Each variant corresponds to a system metric rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum SysinfoView {
    /// CPU usage percentage.
    #[default]
    Cpu,
    /// CPU temperature.
    CpuTemperature,
    /// Memory usage percentage.
    Memory,
    /// Battery level percentage and charging state.
    Battery,
    /// Disk usage percentage (first mount or root).
    Disk,
    /// Network download throughput.
    NetworkDownload,
    /// Network upload throughput.
    NetworkUpload,
    /// System uptime.
    Uptime,
    /// 1-minute load average.
    Load,
}
```

#### 5.2 Update: `model/sysinfo/src/messages/mod.rs`

Add `pub mod view;` declaration.

#### 5.3 Update: `model/sysinfo/src/lib.rs`

Add `pub use messages::view::SysinfoView;` re-export.

#### 5.4 Update: `model/sysinfo/src/model/usage_level.rs`

Update `UsageLevel::get_icon_name()` to return state-dependent gauge icons:

```rust
impl WidgetIconRendering for UsageLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Low => Color::GREEN,
            Self::Moderate => Color::YELLOW,
            Self::High => Color::ORANGE,
            Self::Critical => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        let icon = match self {
            Self::Low => "nf-md-gauge_empty",
            Self::Moderate => "nf-md-gauge_low",
            Self::High => "nf-md-gauge_full",
            Self::Critical => "nf-md-gauge_full",
        };
        Some(icon.to_string())
    }
}
```

#### 5.5 Update: `model/sysinfo/src/model/temperature_level.rs`

Update `SysinfoTemperatureLevel::get_icon_name()` to return state-dependent thermometer icons:

```rust
impl WidgetIconRendering for SysinfoTemperatureLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Cool => Color::BLUE,
            Self::Normal => Color::GREEN,
            Self::Warm => Color::YELLOW,
            Self::Hot => Color::ORANGE,
            Self::Critical => Color::RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        let icon = match self {
            Self::Cool => "nf-fa-thermometer_empty",
            Self::Normal => "nf-fa-thermometer_quarter",
            Self::Warm => "nf-fa-thermometer_half",
            Self::Hot => "nf-fa-thermometer_full",
            Self::Critical => "nf-fa-thermometer_full",
        };
        Some(icon.to_string())
    }
}
```

#### 5.6 Exit Criteria

- `SysinfoView` enum is public and serializable.
- `UsageLevel::get_icon_name()` returns gauge icons.
- `SysinfoTemperatureLevel::get_icon_name()` returns thermometer icons.
- `cargo build -p smearor-sysinfo-model` succeeds.

---

### Phase 2: Widget Config (`plugins/sysinfo/src/config.rs`)

#### 5.7 New Config Struct

A new `SysinfoMultiWidgetConfig` struct is added to `config.rs`:

```rust
/// Configuration for the sysinfo multi-view widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SysinfoMultiWidgetConfig {
    /// Widget dimensions (width, height, max_width) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Icon configuration (size, color, icon_only, mode).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Layout mode: compact (vertical) or wide (horizontal).
    pub mode: WidgetMode,
    /// Views to cycle through on swipe up/down.
    pub views: Vec<SysinfoView>,
    /// Action bindings for gestures.
    pub actions: ActionBindings,
}

impl Default for SysinfoMultiWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_config: WidgetIcon::default(),
            mode: WidgetMode::default(),
            views: vec![
                SysinfoView::Cpu,
                SysinfoView::CpuTemperature,
                SysinfoView::Memory,
                SysinfoView::Battery,
                SysinfoView::Disk,
                SysinfoView::NetworkDownload,
                SysinfoView::NetworkUpload,
                SysinfoView::Uptime,
                SysinfoView::Load,
            ],
            actions: ActionBindings::default(),
        }
    }
}
```

The `WidgetIcon` struct (flattened via `#[serde(flatten)]`) provides `icon_size`, `icon_only`, and `icon_color` fields. The `WidgetMode` field provides
`compact`/`wide` layout selection. `WidgetDimensions` provides `width`, `height`, and `max_width`.

#### 5.8 Exit Criteria

- Config struct parses from TOML with `serde_json::from_value`.
- Default views include all 9 metrics in a sensible order.

---

### Phase 3: GTK Widget (`plugins/sysinfo/src/multi_widget.rs`)

#### 5.9 Widget Struct

```rust
/// Multi-view sysinfo widget that cycles through system metrics.
///
/// Subscribes to all sysinfo status topics and renders the current view.
/// Swipe Up / Swipe Down cycles through configured views.
pub struct SysinfoMultiWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: SysinfoMultiWidgetConfig,
    pub icon_image: SharedImage,
    pub value_label: SharedLabel,
    pub info_label: SharedLabel,
    pub level_bar: Rc<RefCell<Option<LevelBar>>>,
    pub spacer_label: SharedLabel,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_cpu: Rc<RefCell<Option<CpuStatusMessage>>>,
    pub latest_memory: Rc<RefCell<Option<MemoryStatusMessage>>>,
    pub latest_battery: Rc<RefCell<Option<BatteryStatusMessage>>>,
    pub latest_disks: Rc<RefCell<Option<DisksStatusMessage>>>,
    pub latest_network: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_uptime: Rc<RefCell<Option<UptimeStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}
```

#### 5.10 Trait Implementations

- `MessageHandler<CpuStatusMessage>` — stores latest CPU status, triggers `update_ui()`
- `MessageHandler<MemoryStatusMessage>` — stores latest memory status, triggers `update_ui()`
- `MessageHandler<BatteryStatusMessage>` — stores latest battery status, triggers `update_ui()`
- `MessageHandler<DisksStatusMessage>` — stores latest disk status, triggers `update_ui()`
- `MessageHandler<NetworkStatusMessage>` — stores latest network status, triggers `update_ui()`
- `MessageHandler<UptimeStatusMessage>` — stores latest uptime status, triggers `update_ui()`
- `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` — updates locale/units
- `MessageBroadcaster` — provides broadcaster for topic subscription
- `PluginMetaGetter` — returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` — provides access to core context
- `WidgetBuilder` — builds the GTK4 widget UI
- `DefaultFallback` — gesture handling for view switching

#### 5.11 View Navigation

| Gesture     | Action                          |
|-------------|---------------------------------|
| Swipe Up    | Advance to next view            |
| Swipe Down  | Go to previous view             |
| Scroll Up   | Advance to next view            |
| Scroll Down | Go to previous view             |
| Click       | No action (display-only widget) |
| Long-press  | No action (display-only widget) |

```rust
fn next_view(&self) {
    let current_view = self.current_view.clone();
    let latest_cpu = self.latest_cpu.clone();
    let latest_memory = self.latest_memory.clone();
    let latest_battery = self.latest_battery.clone();
    let latest_disks = self.latest_disks.clone();
    let latest_network = self.latest_network.clone();
    let latest_uptime = self.latest_uptime.clone();
    let icon_image = self.icon_image.clone();
    let value_label = self.value_label.clone();
    let info_label = self.info_label.clone();
    let config = self.config.clone();
    let personalization = self.personalization.borrow().clone();

    MainContext::default().spawn_local(async move {
        if config.views.is_empty() {
            return;
        }
        let mut idx = current_view.borrow_mut();
        *idx = (*idx + 1) % config.views.len();
        let view = config.views[*idx];
        drop(idx);

        let view_data = render_view(
            view,
            &latest_cpu.borrow(),
            &latest_memory.borrow(),
            &latest_battery.borrow(),
            &latest_disks.borrow(),
            &latest_network.borrow(),
            &latest_uptime.borrow(),
            &personalization,
        );

        if let Some(ref img) = *icon_image.borrow() {
            update_icon_display(img, &view_data, config.icon_config.icon_size(), config.icon_config.icon_color());
        }
        if let Some(ref label) = *value_label.borrow() {
            label.set_text(&view_data.main_text);
        }
        if let Some(ref label) = *info_label.borrow() {
            label.set_text(&view_data.info_text);
        }
    });
}
```

#### 5.12 Render View Function

A shared `render_view` function constructs `ViewData` for each view, applying semantic colors:

```rust
/// Renders the display data for a given sysinfo view.
#[allow(clippy::too_many_arguments)]
fn render_view(
    view: SysinfoView,
    cpu: &Option<CpuStatusMessage>,
    memory: &Option<MemoryStatusMessage>,
    battery: &Option<BatteryStatusMessage>,
    disks: &Option<DisksStatusMessage>,
    network: &Option<NetworkStatusMessage>,
    uptime: &Option<UptimeStatusMessage>,
    override_data: &PersonalizationOverride,
) -> ViewData {
    let locale = override_data.locale;
    match view {
        SysinfoView::Cpu => {
            let status = match cpu {
                Some(s) => s,
                None => return ViewData::error("nf-md-gauge_empty".to_string(), "Loading...".to_string()),
            };
            let usage = status.cpu_usage.clamp(0.0, 100.0);
            let level = UsageLevel::from_percent(usage);
            let icon = level.get_icon_name().unwrap_or_else(|| "nf-md-gauge_empty".to_string());
            let label = SysinfoLabel::Cpu.localized_label(locale);
            let color = level.get_icon_color();
            ViewData::with_color(icon, format!("{:.0}%", usage), label.to_string(), color)
        }
        SysinfoView::CpuTemperature => {
            let status = match cpu {
                Some(s) => s,
                None => return ViewData::error("nf-fa-thermometer_empty".to_string(), "Loading...".to_string()),
            };
            let temp = match status.cpu_temperature.as_ref().copied() {
                Some(t) => t,
                None => return ViewData::new("nf-fa-thermometer_empty".to_string(), "--".to_string(), SysinfoLabel::Temperature.localized_label(locale).to_string()),
            };
            let formatted = override_data.format_temperature(temp);
            let level = SysinfoTemperatureLevel::from_celsius(temp);
            let icon = level.get_icon_name().unwrap_or_else(|| "nf-fa-thermometer_empty".to_string());
            let label = SysinfoLabel::Temperature.localized_label(locale);
            let color = level.get_icon_color();
            ViewData::with_color(icon, formatted, label.to_string(), color)
        }
        SysinfoView::Memory => {
            let status = match memory {
                Some(s) => s,
                None => return ViewData::error("nf-md-memory".to_string(), "Loading...".to_string()),
            };
            let usage = status.memory_usage.clamp(0.0, 100.0);
            let label = SysinfoLabel::Memory.localized_label(locale);
            let color = UsageLevel::from_percent(usage).get_icon_color();
            ViewData::with_color("nf-md-memory".to_string(), format!("{:.0}%", usage), label.to_string(), color)
        }
        SysinfoView::Battery => {
            let status = match battery {
                Some(s) => s,
                None => return ViewData::error("nf-md-battery".to_string(), "Loading...".to_string()),
            };
            let level = status.level.clamp(0.0, 100.0);
            let icon = match status.status {
                BatteryStatus::Charging => "nf-md-battery_charging",
                BatteryStatus::Full => "nf-md-battery",
                BatteryStatus::Discharging => "nf-md-battery_alert",
                BatteryStatus::Unknown => "nf-md-battery",
            };
            let label = SysinfoLabel::Battery.localized_label(locale);
            let color = BatteryLevel::from_status(level, status.status).get_icon_color();
            ViewData::with_color(icon.to_string(), format!("{:.0}%", level), label.to_string(), color)
        }
        SysinfoView::Disk => {
            let status = match disks {
                Some(s) => s,
                None => return ViewData::error("nf-md-harddisk".to_string(), "Loading...".to_string()),
            };
            let usage = status.mounts.iter().next().map(|m| m.usage).unwrap_or(0.0);
            let label = SysinfoLabel::Disk.localized_label(locale);
            let color = UsageLevel::from_percent(usage).get_icon_color();
            ViewData::with_color("nf-md-harddisk".to_string(), format!("{:.0}%", usage), label.to_string(), color)
        }
        SysinfoView::NetworkDownload => {
            let status = match network {
                Some(s) => s,
                None => return ViewData::error("nf-md-download".to_string(), "Loading...".to_string()),
            };
            let formatted = override_data.format_data_rate(status.received_bytes_per_second);
            let label = SysinfoLabel::Download.localized_label(locale);
            ViewData::new("nf-md-download".to_string(), formatted, label.to_string())
        }
        SysinfoView::NetworkUpload => {
            let status = match network {
                Some(s) => s,
                None => return ViewData::error("nf-md-upload".to_string(), "Loading...".to_string()),
            };
            let formatted = override_data.format_data_rate(status.transmitted_bytes_per_second);
            let label = SysinfoLabel::Upload.localized_label(locale);
            ViewData::new("nf-md-upload".to_string(), formatted, label.to_string())
        }
        SysinfoView::Uptime => {
            let status = match uptime {
                Some(s) => s,
                None => return ViewData::error("nf-md-clock_outline".to_string(), "Loading...".to_string()),
            };
            let seconds = status.uptime_seconds;
            let days = seconds / 86400;
            let hours = (seconds % 86400) / 3600;
            let minutes = (seconds % 3600) / 60;
            let formatted = if days > 0 {
                format!("{}d {:02}h", days, hours)
            } else {
                format!("{:02}h {:02}m", hours, minutes)
            };
            let label = SysinfoLabel::Uptime.localized_label(locale);
            ViewData::new("nf-md-clock_outline".to_string(), formatted, label.to_string())
        }
        SysinfoView::Load => {
            let status = match uptime {
                Some(s) => s,
                None => return ViewData::error("nf-md-chart_line".to_string(), "Loading...".to_string()),
            };
            let load = status.load_average_1m;
            let label = SysinfoLabel::Load.localized_label(locale);
            let color = UsageLevel::from_percent((load * 100.0 / num_cpus() as f32).clamp(0.0, 100.0)).get_icon_color();
            ViewData::with_color("nf-md-chart_line".to_string(), format!("{:.2}", load), label.to_string(), color)
        }
    }
}
```

> **Note:** The `render_view` function reuses the same semantic color logic already implemented in the atomic widget's `render` method. The function is shared
> between GTK, Headless, and Web rendering paths.

#### 5.13 GTK UI Layout — Unified 4-Line Layout

The GTK layout follows the **Unified 4-Line Layout** from `docs/ICON_RENDERING.md`, identical to the Network Widget:

**Compact mode** (vertical):

```
+-----------------------------------+
|            [Icon]                 |  Line 0 (icon_size)
|               42%                 |  Line 1 (20px, widget-main-text)
|              Cpu                  |  Line 2 (16px, widget-info-text)
|                                   |  Line 3 (16px, spacer)
+-----------------------------------+
```

**Wide mode** (horizontal):

```
+-----------------------------------+
|  [Icon]  42%                      |  Line 0 (icon_size) + Line 1 (20px)
|          Cpu                      |  Line 2 (16px, widget-info-text)
|                                   |  Line 3 (16px, spacer)
+-----------------------------------+
```

**Structure:**

- `GtkBox` (vertical, `content_box`) containing:
    - `Image` (icon, Nerd Font character) — Line 0, `height_request = icon_size`
    - `Label` (`widget-main-text`) — Line 1, `height_request = 20`, CSS class `widget-main-text`
    - `Label` (`widget-info-text`) — Line 2, `height_request = 16`, CSS class `widget-info-text`
    - `LevelBar` (progress bar) — Line 3, `height_request = 16`, CSS classes `sysinfo-bar` + `sysinfo-normal`/`sysinfo-warning`/`sysinfo-critical`
    - `Label` (spacer) — Line 3, `height_request = 16`, hidden when `LevelBar` is visible

The `LevelBar` and spacer share Line 3. Only one is visible at a time:

- **Percentage views** (Cpu, Memory, Battery, Disk, Load): `LevelBar` visible, spacer hidden
- **Non-percentage views** (CpuTemperature, NetworkDownload, NetworkUpload, Uptime): `LevelBar` hidden, spacer visible

The `LevelBar` is configured as:

```rust
let bar = LevelBar::builder()
.min_value(0.0)
.max_value(100.0)
.orientation(Orientation::Horizontal)
.width_request(effective_width)
.height_request(16)
.css_classes(["sysinfo-bar", "sysinfo-normal"])
.visible(false)
.build();
```

In Compact mode with `icon_only = true`, lines 1–3 are empty/hidden but retain their `height_request` to preserve icon alignment across widgets.

In Wide mode, the layout is horizontal: icon on the left, text labels on the right. The `LevelBar` is shown below the text labels in Wide mode as well.

#### 5.14 `icon_only` Support

The widget supports `icon_only` via `WidgetIcon` (flattened into config). When `icon_only = true` (Compact mode only):

- `widget-main-text` label is set to `""`
- `widget-info-text` label is set to `""`
- Spacer retains its `height_request`
- Icon remains visible

This matches the behavior of all other widgets using `WidgetIcon` (see `docs/ICON_RENDERING.md` → `icon_only` Support table).

#### 5.15 Icon Color Application

The `update_icon_display` helper (already implemented in `plugins/network/src/widget.rs`) is reused:

```rust
fn update_icon_display(img: &Image, view_data: &ViewData, icon_size: i32, configured_color: Option<Color>) {
    set_icon_image(img, &view_data.icon_name, icon_size);
    let color = view_data.icon_color.or(configured_color);
    if let Some(c) = color {
        apply_icon_color(img, c);
    }
}
```

> **Note:** This helper should be extracted to a shared utility (e.g. `plugin-api/src/widget/helpers.rs` or `smearor-render-utils`) to avoid duplication between
> Network and Sysinfo widgets.

#### 5.16 Exit Criteria

- `SysinfoMultiWidget` implements all required traits.
- View switching works via swipe/scroll gestures.
- Semantic icon colors are applied per view.
- Unified 4-Line Layout is followed (icon alignment matches Network, Weather, and other widgets).
- `LevelBar` is shown for percentage views (Cpu, Memory, Battery, Disk, Load) and hidden for non-percentage views.
- `LevelBar` CSS classes update dynamically based on warning/critical thresholds.
- `icon_only` mode works correctly in Compact mode.
- `WidgetMode` (compact/wide) is supported.
- `cargo build -p smearor-sysinfo-widget` succeeds.

---

### Phase 4: Headless Graphic Renderer (`plugins/sysinfo/src/graphic.rs`)

#### 5.17 GraphicRenderer Implementation

A `GraphicRenderer` implementation for `SysinfoMultiWidget` is added to `graphic.rs`:

```rust
impl GraphicRenderer for SysinfoMultiWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(SysinfoView::Cpu);
        let override_data = self.personalization.borrow().clone();

        let view_data = render_view(
            view,
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_network.borrow(),
            &self.latest_uptime.borrow(),
            &override_data,
        );

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}
```

The existing `render_view_data_to_graphic` function is reused — it already handles `view_data.icon_color` for semantic icon coloring.

#### 5.18 Exit Criteria

- Headless rendering produces correct icon + text for each view.
- Semantic colors are applied to the icon.
- `cargo build` succeeds with headless feature.

---

### Phase 5: Web Renderer (`plugins/sysinfo/src/html.rs`)

#### 5.19 WebRenderer Implementation

A `WebRenderer` implementation for `SysinfoMultiWidget` is added to `html.rs`:

```rust
impl WebRenderer for SysinfoMultiWidget {
    fn render_html(&self) -> String {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(SysinfoView::Cpu);
        let override_data = self.personalization.borrow().clone();

        let view_data = render_view(
            view,
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_network.borrow(),
            &self.latest_uptime.borrow(),
            &override_data,
        );

        render_view_data_to_html(&view_data, &self.config.icon_config)
    }
}
```

The existing `render_view_data_to_html` function is reused — it already injects `color: rgba(...)` inline CSS for semantic icon coloring.

#### 5.20 Exit Criteria

- Web rendering produces correct HTML with semantic icon colors.
- `cargo build` succeeds with web feature.

---

### Phase 6: Plugin Registration (`plugins/sysinfo/src/lib.rs`)

#### 5.21 Macro Registration

Add the multi-view widget to the `widget_factory_plugin_graphic!` macro:

```rust
widget_factory_plugin_graphic! {
    // ... existing widgets ...
    "sysinfo_multi" => sysinfo_multi_widget => SysinfoMultiWidget => html,
    // ... existing atomic widgets ...
}
```

#### 5.22 Module Declaration

Add `pub mod multi_widget;` to `lib.rs`.

#### 5.23 Exit Criteria

- Widget is registered and loadable as `sysinfo_multi`.
- `cargo build --release` succeeds.

---

### Phase 7: Configuration & Integration

#### 5.24 TOML Configuration Example

```toml
[[main_menu.plugins]]
id = "sysinfo_multi"
path = "target/release/libsmearor_sysinfo_widget.so"
widget = "sysinfo_multi"
icon_size = 36
icon_only = false
mode = "compact"
views = ["cpu", "cpu_temperature", "memory", "battery", "disk", "network_download", "network_upload", "uptime", "load"]
```

Or with a subset of views in wide mode:

```toml
[[main_menu.plugins]]
id = "sysinfo_compact"
path = "target/release/libsmearor_sysinfo_widget.so"
widget = "sysinfo_multi"
icon_size = 36
mode = "wide"
max_width = 200
views = ["cpu", "memory", "battery", "uptime"]
```

#### 5.25 Action Bindings

Default gesture bindings (matching Network Widget):

```toml
[sysinfo_multi.actions]
swipe_up = "next_view"
swipe_down = "prev_view"
scroll_up = "next_view"
scroll_down = "prev_view"
```

#### 5.26 Exit Criteria

- Widget loads from TOML config with `widget = "sysinfo_multi"`.
- Views list is configurable and defaults to all 9 metrics.
- Gesture bindings work out of the box.

---

## 6. File Structure

### 6.1 New Files

| File                                  | Responsibility                                              |
|---------------------------------------|-------------------------------------------------------------|
| `model/sysinfo/src/messages/view.rs`  | `SysinfoView` enum definition                               |
| `plugins/sysinfo/src/multi_widget.rs` | `SysinfoMultiWidget` struct, traits, view switching, GTK UI |

### 6.2 Modified Files

| File                                           | Change                                               |
|------------------------------------------------|------------------------------------------------------|
| `model/sysinfo/src/messages/mod.rs`            | Add `pub mod view;`                                  |
| `model/sysinfo/src/lib.rs`                     | Add `pub use messages::view::SysinfoView;`           |
| `model/sysinfo/src/model/usage_level.rs`       | Update `get_icon_name()` to return gauge icons       |
| `model/sysinfo/src/model/temperature_level.rs` | Update `get_icon_name()` to return thermometer icons |
| `plugins/sysinfo/src/config.rs`                | Add `SysinfoMultiWidgetConfig` struct                |
| `plugins/sysinfo/src/graphic.rs`               | Add `GraphicRenderer for SysinfoMultiWidget`         |
| `plugins/sysinfo/src/html.rs`                  | Add `WebRenderer for SysinfoMultiWidget`             |
| `plugins/sysinfo/src/lib.rs`                   | Add `pub mod multi_widget;` and macro registration   |

### 6.3 Unchanged Files

- `services/sysinfo/` — no changes needed (service already broadcasts all topics)
- `model/sysinfo/src/messages/` — message structs unchanged
- `model/sysinfo/src/model/battery_level.rs` — `BatteryLevel` already has state-dependent battery icons in `render_view`
- Existing single-metric widgets — unchanged (but benefit from `get_icon_name()` updates in atomic widgets)

---

## 7. Dependencies

### 7.1 Existing Dependencies (Reused)

| Crate                               | Usage                                       |
|-------------------------------------|---------------------------------------------|
| `smearor-sysinfo-model`             | Message types, `SysinfoView`, level enums   |
| `smearor-swipe-launcher-plugin-api` | `ViewData`, `WidgetIcon`, `GraphicRenderer` |
| `smearor-render-utils`              | Drawing utilities for headless rendering    |
| `smearor-personization-model`       | Locale and unit overrides                   |
| `gtk4`                              | GTK4 widget UI                              |
| `serde`                             | Config deserialization                      |
| `typed-builder`                     | Builder pattern for config                  |

### 7.2 No New Dependencies

The multi-view widget uses only existing dependencies. No new crates are needed.

---

## 8. Testing

### 8.1 Unit Tests

- `SysinfoView::from_str` parses all view names correctly.
- `SysinfoMultiWidgetConfig` deserializes from TOML with default and custom views.
- `render_view` produces correct `ViewData` for each view variant.
- `render_view` returns error `ViewData` when status messages are `None`.

### 8.2 Integration Tests

- Widget loads from TOML config with `widget = "sysinfo_multi"`.
- View switching cycles through all configured views.
- Semantic colors are applied per view (e.g. CPU at 95% → red icon).
- Headless rendering produces non-empty `FfiGraphic` with correct pixel data.
- Web rendering produces HTML with `color: rgba(...)` style for semantic colors.

### 8.3 Manual Tests

- Swipe Up / Swipe Down cycles through views in the launcher.
- Icon and text update correctly when new status messages arrive.
- Semantic colors are visible (e.g. green CPU icon at low load, red at high load).

---

## 9. Migration Path

The multi-view widget is **additive** — it does not replace or modify existing widgets:

1. **Existing configs continue to work** — single-metric widgets (`cpu`, `memory`, etc.) remain registered and functional.
2. **Users can opt-in** by adding a `sysinfo_multi` widget to their config.
3. **No breaking changes** — no existing config fields are removed or renamed.
4. **Gradual adoption** — users can replace multiple single-metric tiles with one multi-view tile at their own pace.

---

## 10. Documentation Updates

The following documentation files must be updated when implementing this concept:

### 10.1 `docs/ICON_RENDERING.md`

**Widget Icon Matrix** — add a new row:

| Widget            | `icon` | `icon_size` | `icon_only` | Dynamic Icon | View-dependent | State-dependent | Other Icon Config                           |
|-------------------|:------:|:-----------:|:-----------:|:------------:|:--------------:|:---------------:|---------------------------------------------|
| **sysinfo-multi** |   —    |     yes     |     yes     |     yes      |      yes       |       yes       | `mode` (compact/wide), `max_width`, `views` |

**Dynamic Icon Categories → View-dependent Icons** — add:

- **sysinfo-multi**: 9 views (Cpu, CpuTemperature, Memory, Battery, Disk, NetworkDownload, NetworkUpload, Uptime, Load), each with its own icon. Within the
  Battery view, the icon is additionally state-dependent (charging/discharging/full). Semantic colors are applied via `UsageLevel`, `BatteryLevel`,
  `SysinfoTemperatureLevel`.

**Dynamic Icon Categories → State-dependent Icons** — add:

- **sysinfo-multi**: Within the Battery view, the icon changes based on charging state (`nf-md-battery_charging`, `nf-md-battery`, `nf-md-battery_alert`).
  Within the Cpu view, the icon changes based on usage level (`nf-md-gauge_empty`, `nf-md-gauge_low`, `nf-md-gauge_full` via `UsageLevel::get_icon_name()`).
  Within the CpuTemperature view, the icon changes based on temperature level (`nf-fa-thermometer_empty`, `nf-fa-thermometer_quarter`, `nf-fa-thermometer_half`,
  `nf-fa-thermometer_full` via `SysinfoTemperatureLevel::get_icon_name()`). Semantic icon colors change based on the respective level enums for Cpu, Memory,
  Disk, Load, Battery, and CpuTemperature views.

**Unified 4-Line Layout** table — add column:

| Sysinfo Multi-View                      |
|-----------------------------------------|
| Icon                                    |
| `widget-main-text` (value)              |
| `widget-info-text` (label)              |
| `LevelBar` (percentage views) or spacer |

**`icon_only` Support** table — add row:

| Widget            |       Config Field        | Behavior                                                 |
|-------------------|:-------------------------:|----------------------------------------------------------|
| **sysinfo-multi** | `icon_config: WidgetIcon` | Hides `value_label` and `info_label` (compact mode only) |

**Widgets where `icon_only` is not applicable** — remove `sysinfo` from this list (the multi-view widget supports `icon_only`; the existing single-metric
widgets remain unaffected).

### 10.2 `concepts/planned/WIDGET_ICON_COLOR.md`

No changes needed — the `icon_color` priority model already covers the multi-view widget.

---

## 11. Future Extensions

- **Per-view icon overrides** — allow configuring a custom icon per view (e.g. `icon_cpu = "nf-fae-chip"`).
- **Per-view display modes** — support Bar/Gauge rendering for percentage-based views in the multi-view widget.
- **View-specific actions** — e.g. long-press on Battery view opens power settings, long-press on CPU view opens system monitor.
- **Auto-rotation** — automatically cycle views every N seconds if no user interaction.
- **View ordering persistence** — remember the last viewed metric across launcher sessions.
