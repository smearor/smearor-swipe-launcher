# Layout Profile per Monitor Concept

## 1. Goal

Enable each launcher instance to display a different layout (areas, plugins, widths) depending on
which monitor it is placed on. This builds on the existing `LayoutProfile` and `LayoutTrigger`
infrastructure, which is already defined but not yet wired into the runtime.

## 2. Current State

### 2.1 Existing Infrastructure

`LayoutTrigger` in `smearor-swipe-launcher/src/config/layout/trigger.rs` already defines monitor-based
triggers:

```rust
pub enum LayoutTrigger {
    Default,
    Monitor(String),
    Workspace(i32),
    MonitorWorkspace { monitor: String, workspace: i32 },
}
```

`LayoutProfile` in `smearor-swipe-launcher/src/config/layout/profile.rs` holds an ordered area list and
entries:

```rust
pub struct LayoutProfile {
    pub trigger: LayoutTrigger,
    pub areas: Vec<String>,
    #[serde(flatten)]
    pub entries: HashMap<String, ConfigEntry>,
}
```

`SwipeLauncherConfig` in `smearor-swipe-launcher/src/config/launcher.rs` stores profiles and provides
`get_layout_for_context`:

```rust
pub fn get_layout_for_context(
    &self,
    monitor: Option<&str>,
    workspace: Option<i32>,
) -> (&Vec<String>, &HashMap<String, ConfigEntry>) {
    for profile in &self.profiles {
        match &profile.trigger {
            LayoutTrigger::MonitorWorkspace { monitor: m, workspace: w } => {
                if Some(m.as_str()) == monitor && Some(*w) == workspace {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::Monitor(m) => {
                if Some(m.as_str()) == monitor {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::Workspace(w) => {
                if Some(*w) == workspace {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::Default => {}
        }
    }
    (&self.areas, &self.entries)
}
```

### 2.2 Problem

`get_layout_for_context` is **never called**. The `LauncherInstance` always uses `self.config.areas`
and `self.config.entries` directly in `build_window` and `load_plugins`. The `LayoutTrigger::Monitor`
and `LayoutTrigger::MonitorWorkspace` variants are defined but have no runtime effect.

### 2.3 Monitor Identification Gap

`LayoutTrigger::Monitor(String)` expects a monitor **name** (e.g. `"DP-1"`), but GDK does not expose
connector names. The `LAYER_SHELL_WINDOW_CONCEPT.md` uses monitor **indices** (`Option<u32>`) for
window placement. These two identification systems need to be reconciled.

## 3. Requirements

1. **Per-monitor layout selection** — When a launcher instance is placed on a specific monitor, the
   matching layout profile is selected automatically.
2. **Index-based matching** — Since the monitor assignment from `LAYER_SHELL_WINDOW_CONCEPT.md` uses
   indices, layout profiles must also support index-based triggers.
3. **Fallback to default** — When no profile matches the current monitor, the default layout
   (`self.areas` / `self.entries`) is used.
4. **Runtime switching** — When a monitor change is detected (hotplug, see
   `LAYER_SHELL_WINDOW_CONCEPT.md` Section 9), the layout profile is re-evaluated.

## 4. Proposed Solution

### 4.1 Extend LayoutTrigger with Index-Based Monitor

Add a new variant to `LayoutTrigger` for index-based matching, complementing the existing
name-based `Monitor(String)`:

```rust
/// Defines the trigger condition for switching to a specific layout profile
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum LayoutTrigger {
    /// Default layout, used when no other trigger matches
    Default,
    /// Trigger based on monitor name (connector name, e.g. "DP-1")
    Monitor(String),
    /// Trigger based on monitor index (0-based, matching GDK display order)
    MonitorIndex(u32),
    /// Trigger based on workspace number
    Workspace(i32),
    /// Trigger based on both monitor name and workspace
    MonitorWorkspace { monitor: String, workspace: i32 },
    /// Trigger based on both monitor index and workspace
    MonitorIndexWorkspace { monitor: u32, workspace: i32 },
}
```

