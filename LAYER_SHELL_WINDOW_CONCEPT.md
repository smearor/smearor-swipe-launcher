# Layer Shell Window Monitor Assignment Concept

## 1. Goal

Enable each launcher instance to independently target a specific monitor for its layer-shell surface. When no monitor is configured, the primary monitor is
used. Window dimensions (width for horizontal launchers, height for vertical launchers) are derived from the selected monitor's geometry.

## 2. Current State

### 2.1 Window Creation

`create_window` in `smearor-swipe-launcher/src/window.rs` calls `window.init_layer_shell()` but never sets a monitor. The compositor implicitly places the
surface on the primary monitor:

```rust
window.init_layer_shell();
// no set_monitor() call — compositor picks the monitor
```

### 2.2 Hard-Coded Monitor Index

Both `calculate_area_size` in `display.rs` and `calculate_coordinated_sizes` in `application.rs` fetch `monitors.item(0)` — always the first monitor:

```rust
// display.rs:38
let Some(monitor) = monitors.item(0).and_then( | m| m.downcast::<Monitor>().ok()) else { ... };

// application.rs:155
let Some(monitor) = monitors.item(0).and_then( | m| m.downcast::<Monitor>().ok()) else { ... };
```

This means all size calculations are based on the first monitor's geometry, regardless of where the window actually appears.

### 2.3 Multi-Instance Architecture

The launcher already supports multiple instances via `LauncherHost` and `LauncherInstance` (see `MULTI_INSTANCE_CONCEPT.md`). Each instance has its own
`SwipeLauncherConfig` and builds its own `ApplicationWindow`. This is the perfect foundation for per-instance monitor assignment.

### 2.4 Existing LayoutTrigger

`LayoutTrigger` in `config/layout/trigger.rs` already has a `Monitor(String)` variant for layout profile switching, but this is unrelated to window placement —
it only affects which layout profile is selected.

## 3. Requirements

1. **Per-instance monitor selection** — Each `LauncherInstance` can independently specify which monitor to target.
2. **Primary monitor fallback** — When no monitor is configured, the primary monitor is used.
3. **Monitor-dependent sizing** — The launcher's width (horizontal: 0°/180°) or height (vertical: 90°/270°) is calculated from the selected monitor's geometry,
   not a hard-coded index.
4. **Configuration file support** — The monitor for each instance is configurable via the TOML config file.

## 4. Proposed Solution

### 4.1 Monitor Identification

Monitors are identified by **index** (integer). The GDK `Display::monitors()` list model provides monitors in a stable order determined by the compositor. Index
`0` is the primary monitor.

**Why index-based?**

- `gtk4_layer_shell::LayerShell::set_monitor` accepts a `&gdk::Monitor` directly.
- GDK's `ListModel` is index-addressable via `item(n)`.
- Monitor names (connector names like "HDMI-A-1") are not directly available from GDK's `Monitor` API in a reliable cross-compositor way. The index is the
  simplest stable identifier.

**Future enhancement:** Optionally support monitor identification by connector name. GDK's `Monitor` does not expose the connector name directly, but
`zwp_output_head_v1` (via the Wayland `xdg-output` protocol) could be used. This is out of scope for the initial implementation.

### 4.2 Configuration