### 4.2 Update get_layout_for_context

Extend the matching logic to handle index-based triggers:

```rust
pub fn get_layout_for_context(
    &self,
    monitor_name: Option<&str>,
    monitor_index: Option<u32>,
    workspace: Option<i32>,
) -> (&Vec<String>, &HashMap<String, ConfigEntry>) {
    for profile in &self.profiles {
        match &profile.trigger {
            LayoutTrigger::MonitorIndexWorkspace { monitor: mi, workspace: w } => {
                if Some(*mi) == monitor_index && Some(*w) == workspace {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::MonitorWorkspace { monitor: m, workspace: w } => {
                if Some(m.as_str()) == monitor_name && Some(*w) == workspace {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::MonitorIndex(mi) => {
                if Some(*mi) == monitor_index {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::Monitor(m) => {
                if Some(m.as_str()) == monitor_name {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::Workspace(w) => {
                if Some(*w) == workspace {
                    return (&profile.areas, &profile.entries);
                }
            }
            LayoutTrigger::Default => {}
        }
    }
    (&self.areas, &self.entries)
}
```

### 4.3 Profile Priority

Profiles are evaluated in declaration order. The first matching profile wins. This allows users to
define more specific profiles (e.g. `MonitorIndexWorkspace`) before less specific ones (e.g.
`MonitorIndex`), and to define a `Default` profile as a catch-all at the end.

**Recommended declaration order:**

```toml
# Most specific first
[[profiles]]
trigger = { monitor_index = 1, workspace = 3 }
areas = ["special_layout"]
# ...

# Then by monitor index
[[profiles]]
trigger = { monitor_index = 1 }
areas = ["monitor_1_layout"]
# ...

# Default fallback
[[profiles]]
trigger = "Default"
areas = ["default_layout"]
# ...
```

### 4.4 Integration into LauncherInstance

`LauncherInstance::build_window` currently iterates over `self.config.areas` directly. It must be
changed to resolve the active layout profile first:

```rust
// instance.rs — in build_window

let monitor_index = config.launcher.layer.monitor;
let (areas, entries) = config.get_layout_for_context(None, monitor_index, None);

// Use `areas` and `entries` instead of `self.config.areas` and `self.config.entries`
for area_id in areas {
if let Some(area_config) = entries.get(area_id).and_then( | e | match e {
ConfigEntry::Area(config) => Some(config),
ConfigEntry::Plugin(_) => None,
}) {
// ... add area to UI
}
}
```

Similarly, `LauncherInstance::load_plugins` must use the resolved layout:

```rust
// instance.rs — in load_plugins

let monitor_index = self .config.launcher.layer.monitor;
let (areas, entries) = self .config.get_layout_for_context(None, monitor_index, None);

for area_id in areas {
if let Some(ConfigEntry::Area(area_config)) = entries.get(area_id) {
for plugin_entry in & area_config.plugins {
// ... load plugin
}
}
}
```

### 4.5 Monitor Name Resolution (Future Enhancement)

The `Monitor(String)` trigger uses connector names (e.g. `"DP-1"`). GDK does not expose these
directly. To support name-based triggers in the future:

1. **Via Hyprland IPC** — The Hyprland service can query `hyprctl monitors` and return a list of
   monitor names with their indices. See `HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md` for the event
   listener pattern.
2. **Via `wlr-output-management` protocol** — A Wayland protocol that exposes output names and
   geometry. Requires a separate protocol binding crate.
3. **Via `xdg-output` protocol** — Deprecated in favor of `wlr-output-management`, but still
   available on many compositors.

Until one of these is implemented, users should use `MonitorIndex(u32)` for index-based matching,
which works with the GDK monitor list directly.

## 5. Configuration Example

### 5.1 Index-Based Profiles