Add a `monitor` field to `LayerConfigFile` in `smearor-swipe-launcher/src/config/layer.rs`:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayerConfigFile {
    /// Specify the layer for the layer shell protocol (e.g., Background, Top).
    #[serde(default)]
    pub(crate) layer: Option<SmearorLayer>,

    /// Namespace for the layer shell, used by compositors for rules.
    #[serde(default)]
    pub(crate) namespace: Option<String>,

    /// Exclusive zone in pixels.
    /// When set to None, auto-exclusive-zone is enabled.
    /// Use 0 to disable exclusive zone (overlay mode).
    #[serde(default)]
    pub(crate) exclusive_zone: Option<i32>,

    /// Monitor index for the layer shell surface.
    /// When None, the primary monitor (index 0) is used.
    #[serde(default)]
    pub(crate) monitor: Option<u32>,
}
```

**Example config:**

```toml
[launcher.layer]
layer = "Top"
namespace = "smearor-bottom"
exclusive_zone = 50
monitor = 1          # Place on the second monitor
```

```toml
# No monitor specified — uses primary monitor
[launcher.layer]
layer = "Top"
namespace = "smearor-top"
exclusive_zone = 50
```

### 4.3 CLI Argument (Optional)

Add an optional `--monitor` CLI argument for override capability, mirroring the existing `--layer` and `--namespace` pattern:

```rust
// args/layer.rs
#[derive(Parser, Debug, Clone)]
pub struct LayerArguments {
    #[arg(long)]
    pub(crate) layer: Option<SmearorLayer>,

    #[arg(short = 'n', long)]
    pub(crate) namespace: Option<String>,