```toml
areas = ["main"]
# Default entries for the default layout

[launcher]
rotation = "Bottom"

[launcher.layer]
layer = "Top"
namespace = "smearor-bottom"
exclusive_zone = 50
monitor = 0

# Layout for monitor 0 (primary)
[[profiles]]
trigger = { monitor_index = 0 }
areas = ["left", "scroll", "right"]

[left]
type = "fixed"
width = 200
plugins = [{ id = "clock_left", path = "target/debug/libsmearor_clock_widget.so" }]

[scroll]
type = "scroll"
plugins = [{ id = "app_launcher", path = "target/debug/libsmearor_app_launcher_widget.so" }]

[right]
type = "fixed"
width = 200
plugins = [{ id = "clock_right", path = "target/debug/libsmearor_clock_widget.so" }]

# Layout for monitor 1 (secondary)
[[profiles]]
trigger = { monitor_index = 1 }
areas = ["main"]

[profile_main]
type = "scroll"
plugins = [{ id = "app_launcher_2", path = "target/debug/libsmearor_app_launcher_widget.so" }]
```

### 5.2 Second Instance on Monitor 1

```toml
# config-monitor1.toml
areas = ["main"]

[launcher]
rotation = "Right"

[launcher.layer]
layer = "Top"
namespace = "smearor-side"
exclusive_zone = 50
monitor = 1

# Different layout when on monitor 1
[[profiles]]
trigger = { monitor_index = 1 }
areas = ["compact"]

[compact]
type = "scroll"
plugins = [{ id = "clock_side", path = "target/debug/libsmearor_clock_widget.so" }]
```

### 5.3 Combined with Workspace (Future)

Once workspace tracking is implemented (see `HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md`):

```toml
[[profiles]]
trigger = { monitor_index = 0, workspace = 1 }
areas = ["full_layout"]

[[profiles]]
trigger = { monitor_index = 0, workspace = 2 }
areas = ["minimal_layout"]

[[profiles]]
trigger = { monitor_index = 0 }
areas = ["default_for_monitor_0"]
```

## 6. Runtime Layout Switching

### 6.1 Trigger: Monitor Change

When the hotplug handler from `LAYER_SHELL_WINDOW_CONCEPT.md` Section 9 rebuilds a window, the
layout profile is automatically re-evaluated because `build_window` calls
`get_layout_for_context` with the current monitor index.

No additional logic is needed — the rebuild flow naturally picks up the new monitor context.

### 6.2 Trigger: Workspace Change (Future)

When workspace tracking is available (see `HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md`), a workspace
change event triggers a layout re-evaluation:

```rust
// Future: in the workspace event handler
fn on_workspace_changed(&self, workspace: i32) {
    let monitor_index = self.config.launcher.layer.monitor;
    let (areas, entries) = self.config.get_layout_for_context(None, monitor_index, Some(workspace));

    // Rebuild UI with new layout
    self.rebuild_areas(areas, entries);
}
```

### 6.3 Area Rebuild

When the layout changes at runtime, the existing areas must be removed and new ones added. This
reuses the `AreaManager` API:

```rust
impl LauncherInstance {
    pub fn rebuild_areas(&self, areas: &[String], entries: &HashMap<String, ConfigEntry>) {
        if let Ok(mut area_manager) = self.area_manager.lock() {
            // Remove all existing areas
            area_manager.clear_areas();

            // Add new areas from the resolved profile
            for area_id in areas {
                if let Some(ConfigEntry::Area(area_config)) = entries.get(area_id) {
                    if let Err(error) = area_manager.add_area_from_config(area_id, area_config.clone()) {
                        error!("Failed to add area {area_id}: {error}");
                    }
                }
            }
        }
    }
}
```

**Note:** `AreaManager::clear_areas()` does not exist yet and must be implemented as part of this
feature. It should unload all plugins in each area before removing the area widgets.

## 7. Affected Files

| File                                                  | Change                                                                                                |
|-------------------------------------------------------|-------------------------------------------------------------------------------------------------------|
| `smearor-swipe-launcher/src/config/layout/trigger.rs` | Add `MonitorIndex(u32)` and `MonitorIndexWorkspace { monitor: u32, workspace: i32 }` variants         |
| `smearor-swipe-launcher/src/config/launcher.rs`       | Update `get_layout_for_context` to accept `monitor_index: Option<u32>` and match index-based triggers |
| `smearor-swipe-launcher/src/instance.rs`              | Call `get_layout_for_context` in `build_window` and `load_plugins`                                    |
| `smearor-swipe-launcher/src/area/area_manager.rs`     | Add `clear_areas()` method for runtime layout switching                                               |

## 8. Edge Cases

- **No profiles defined** — `get_layout_for_context` returns the default layout. Behavior is
  identical to the current implementation.
- **No matching profile** — Falls back to the default layout (`self.areas` / `self.entries`).
- **Multiple profiles match** — The first matching profile in declaration order wins. Users should
  declare more specific profiles first.
- **Monitor index changes at runtime** — The hotplug rebuild from `LAYER_SHELL_WINDOW_CONCEPT.md`
  automatically re-evaluates the profile.
- **Profile references undefined area** — Caught by `validate_layout_profile` at config load time,
  which already checks that all area IDs in `profile.areas` exist in `profile.entries`.

## 9. Implementation Roadmap

### Phase 1: Trigger Extension

| #   | Task                                                                 | Files                      | Effort |
|-----|----------------------------------------------------------------------|----------------------------|--------|
| 1.1 | Add `MonitorIndex(u32)` variant to `LayoutTrigger`                   | `config/layout/trigger.rs` | Small  |
| 1.2 | Add `MonitorIndexWorkspace { monitor: u32, workspace: i32 }` variant | `config/layout/trigger.rs` | Small  |

### Phase 2: Context Resolution

| #   | Task                                                                | Files                | Effort |
|-----|---------------------------------------------------------------------|----------------------|--------|
| 2.1 | Update `get_layout_for_context` signature to accept `monitor_index` | `config/launcher.rs` | Small  |
| 2.2 | Add matching logic for `MonitorIndex` and `MonitorIndexWorkspace`   | `config/launcher.rs` | Small  |

### Phase 3: Instance Integration

| #   | Task                                            | Files         | Effort |
|-----|-------------------------------------------------|---------------|--------|
| 3.1 | Call `get_layout_for_context` in `build_window` | `instance.rs` | Small  |
| 3.2 | Call `get_layout_for_context` in `load_plugins` | `instance.rs` | Small  |

### Phase 4: Runtime Switching

| #   | Task                                              | Files                  | Effort |
|-----|---------------------------------------------------|------------------------|--------|
| 4.1 | Implement `clear_areas()` in `AreaManager`        | `area/area_manager.rs` | Medium |
| 4.2 | Implement `rebuild_areas()` in `LauncherInstance` | `instance.rs`          | Medium |

### Phase 5: Testing & Validation

| #   | Task                                           | Effort |
|-----|------------------------------------------------|--------|
| 5.1 | Test default layout when no profiles defined   | Small  |
| 5.2 | Test `MonitorIndex` profile selection          | Small  |
| 5.3 | Test fallback when no profile matches          | Small  |
| 5.4 | Test profile priority (first match wins)       | Small  |
| 5.5 | Test runtime layout switch on monitor hotplug  | Medium |
| 5.6 | Test `clear_areas()` unloads plugins correctly | Medium |

## 10. Dependencies

- **LAYER_SHELL_WINDOW_CONCEPT.md** — The monitor index (`config.launcher.layer.monitor`) is the
  primary input for `MonitorIndex` trigger matching. This concept depends on the monitor assignment
  feature being implemented first.
- **HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md** — The `Workspace` and `MonitorIndexWorkspace` triggers
  require workspace tracking, which is only available on Hyprland. Without workspace tracking, only
  `MonitorIndex` and `Default` triggers are functional.