    /// Monitor index for the layer shell surface.
    /// Overrides the config file value.
    #[arg(long)]
    pub(crate) monitor: Option<u32>,
}
```

Merge in `LayerConfigFile::merge_with_arguments`:

```rust
impl MergeWithArguments<LayerArguments> for LayerConfigFile {
    fn merge_with_arguments(self, args: &LayerArguments) -> Self {
        let mut config = self;
        if let Some(layer) = args.layer {
            config.layer = Some(layer);
        }
        if let Some(namespace) = &args.namespace {
            config.namespace = Some(namespace.clone());
        }
        if let Some(monitor) = args.monitor {
            config.monitor = Some(monitor);
        }
        config
    }
}
```

### 4.4 Monitor Resolution Helper

Add a helper function in `display.rs` to resolve a monitor by index with fallback to primary:

```rust
/// Resolves the monitor for the given index.
/// Falls back to the primary monitor (index 0) if the index is
/// out of bounds or no display is available.
pub fn resolve_monitor(monitor_index: Option<u32>) -> Option<Monitor> {
    let display = Display::default()?;
    let monitors = display.monitors();
    let index = monitor_index.unwrap_or(0);
    monitors
        .item(index)
        .and_then(|m| m.downcast::<Monitor>().ok())
        .or_else(|| {
            // Fallback to primary monitor
            monitors
                .item(0)
                .and_then(|m| m.downcast::<Monitor>().ok())
        })
}
```

### 4.5 Window Creation: `set_monitor`

In `create_window` (`window.rs`), call `window.set_monitor()` after `init_layer_shell()`:

```rust
pub fn create_window(
    app: &gtk4::Application,
    config: &SwipeLauncherSettings,
    coordinated_size: Option<AreaSize>,
) -> ApplicationWindow {
    let rotation = config.rotation.rotation();
    let monitor_index = config.layer.monitor;

    // Resolve monitor before calculating size
    let monitor = resolve_monitor(monitor_index);
    let height = config.layer.exclusive_zone().unwrap_or(DEFAULT_HEIGHT);
    let area_size = coordinated_size
        .unwrap_or_else(|| calculate_area_size_for_monitor(rotation, height, &monitor));

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(area_size.width)
        .default_height(area_size.height)
        .build();

    window.init_layer_shell();

    // Assign the layer surface to the selected monitor
    if let Some(ref monitor) = monitor {
        window.set_monitor(monitor);
    }

    // ... rest of layer configuration (layer, namespace, exclusive_zone, anchors)
    window
}
```

### 4.6 Monitor-Aware Size Calculation

Update `calculate_area_size` in `display.rs` to accept an optional monitor reference instead of hard-coding `item(0)`:

```rust
/// Calculates the area size based on rotation and the given monitor's geometry.
/// Falls back to defaults if no monitor is available.
pub fn calculate_area_size_for_monitor(
    rotation: SmearorRotation,
    default_size: i32,
    monitor: &Option<Monitor>,
) -> AreaSize {
    let Some(monitor) = monitor else {
        return AreaSize::default();
    };
    let geometry = monitor.geometry();
    let screen_width = geometry.width();
    let screen_height = geometry.height();

    let rotation = rotation.to_degrees();
    let is_horizontal = (rotation - 0.0).abs() < 0.1 || (rotation - 180.0).abs() < 0.1;
    let is_vertical = (rotation - 90.0).abs() < 0.1 || (rotation - 270.0).abs() < 0.1;

    if is_horizontal {
        AreaSize::new(screen_width, default_size)
    } else if is_vertical {
        AreaSize::new(default_size, screen_height)
    } else {
        AreaSize::default()
    }
}
```

The existing `calculate_area_size` function can delegate to this new function:

```rust
pub fn calculate_area_size(rotation: SmearorRotation, default_size: i32) -> AreaSize {
    let monitor = resolve_monitor(None);
    calculate_area_size_for_monitor(rotation, default_size, &monitor)
}
```

### 4.7 Monitor-Aware Coordinated Sizes

Update `calculate_coordinated_sizes` in `application.rs` to group instances by their configured monitor and calculate sizes per-monitor:

```rust
pub fn calculate_coordinated_sizes(&self) {
    let Some(display) = Display::default() else {
        return;
    };
    let monitors = display.monitors();

    let Ok(instances) = self.instances.lock() else {
        return;
    };

    // Group instances by their configured monitor index
    // and calculate coordinated sizes per monitor group.
    let mut monitor_groups: HashMap<u32, Vec<&LauncherInstance>> = HashMap::new();
    for instance in instances.values() {
        let monitor_index = instance.config.launcher.layer.monitor.unwrap_or(0);
        monitor_groups.entry(monitor_index).or_default().push(instance);
    }

    for (monitor_index, group) in &monitor_groups {
        let Some(monitor) = monitors
            .item(*monitor_index)
            .and_then(|m| m.downcast::<Monitor>().ok())
        else {
            continue;
        };
        let geometry = monitor.geometry();
        let monitor_height = geometry.height();

        // Sum exclusive-zone heights of all long-side launchers on this monitor
        let mut long_side_height_sum = 0_i32;
        for instance in group {
            let rotation = instance.config.launcher.rotation.rotation().to_degrees();
            let is_long_side = (rotation - 0.0).abs() < 0.1 || (rotation - 180.0).abs() < 0.1;
            if is_long_side {
                let height = instance.config.launcher.layer.exclusive_zone().unwrap_or(150);
                long_side_height_sum += height;
            }
        }

        // Adjust short-side launchers so they avoid the reserved bands
        for instance in group {
            let rotation = instance.config.launcher.rotation.rotation().to_degrees();
            let is_short_side = (rotation - 90.0).abs() < 0.1 || (rotation - 270.0).abs() < 0.1;
            if is_short_side {
                let default_size = instance.config.launcher.layer.exclusive_zone().unwrap_or(150);
                let adjusted_height = (monitor_height - long_side_height_sum).max(default_size);
                let coordinated_size = AreaSize::new(default_size, adjusted_height);
                if let Ok(mut size) = instance.coordinated_size.lock() {
                    *size = Some(coordinated_size);
                }
                debug!(
                    "Instance {} short-side coordinated size: {}x{} (monitor {})",
                    instance.instance_id, coordinated_size.width, coordinated_size.height, monitor_index
                );
            }
        }
    }
}
```

This ensures that instances on different monitors are sized independently — two long-side launchers on monitor 0 do not affect the height budget of a short-side
launcher on monitor 1.

## 5. Affected Files

| File                                         | Change                                                                                                            |
|----------------------------------------------|-------------------------------------------------------------------------------------------------------------------|
| `smearor-swipe-launcher/src/config/layer.rs` | Add `monitor: Option<u32>` field to `LayerConfigFile`                                                             |
| `smearor-swipe-launcher/src/args/layer.rs`   | Add `--monitor` CLI argument to `LayerArguments`                                                                  |
| `smearor-swipe-launcher/src/display.rs`      | Add `resolve_monitor()` helper; add `calculate_area_size_for_monitor()`; update `calculate_area_size` to delegate |
| `smearor-swipe-launcher/src/window.rs`       | Call `window.set_monitor()` after `init_layer_shell()`; use monitor-aware size calculation                        |
| `smearor-swipe-launcher/src/application.rs`  | Update `calculate_coordinated_sizes` to group instances by monitor and calculate per-monitor                      |

## 6. Example Configurations

### 6.1 Single Instance — Primary Monitor (Default)

```toml
[launcher.layer]
layer = "Top"
namespace = "smearor-bottom"
exclusive_zone = 50
# No monitor field → primary monitor (index 0)
```

### 6.2 Two Instances — Different Monitors

**`config-bottom.toml`** (monitor 0, bottom edge):

```toml
areas = ["main"]

[launcher]
rotation = "Bottom"

[launcher.layer]
layer = "Top"
namespace = "smearor-bottom"
exclusive_zone = 50
monitor = 0
```

**`config-side.toml`** (monitor 1, right edge):

```toml
areas = ["main"]

[launcher]
rotation = "Right"

[launcher.layer]
layer = "Top"
namespace = "smearor-side"
exclusive_zone = 50
monitor = 1
```

**Launch:**

```bash
./smearor-swipe-launcher \
    --config config-bottom.toml --instance-id bottom \
    --config config-side.toml --instance-id side
```

### 6.3 CLI Override

```bash
./smearor-swipe-launcher --config config.toml --monitor 2
```

The `--monitor 2` argument overrides any `monitor` value in the config file.

## 7. Edge Cases

- **Monitor index out of bounds** — `resolve_monitor` falls back to the primary monitor (index 0). A warning is logged.
- **Hotplugged monitors** — If a monitor is disconnected after the launcher starts, the layer-shell surface is typically moved by the compositor to the
  remaining primary monitor. No special handling required.
- **Single monitor setup** — `monitor = 0` (or unset) works as before. Setting `monitor = 1` on a single-monitor system falls back to index 0.
- **Coordinated sizes across monitors** — Instances on different monitors are sized independently. The long-side/short-side coordination only applies to
  instances sharing the same monitor.

## 8. Implementation Roadmap

### Phase 1: Config & Args

| #   | Task                                              | Files                              | Effort |
|-----|---------------------------------------------------|------------------------------------|--------|
| 1.1 | Add `monitor: Option<u32>` to `LayerConfigFile`   | `config/layer.rs`                  | Small  |
| 1.2 | Add `--monitor` to `LayerArguments` + merge logic | `args/layer.rs`, `config/layer.rs` | Small  |

### Phase 2: Monitor Resolution & Sizing

| #   | Task                                                                              | Files        | Effort |
|-----|-----------------------------------------------------------------------------------|--------------|--------|
| 2.1 | Add `resolve_monitor()` helper                                                    | `display.rs` | Small  |
| 2.2 | Add `calculate_area_size_for_monitor()`; update `calculate_area_size` to delegate | `display.rs` | Small  |

### Phase 3: Window Creation

| #   | Task                                                  | Files       | Effort |
|-----|-------------------------------------------------------|-------------|--------|
| 3.1 | Call `window.set_monitor()` in `create_window`        | `window.rs` | Small  |
| 3.2 | Use monitor-aware size calculation in `create_window` | `window.rs` | Small  |

### Phase 4: Coordinated Sizes

| #   | Task                                                              | Files            | Effort |
|-----|-------------------------------------------------------------------|------------------|--------|
| 4.1 | Group instances by monitor index in `calculate_coordinated_sizes` | `application.rs` | Medium |

### Phase 5: Testing & Validation

| #   | Task                                                           | Effort |
|-----|----------------------------------------------------------------|--------|
| 5.1 | Test single-instance with no `monitor` field (backward compat) | Small  |
| 5.2 | Test two instances on different monitors                       | Medium |
| 5.3 | Test out-of-bounds monitor index fallback                      | Small  |
| 5.4 | Test CLI `--monitor` override                                  | Small  |
