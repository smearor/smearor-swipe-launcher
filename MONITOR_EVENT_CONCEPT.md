# Monitor Event Concept

## 1. Goal

Extend the three compositor services (`services/hyprland`, `services/wayland`, `services/gnome`) with
monitor hotplug detection, improved monitor-index resolution, and workspace creation/deletion events.
All three services broadcast generic events via `model/workspace` so the launcher core remains
compositor-agnostic.

## 2. Scope

- **In scope:**
    - Monitor hotplug events (connect/disconnect) for all three compositors
    - Improved monitor-index resolution (connector name matching against GDK)
    - Workspace creation and deletion events
    - New model types in `model/workspace` for monitor and workspace lifecycle events
    - Launcher core handling of monitor and workspace lifecycle events
- **Out of scope:**
    - Workspace tracking (active workspace changes) — already implemented, see
      `WORKSPACE_SWITCHING_CONCEPT.md`
    - Dispatch commands (workspace switching, window management) — Hyprland already has dispatch,
      Wayland/GNOME dispatch is a separate future concept
    - Widget-level workspace state (future concept)

## 3. Current State

### 3.1 Model Crate (`model/workspace`)

Currently defines only `WorkspaceChangedEvent` and `TOPIC_WORKSPACE_CHANGED`:

```rust
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceChangedEvent {
    pub workspace_name: stabby::string::String,
    pub workspace_id: i32,
    pub monitor_index: u32,
}
```

No monitor lifecycle or workspace lifecycle event types exist yet.

### 3.2 Hyprland Service (`services/hyprland`)

Already receives `MonitorAdded` and `MonitorRemoved` events from the `hyprland` crate's
`EventListener`, but only logs them — no broadcasting to the launcher core:

```rust
HyprlandEvent::MonitorAdded(name) => {
debug ! ("Monitor added: {}", name);
}
HyprlandEvent::MonitorRemoved(name) => {
debug ! ("Monitor removed: {}", name);
}
```

Monitor-index resolution uses `hyprctl monitors` via `hyprland::data::Monitors::get()`, matching
`monitor.active_workspace.id == workspace_id`. This works but does not provide connector names for
GDK matching.

### 3.3 Wayland Service (`services/wayland`)

The `WaylandState` struct tracks `wl_output` globals in `outputs` and assigns sequential indices
via `output_to_index` (Option B — bind order). When a `wl_output` global is removed
(`RegistryEvent::GlobalRemove`), it is only logged — no event is broadcast.

The `WlOutput` dispatch handler is empty — output geometry, mode, and name events are ignored.
`xdg_output` protocol is not bound, so connector names are unavailable for GDK matching.

Workspace creation (`ManagerEvent::Workspace`) and removal (`WorkspaceHandleEvent::Removed`) are
tracked internally but not broadcast as events.

### 3.4 GNOME Service (`services/gnome`)

`MutterDisplayConfigProxy` is already defined in `workspace/dbus.rs` with `get_resources()` and
`get_current_state()` methods, but `resolve_monitor_index()` always returns `0` — the proxy is
unused.

No `MonitorsChanged` signal subscription exists. No workspace creation/deletion detection.

### 3.5 Launcher Core

`LauncherInstance::on_workspace_changed` handles `WorkspaceChangedEvent` by re-evaluating layout
profiles. No handler exists for monitor hotplug or workspace lifecycle events.

## 4. Architecture

### 4.1 Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Compositor Services                                    │
│                                                                               │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐            │
│  │  HyprlandService │  │  WaylandService   │  │  GnomeService   │            │
│  │  (IPC socket)    │  │  (ext-workspace) │  │  (D-Bus)        │            │
│  │                  │  │  + xdg-output     │  │  + MonitorsChg  │            │
│  │                  │  │  + wl_output      │  │  + DisplayConfig│            │
│  │                  │  │                  │  │                 │            │
│  │  workspace      │  │  workspace       │  │  workspace      │            │
│  │  change ────────┼──┤  change ─────────┼──┤  change ────────┼──┐         │
│  │  monitor        │  │  monitor         │  │  monitor        │  │         │
│  │  added/removed ─┼──┤  added/removed ──┼──┤  added/removed ─┼──┤         │
│  │  workspace      │  │  workspace       │  │  workspace      │  │         │
│  │  created/removed┼──┤  created/removed ─┼──┤  created/removed┼──┤         │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘  │         │
│                                                                     │         │
│  All three broadcast generic events via model/workspace             │         │
└─────────────────────────────────────────────────────────────────────┼─────────┘
                                                                      │
                                                                      ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        Launcher Core                                             │
│                                                                                 │
│  LauncherHost receives events                                                     │
│  → routes to all LauncherInstances                                                │
│  → on_workspace_changed()     → re-evaluate layout profile                        │
│  → on_monitor_changed()       → re-evaluate monitor mapping, rebuild areas       │
│  → on_workspace_lifecycle()   → update workspace state (future widget use)        │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Event Flow

1. Compositor emits a monitor or workspace lifecycle event (IPC, Wayland protocol, or D-Bus signal).
2. The service-specific event listener receives and parses it.
3. The service resolves monitor information (connector name, GDK index) where applicable.
4. The service constructs a generic event (`MonitorChangedEvent`, `WorkspaceLifecycleEvent`).
5. The service broadcasts the event via `FfiCoreContext` to all launcher instances.
6. Each launcher instance reacts accordingly (rebuild areas, update state).

## 5. New Model Types (`model/workspace`)

### 5.1 Monitor Changed Event

Broadcast when a monitor is connected or disconnected. The launcher uses this to re-evaluate
monitor mappings and rebuild areas.

```rust
/// Topic for monitor change events broadcast by compositor services.
pub const TOPIC_MONITOR_CHANGED: &str = "compositor::monitor_changed";

/// Event broadcast when a monitor is connected or disconnected.
///
/// Launcher instances use this to re-evaluate monitor mappings and rebuild areas.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct MonitorChangedEvent {
    /// The monitor index (0-based, matching GDK display order).
    pub monitor_index: u32,
    /// The connector name of the monitor (e.g. "HDMI-A-1", "eDP-1").
    pub connector_name: stabby::string::String,
    /// Whether the monitor was connected or disconnected.
    pub change_type: MonitorChangeType,
}

/// Type of monitor change.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum MonitorChangeType {
    /// Monitor was connected.
    #[default]
    Connected,
    /// Monitor was disconnected.
    Disconnected,
}

impl TypedMessage for MonitorChangedEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::MonitorChangedEvent");
}

impl MessageTopic for MonitorChangedEvent {
    fn topic() -> &'static str {
        TOPIC_MONITOR_CHANGED
    }
}

impl SharedMessage for MonitorChangedEvent {
    fn topic(&self) -> &'static str {
        TOPIC_MONITOR_CHANGED
    }
}
```

### 5.2 Workspace Lifecycle Event

Broadcast when a workspace is created or destroyed. Useful for widgets that display workspace
lists or for the launcher to track available workspaces.

```rust
/// Topic for workspace lifecycle events broadcast by compositor services.
pub const TOPIC_WORKSPACE_LIFECYCLE: &str = "compositor::workspace_lifecycle";

/// Event broadcast when a workspace is created or destroyed.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceLifecycleEvent {
    /// The workspace name or number.
    pub workspace_name: stabby::string::String,
    /// The workspace ID (numeric, as reported by the compositor).
    pub workspace_id: i32,
    /// The monitor index the workspace is on, if known.
    pub monitor_index: u32,
    /// Whether the workspace was created or destroyed.
    pub lifecycle_type: WorkspaceLifecycleType,
}

/// Type of workspace lifecycle event.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum WorkspaceLifecycleType {
    /// Workspace was created.
    #[default]
    Created,
    /// Workspace was destroyed.
    Destroyed,
}

impl TypedMessage for WorkspaceLifecycleEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::WorkspaceLifecycleEvent");
}

impl MessageTopic for WorkspaceLifecycleEvent {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_LIFECYCLE;
    }
}

impl SharedMessage for WorkspaceLifecycleEvent {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_LIFECYCLE
    }
}
```

### 5.3 Updated `model/workspace/src/lib.rs`

```rust
pub mod messages;

pub use messages::MonitorChangedEvent;
pub use messages::MonitorChangeType;
pub use messages::TOPIC_MONITOR_CHANGED;
pub use messages::TOPIC_WORKSPACE_CHANGED;
pub use messages::TOPIC_WORKSPACE_LIFECYCLE;
pub use messages::WorkspaceChangedEvent;
pub use messages::WorkspaceLifecycleEvent;
pub use messages::WorkspaceLifecycleType;
```

## 6. Service: `services/wayland`

### 6.1 Monitor Hotplug Events

#### Current State

`RegistryEvent::Global` binds `wl_output` globals and assigns sequential indices.
`RegistryEvent::GlobalRemove` only logs — no event is broadcast.

#### Implementation

Extend `WaylandState` with a `connector_names: HashMap<ObjectId, String>` field populated from
`xdg_output` events. When a `wl_output` global appears or disappears, broadcast a
`MonitorChangedEvent`.

Bind the `xdg_output_manager_v1` global in the registry handler. For each `wl_output`, create an
`xdg_output` and listen for `name` and `description` events to get the connector name.

```rust
// In Dispatch<WlRegistry, ()> for WaylandState:
RegistryEvent::Global { name, interface, version } => {
if interface == "ext_workspace_manager_v1" {
// ... existing binding ...
} else if interface == "wl_output" {
let output = registry.bind::< WlOutput, (), WaylandState > (name, version.min(4), qh, ());
let id = output.id();
let index = state.next_output_index;
state.next_output_index += 1;
state.output_to_index.insert(id.clone(), index);
state.outputs.insert(id.clone(), output);

// Broadcast monitor connected event.
let event = MonitorChangedEvent {
monitor_index: index,
connector_name: String::new().into(), // Filled later from xdg_output.
change_type: MonitorChangeType::Connected,
};
let _ = state.sender.send(WorkspaceEvent::MonitorChanged(event));
} else if interface == "xdg_output_manager_v1" {
let manager = registry.bind::< ZxdgOutputManagerV1, (), WaylandState > (name, version.min(3), qh, ());
state.xdg_output_manager = Some(manager);
}
}

RegistryEvent::GlobalRemove { name } => {
// Find the output by its global name and broadcast MonitorChangedEvent::Disconnected.
if let Some((id, index)) = state.find_output_by_global_name(name) {
let connector_name = state.connector_names.get( &id).cloned().unwrap_or_default();
let event = MonitorChangedEvent {
monitor_index: index,
connector_name: connector_name.into(),
change_type: MonitorChangeType::Disconnected,
};
let _ = state.sender.send(WorkspaceEvent::MonitorChanged(event));
state.outputs.remove( & id);
state.output_to_index.remove( & id);
state.connector_names.remove( & id);
}
}
```

#### xdg_output Dispatch

```rust
impl Dispatch<ZxdgOutputV1, ObjectId> for WaylandState {
    fn event(state: &mut WaylandState, xdg_output: &ZxdgOutputV1, event: XdgOutputEvent, _: &ObjectId, _: &Connection, _: &QueueHandle<WaylandState>) {
        let output_id = xdg_output.id();
        match event {
            XdgOutputEvent::Name { name } => {
                state.connector_names.insert(output_id, name);
            }
            _ => {}
        }
    }
}
```

### 6.2 Monitor-Index Resolution Improvement

#### Current State

Uses output bind order (Option B). No connector name matching against GDK.

#### Implementation

After `xdg_output` provides the connector name, store it in `connector_names`. The launcher core
can match connector names against GDK's `Monitor::connector()` for accurate index mapping.

The `MonitorChangedEvent` carries the `connector_name`, so the launcher can build a
connector-to-GDK-index map at startup and update it on hotplug events.

For `WorkspaceChangedEvent`, the monitor index is resolved via the workspace group's output →
`output_to_index` mapping. With `xdg_output` names available, the launcher can additionally verify
or correct the index using the connector name.

### 6.3 Workspace Creation and Deletion

#### Current State

`ManagerEvent::Workspace` and `WorkspaceHandleEvent::Removed` are tracked internally but not
broadcast.

#### Implementation

Extend `WorkspaceEvent` enum and broadcast `WorkspaceLifecycleEvent`:

```rust
pub enum WorkspaceEvent {
    WorkspaceChanged(WorkspaceChangedEvent),
    MonitorChanged(MonitorChangedEvent),
    WorkspaceLifecycle(WorkspaceLifecycleEvent),
}
```

In `Dispatch<ExtWorkspaceManagerV1, ()>`:

```rust
ManagerEvent::Workspace { workspace } => {
let id: ObjectId = workspace.id();
debug ! ("Workspace created: {id}");
state.workspaces.insert(id.clone(), WorkspaceInfo { /* ... */ });

// Broadcast workspace created event (after Done, when id/name are known).
// Deferred to process_done() or sent immediately with partial info.
}
```

In `Dispatch<ExtWorkspaceHandleV1, ()>`:

```rust
WorkspaceHandleEvent::Removed => {
let ws_id: ObjectId = workspace.id();
if let Some(ws) = state.workspaces.get( & ws_id) {
let monitor_index = ws.group_id
.as_ref()
.and_then( | gid | state.groups.get(gid))
.and_then( | g| g.output_ids.first())
.and_then( | oid | state.output_to_index.get(oid))
.copied()
.unwrap_or(0);

let id_num = ws.id.parse::< i32 > ().or_else( | _ | ws.name.parse::< i32 > ()).unwrap_or( -1);
let event = WorkspaceLifecycleEvent {
workspace_name: ws.name.clone().into(),
workspace_id: id_num,
monitor_index,
lifecycle_type: WorkspaceLifecycleType::Destroyed,
};
let _ = state.sender.send(WorkspaceEvent::WorkspaceLifecycle(event));
}
state.workspaces.remove( & ws_id);
}
```

For workspace creation, broadcast after the `Done` event when `id` and `name` are populated:

```rust
// In process_done(), after detecting a new workspace:
fn process_done(&mut self) {
    // ... existing active workspace detection ...

    // Detect newly created workspaces (workspaces with id/name that weren't seen before).
    for (_, ws) in &self.workspaces {
        if !ws.id.is_empty() && !self.broadcasted_workspaces.contains(&ws.id) {
            let monitor_index = /* resolve from group */;
            let event = WorkspaceLifecycleEvent {
                workspace_name: ws.name.clone().into(),
                workspace_id: ws.id.parse::<i32>().unwrap_or(-1),
                monitor_index,
                lifecycle_type: WorkspaceLifecycleType::Created,
            };
            let _ = self.sender.send(WorkspaceEvent::WorkspaceLifecycle(event));
            self.broadcasted_workspaces.insert(ws.id.clone());
        }
    }

    // Remove destroyed workspaces from broadcasted set.
    self.broadcasted_workspaces.retain(|id| self.workspaces.contains_key(/* by id string */));
}
```

### 6.4 Updated `WaylandState`

```rust
pub struct WaylandState {
    pub sender: mpsc::UnboundedSender<WorkspaceEvent>,
    pub manager: Option<ExtWorkspaceManagerV1>,
    pub xdg_output_manager: Option<ZxdgOutputManagerV1>,
    pub workspaces: HashMap<ObjectId, WorkspaceInfo>,
    pub groups: HashMap<ObjectId, GroupInfo>,
    pub outputs: HashMap<ObjectId, WlOutput>,
    pub output_to_index: HashMap<ObjectId, u32>,
    pub connector_names: HashMap<ObjectId, String>,
    pub next_output_index: u32,
    pub last_active: Option<(String, i32, u32)>,
    pub broadcasted_workspaces: HashSet<String>,
}
```

### 6.5 New Dependencies

| Crate               | Purpose                                   |
|---------------------|-------------------------------------------|
| `wayland-protocols` | Already includes `xdg-output-unstable-v1` |

No new crate dependencies needed — `xdg-output-unstable-v1` is part of `wayland-protocols`.

## 7. Service: `services/gnome`

### 7.1 Monitor Hotplug Events

#### Current State

`MutterDisplayConfigProxy` exists but is unused. No `MonitorsChanged` signal subscription.

#### Implementation

Subscribe to `org.gnome.Mutter.DisplayConfig.MonitorsChanged` signal. When it fires, query
`GetResources()` (or `GetCurrentState()` on GNOME 46+) to get the current monitor list. Compare
with the previous monitor list to detect additions and removals. Broadcast
`MonitorChangedEvent` for each change.

```rust
// In poll_workspace_loop, add signal subscription:
use zbus::fdo::PropertyStream;

// Subscribe to MonitorsChanged signal.
let monitor_changed = display_proxy.receive_signal("MonitorsChanged").await;

// In a separate task or interleaved with polling:
while let Some(_signal) = monitor_changed.next().await {
let monitors = query_monitors( & display_proxy).await.unwrap_or_default();
let changed = detect_monitor_changes(& previous_monitors, & monitors);
for change in changed {
let event = MonitorChangedEvent {
monitor_index: change.index,
connector_name: change.connector.into(),
change_type: change.change_type,
};
let _ = sender.send(WorkspaceEvent::MonitorChanged(event));
}
previous_monitors = monitors;
}
```

#### Monitor Query Helper

```rust
struct MonitorInfo {
    index: u32,
    connector: String,
}

async fn query_monitors(display_proxy: &MutterDisplayConfigProxy<'_>) -> Result<Vec<MonitorInfo>, zbus::Error> {
    let (serial, monitors, _logical, _props) = display_proxy.get_resources().await?;

    let mut result = Vec::new();
    for (index, (connector, _modes)) in monitors.into_iter().enumerate() {
        result.push(MonitorInfo {
            index: index as u32,
            connector,
        });
    }
    Ok(result)
}

fn detect_monitor_changes(previous: &[MonitorInfo], current: &[MonitorInfo]) -> Vec<MonitorChangedEvent> {
    let mut events = Vec::new();

    // Detect connected monitors.
    for mon in current {
        if !previous.iter().any(|p| p.connector == mon.connector) {
            events.push(MonitorChangedEvent {
                monitor_index: mon.index,
                connector_name: mon.connector.clone().into(),
                change_type: MonitorChangeType::Connected,
            });
        }
    }

    // Detect disconnected monitors.
    for mon in previous {
        if !current.iter().any(|c| c.connector == mon.connector) {
            events.push(MonitorChangedEvent {
                monitor_index: mon.index,
                connector_name: mon.connector.clone().into(),
                change_type: MonitorChangeType::Disconnected,
            });
        }
    }

    events
}
```

### 7.2 Monitor-Index Resolution Improvement

#### Current State

`resolve_monitor_index()` always returns `0`.

#### Implementation

Parse `GetResources()` return value to find the primary monitor. The monitor list from
`GetResources()` contains connector names. Match the connector name against GDK's
`Monitor::connector()` to get the correct GDK monitor index.

For workspace events, GNOME workspaces span all monitors by default. The monitor index should be
the primary monitor's index. Query the primary monitor from `GetResources()` properties or from
`GetCurrentState()` (GNOME 46+).

```rust
async fn resolve_monitor_index(display_proxy: &MutterDisplayConfigProxy<'_>) -> Option<u32> {
    let (serial, monitors, logical_monitors, _props) = display_proxy.get_resources().await.ok()?;

    // Find the primary logical monitor (index 0 in logical_monitors typically).
    // Each logical monitor is: (x, y, scale, primary, monitors).
    for (index, logical) in logical_monitors.iter().enumerate() {
        let (_x, _y, _scale, primary, _monitors) = logical;
        if *primary {
            return Some(index as u32);
        }
    }

    // Fallback: first logical monitor.
    if !logical_monitors.is_empty() {
        return Some(0);
    }

    None
}
```

### 7.3 Workspace Creation and Deletion

#### Current State

No workspace lifecycle detection. Only active workspace changes are polled.

#### Implementation

Query the total number of workspaces via `org.gnome.Shell.Eval`:

```rust
let js_count = "global.workspace_manager.get_n_workspaces()";
let count = shell_proxy.eval(js_count).await
.ok()
.and_then( | (success, result) | if success { result.trim().parse::<i32>().ok() } else { None });
```

Compare with the previous count. If increased, broadcast `WorkspaceLifecycleType::Created` for the
new workspace(s). If decreased, broadcast `WorkspaceLifecycleType::Destroyed` for the removed
workspace(s).

```rust
if let Some(current_count) = count {
if let Some(prev_count) = last_workspace_count {
if current_count > prev_count {
for i in prev_count..current_count {
let js_name = format ! ("global.workspace_manager.get_workspace_by_index({i}).title() || '{i}'");
let name = shell_proxy.eval( & js_name).await
.ok()
.and_then( | (success, result) | if success { Some(result.trim().to_string()) } else { None })
.unwrap_or_else( | | i.to_string());

let event = WorkspaceLifecycleEvent {
workspace_name: name.into(),
workspace_id: i,
monitor_index: 0,
lifecycle_type: WorkspaceLifecycleType::Created,
};
let _ = sender.send(WorkspaceEvent::WorkspaceLifecycle(event));
}
} else if current_count < prev_count {
for i in current_count..prev_count {
let event = WorkspaceLifecycleEvent {
workspace_name: i.to_string().into(),
workspace_id: i,
monitor_index: 0,
lifecycle_type: WorkspaceLifecycleType::Destroyed,
};
let _ = sender.send(WorkspaceEvent::WorkspaceLifecycle(event));
}
}
}
last_workspace_count = Some(current_count);
}
```

### 7.4 Updated `WorkspaceEvent` Enum

```rust
pub enum WorkspaceEvent {
    WorkspaceChanged(WorkspaceChangedEvent),
    MonitorChanged(MonitorChangedEvent),
    WorkspaceLifecycle(WorkspaceLifecycleEvent),
}
```

### 7.5 Updated Configuration

```rust
pub struct GnomeWorkspaceServiceConfig {
    #[serde(default)]
    pub enable_workspace_tracking: bool,
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Enable monitor hotplug detection via MonitorsChanged signal.
    #[serde(default = "default_enable_monitor_events")]
    pub enable_monitor_events: bool,
    /// Enable workspace creation/deletion detection.
    #[serde(default = "default_enable_workspace_lifecycle")]
    pub enable_workspace_lifecycle: bool,
}

fn default_enable_monitor_events() -> bool {
    true
}

fn default_enable_workspace_lifecycle() -> bool {
    true
}
```

## 8. Service: `services/hyprland`

### 8.1 Monitor Hotplug Events

#### Current State

`MonitorAdded` and `MonitorRemoved` events are received but only logged.

#### Implementation

Broadcast `MonitorChangedEvent` instead of just logging:

```rust
HyprlandEvent::MonitorAdded(name) => {
debug ! ("Monitor added: {}", name);
let monitor_index = resolve_monitor_index_by_name( & name).await.unwrap_or(0);
let event = MonitorChangedEvent {
monitor_index,
connector_name: name.into(),
change_type: MonitorChangeType::Connected,
};
broadcast_event( & event_core_context, & event_meta, event);
}
HyprlandEvent::MonitorRemoved(name) => {
debug ! ("Monitor removed: {}", name);
let monitor_index = resolve_monitor_index_by_name( & name).await.unwrap_or(0);
let event = MonitorChangedEvent {
monitor_index,
connector_name: name.into(),
change_type: MonitorChangeType::Disconnected,
};
broadcast_event( & event_core_context, & event_meta, event);
}
```

Add a helper to resolve monitor index by name:

```rust
async fn resolve_monitor_index_by_name(name: &str) -> Option<u32> {
    let monitors = match hyprland::data::Monitors::get() {
        Ok(monitors) => monitors,
        Err(error) => {
            warn!("Failed to query monitors for '{name}': {error}");
            return None;
        }
    };
    for monitor in monitors {
        if monitor.name == name {
            return Some(monitor.id as u32);
        }
    }
    None
}
```

### 8.2 Monitor-Index Resolution Improvement

#### Current State

`resolve_monitor_for_workspace` matches `monitor.active_workspace.id == workspace_id`. This works
but relies on Hyprland's internal monitor IDs, which may not match GDK indices.

#### Implementation

Hyprland's `hyprctl monitors` returns connector names (`monitor.name`) and Hyprland-internal IDs
(`monitor.id`). The connector name can be matched against GDK's `Monitor::connector()`.

For the `WorkspaceChangedEvent`, the monitor index should be the Hyprland ID (which is typically
0-based and matches the order monitors were connected). If GDK ordering differs, the launcher core
can remap using connector names from `MonitorChangedEvent`.

No change needed to `resolve_monitor_for_workspace` — it already returns `monitor.id as u32`. The
improvement is in broadcasting connector names via `MonitorChangedEvent` so the launcher can build
a connector-to-index map.

### 8.3 Workspace Creation and Deletion

#### Current State

Not implemented. Hyprland's `EventListener` does not have explicit workspace created/removed
handlers, but workspace changes can be inferred.

#### Implementation

Hyprland broadcasts `WorkspaceChanged` events that include the workspace ID. Track known workspace
IDs in the event worker. When a new ID appears, broadcast `WorkspaceLifecycleType::Created`. When
an ID disappears (workspace destroyed), broadcast `WorkspaceLifecycleType::Destroyed`.

```rust
// In the event worker:
let mut known_workspaces: HashSet<i32> = HashSet::new();

HyprlandEvent::WorkspaceChanged(event) => {
if ! known_workspaces.contains( & event.workspace_id) {
let lifecycle_event = WorkspaceLifecycleEvent {
workspace_name: event.workspace_name.clone(),
workspace_id: event.workspace_id,
monitor_index: event.monitor_index,
lifecycle_type: WorkspaceLifecycleType::Created,
};
broadcast_event( & event_core_context, & event_meta, lifecycle_event);
known_workspaces.insert(event.workspace_id);
}

// ... existing broadcast logic ...
}
```

For workspace destruction, query `hyprctl workspaces` periodically or on workspace change events
to detect removed IDs:

```rust
async fn detect_removed_workspaces(known: &mut HashSet<i32>) -> Vec<WorkspaceLifecycleEvent> {
    let current = match hyprland::data::Workspaces::get() {
        Ok(workspaces) => workspaces,
        Err(_) => return Vec::new(),
    };

    let current_ids: HashSet<i32> = current.iter().map(|ws| ws.id).collect();
    let removed: Vec<WorkspaceLifecycleEvent> = known
        .difference(&current_ids)
        .map(|id| WorkspaceLifecycleEvent {
            workspace_name: id.to_string().into(),
            workspace_id: *id,
            monitor_index: 0,
            lifecycle_type: WorkspaceLifecycleType::Destroyed,
        })
        .collect();
    known.retain(|id| current_ids.contains(id));
    removed
}
```

### 8.4 Updated `HyprlandEvent` Enum

```rust
enum HyprlandEvent {
    WorkspaceChanged(WorkspaceChangedEvent),
    MonitorAdded(String),
    MonitorRemoved(String),
}
```

No change needed — the existing enum already carries the data. The event worker is extended to
broadcast `MonitorChangedEvent` and `WorkspaceLifecycleEvent`.

### 8.5 Updated Configuration

```rust
pub struct HyprlandServiceConfig {
    // ... existing fields ...
    /// Enable monitor hotplug event broadcasting.
    #[serde(default = "default_enable_monitor_events")]
    pub enable_monitor_events: bool,
    /// Enable workspace creation/deletion event broadcasting.
    #[serde(default = "default_enable_workspace_lifecycle")]
    pub enable_workspace_lifecycle: bool,
}
```

## 9. Launcher Core Integration

### 9.1 Message Routing

`LauncherInstance::handle_message` in `smearor-swipe-launcher/src/messages/mod.rs` needs to handle
the two new event types:

```rust
// Handle monitor change events.
if envelope.type_id == FfiEnvelopePayload::<MonitorChangedEvent>::TYPE_ID {
if ! envelope.payload.is_null() {
let event = unsafe { & * (envelope.payload as * const MonitorChangedEvent) };
self.on_monitor_changed(event.monitor_index, &event.connector_name, event.change_type);
}
// ... destroy payload ...
return;
}

// Handle workspace lifecycle events.
if envelope.type_id == FfiEnvelopePayload::<WorkspaceLifecycleEvent>::TYPE_ID {
if ! envelope.payload.is_null() {
let event = unsafe { & * (envelope.payload as * const WorkspaceLifecycleEvent) };
self.on_workspace_lifecycle(event.workspace_id, event.monitor_index, event.lifecycle_type);
}
// ... destroy payload ...
return;
}
```

### 9.2 `LauncherInstance` Methods

```rust
impl LauncherInstance {
    /// Handle a monitor hotplug event.
    ///
    /// Re-evaluates the monitor mapping and rebuilds areas if the monitor
    /// configuration affects this instance.
    pub fn on_monitor_changed(&self, monitor_index: u32, connector_name: &str, change_type: MonitorChangeType) {
        debug!(
            "Instance {} monitor {} ({}): {:?}",
            self.instance_id, monitor_index, connector_name, change_type
        );
        // Re-evaluate layout with updated monitor context.
        let (areas, entries) = self.config.get_layout_for_context(Some(connector_name), Some(monitor_index), None);
        self.rebuild_areas(areas, entries);
    }

    /// Handle a workspace lifecycle event.
    ///
    /// Currently informational — future widgets may use this to display
    /// workspace lists or update state.
    pub fn on_workspace_lifecycle(&self, workspace_id: i32, monitor_index: u32, lifecycle_type: WorkspaceLifecycleType) {
        debug!(
            "Instance {} workspace {} on monitor {}: {:?}",
            self.instance_id, workspace_id, monitor_index, lifecycle_type
        );
        // No layout rebuild needed — workspace creation/deletion does not change
        // the active workspace. Future widget integrations can hook in here.
    }
}
```

### 9.3 JSON Converter Registration

Register JSON converters for `MonitorChangedEvent` and `WorkspaceLifecycleEvent` in the launcher
core startup so the message system can deserialize them.

## 10. Edge Cases

### 10.1 General

- **Multiple services active:** If both `services/hyprland` and `services/wayland` are active,
  duplicate monitor and workspace lifecycle events may fire. The launcher handles this gracefully
  (idempotent rebuild), but it is wasteful.
- **Rapid monitor hotplug:** Connect/disconnect cycles can trigger many events. A debounce (e.g.
  200ms) can be added to the event worker if flicker becomes an issue.
- **No compositor running:** The service fails to connect, logs an error, and retries. The
  launcher continues without monitor or workspace lifecycle events.

### 10.2 Wayland

- **`xdg_output` not supported:** Some compositors may not advertise `xdg_output_manager_v1`.
  Fall back to bind-order index and empty connector names.
- **`ext-workspace` not supported:** The compositor does not advertise
  `ext_workspace_manager_v1`. No workspace lifecycle events are broadcast. Monitor events still
  work via `wl_output`.
- **Output removed while active workspace:** If the output is removed that had the active
  workspace, the compositor will move the workspace to another output. The resulting
  `WorkspaceChangedEvent` handles this.

### 10.3 GNOME

- **`org.gnome.Shell.Eval` restricted:** Some GNOME versions restrict `Eval`. Workspace lifecycle
  detection falls back to `org.gnome.Shell.Introspect.GetWindows()` (GNOME 46+).
- **`MonitorsChanged` signal timing:** The signal may fire multiple times during a monitor
  hotplug. Debounce in the service to avoid duplicate events.
- **GNOME on X11:** D-Bus interfaces are the same, but monitor index mapping may differ from
  Wayland. GDK handles this transparently.

### 10.4 Hyprland

- **Workspace ID reuse:** Hyprland may reuse workspace IDs after destruction. The
  `known_workspaces` set should be cleared on reconnection to avoid stale state.
- **Monitor name changes:** Hyprland may rename monitors on reconnection. The
  `resolve_monitor_index_by_name` helper handles this by querying the current monitor list.

## 11. Reconnection Strategy

All three services follow the same reconnection pattern (5-second retry loop). On reconnect, the
internal state (known workspaces, monitor mappings, connector names) is reset and rebuilt from
scratch. This ensures consistency after compositor restarts.

## 12. Affected Files

### 12.1 New Files

| File                                       | Description |
|--------------------------------------------|-------------|
| (none — all changes are in existing files) |             |

### 12.2 Modified Files

| File                                         | Change                                                                                                                                                               |
|----------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `model/workspace/src/messages.rs`            | Add `MonitorChangedEvent`, `MonitorChangeType`, `WorkspaceLifecycleEvent`, `WorkspaceLifecycleType`, topics                                                          |
| `model/workspace/src/lib.rs`                 | Re-export new types                                                                                                                                                  |
| `services/wayland/src/workspace/state.rs`    | Add `xdg_output_manager`, `connector_names`, `broadcasted_workspaces` fields; add `WorkspaceEvent::MonitorChanged` and `WorkspaceEvent::WorkspaceLifecycle` variants |
| `services/wayland/src/workspace/tracker.rs`  | Bind `xdg_output_manager_v1`, dispatch `ZxdgOutputV1`, broadcast monitor and workspace lifecycle events                                                              |
| `services/wayland/src/workspace/mod.rs`      | Re-export new event types                                                                                                                                            |
| `services/wayland/src/service.rs`            | Handle new `WorkspaceEvent` variants in event worker                                                                                                                 |
| `services/gnome/src/workspace/tracker.rs`    | Subscribe to `MonitorsChanged`, query monitors, detect workspace count changes, broadcast events                                                                     |
| `services/gnome/src/workspace/dbus.rs`       | (already has `MutterDisplayConfigProxy` — no change needed)                                                                                                          |
| `services/gnome/src/workspace/mod.rs`        | Re-export new event types                                                                                                                                            |
| `services/gnome/src/service.rs`              | Handle new `WorkspaceEvent` variants in event worker                                                                                                                 |
| `services/gnome/src/config.rs`               | Add `enable_monitor_events`, `enable_workspace_lifecycle` fields                                                                                                     |
| `services/hyprland/src/service.rs`           | Broadcast `MonitorChangedEvent` on monitor added/removed; track workspace IDs and broadcast `WorkspaceLifecycleEvent`                                                |
| `services/hyprland/src/config.rs`            | Add `enable_monitor_events`, `enable_workspace_lifecycle` fields                                                                                                     |
| `services.toml`                              | Add new config keys for `[wayland]`, `[gnome]`, `[hyprland]` sections                                                                                                |
| `smearor-swipe-launcher/src/messages/mod.rs` | Route `MonitorChangedEvent` and `WorkspaceLifecycleEvent` to instance                                                                                                |
| `smearor-swipe-launcher/src/instance.rs`     | Add `on_monitor_changed`, `on_workspace_lifecycle` methods                                                                                                           |

## 13. Implementation Roadmap

### Phase 1: Model Types (`model/workspace`)

| #   | Task                                                       | Files                             | Effort |
|-----|------------------------------------------------------------|-----------------------------------|--------|
| 1.1 | Add `MonitorChangedEvent` and `MonitorChangeType`          | `model/workspace/src/messages.rs` | Small  |
| 1.2 | Add `WorkspaceLifecycleEvent` and `WorkspaceLifecycleType` | `model/workspace/src/messages.rs` | Small  |
| 1.3 | Add topics and `TypedMessage` / `SharedMessage` impls      | `model/workspace/src/messages.rs` | Small  |
| 1.4 | Re-export new types from `lib.rs`                          | `model/workspace/src/lib.rs`      | Small  |
| 1.5 | Build and verify                                           | —                                 | Small  |

### Phase 2: Wayland Service (`services/wayland`)

| #   | Task                                                                   | Files                                       | Effort |
|-----|------------------------------------------------------------------------|---------------------------------------------|--------|
| 2.1 | Add `xdg_output_manager` and `connector_names` to `WaylandState`       | `services/wayland/src/workspace/state.rs`   | Small  |
| 2.2 | Add `WorkspaceEvent::MonitorChanged` and `WorkspaceLifecycle` variants | `services/wayland/src/workspace/state.rs`   | Small  |
| 2.3 | Bind `xdg_output_manager_v1` in registry handler                       | `services/wayland/src/workspace/tracker.rs` | Small  |
| 2.4 | Implement `ZxdgOutputV1` dispatch for connector names                  | `services/wayland/src/workspace/tracker.rs` | Medium |
| 2.5 | Broadcast `MonitorChangedEvent` on `wl_output` global add/remove       | `services/wayland/src/workspace/tracker.rs` | Medium |
| 2.6 | Broadcast `WorkspaceLifecycleEvent` on workspace created/removed       | `services/wayland/src/workspace/tracker.rs` | Medium |
| 2.7 | Handle new event variants in service event worker                      | `services/wayland/src/service.rs`           | Small  |
| 2.8 | Build and verify                                                       | —                                           | Small  |

### Phase 3: GNOME Service (`services/gnome`)

| #   | Task                                                                    | Files                                     | Effort |
|-----|-------------------------------------------------------------------------|-------------------------------------------|--------|
| 3.1 | Add `WorkspaceEvent::MonitorChanged` and `WorkspaceLifecycle` variants  | `services/gnome/src/workspace/tracker.rs` | Small  |
| 3.2 | Implement `MonitorsChanged` signal subscription                         | `services/gnome/src/workspace/tracker.rs` | Medium |
| 3.3 | Implement monitor query and change detection                            | `services/gnome/src/workspace/tracker.rs` | Medium |
| 3.4 | Implement workspace count polling for lifecycle events                  | `services/gnome/src/workspace/tracker.rs` | Medium |
| 3.5 | Implement `resolve_monitor_index` via `GetResources`                    | `services/gnome/src/workspace/tracker.rs` | Medium |
| 3.6 | Add config fields `enable_monitor_events`, `enable_workspace_lifecycle` | `services/gnome/src/config.rs`            | Small  |
| 3.7 | Handle new event variants in service event worker                       | `services/gnome/src/service.rs`           | Small  |
| 3.8 | Build and verify                                                        | —                                         | Small  |

### Phase 4: Hyprland Service (`services/hyprland`)

| #   | Task                                                                    | Files                              | Effort |
|-----|-------------------------------------------------------------------------|------------------------------------|--------|
| 4.1 | Broadcast `MonitorChangedEvent` on monitor added/removed                | `services/hyprland/src/service.rs` | Small  |
| 4.2 | Track known workspace IDs and broadcast `WorkspaceLifecycleEvent`       | `services/hyprland/src/service.rs` | Medium |
| 4.3 | Add config fields `enable_monitor_events`, `enable_workspace_lifecycle` | `services/hyprland/src/config.rs`  | Small  |
| 4.4 | Build and verify                                                        | —                                  | Small  |

### Phase 5: Launcher Core Integration

| #   | Task                                                     | Files                                        | Effort |
|-----|----------------------------------------------------------|----------------------------------------------|--------|
| 5.1 | Route `MonitorChangedEvent` in message handler           | `smearor-swipe-launcher/src/messages/mod.rs` | Small  |
| 5.2 | Route `WorkspaceLifecycleEvent` in message handler       | `smearor-swipe-launcher/src/messages/mod.rs` | Small  |
| 5.3 | Implement `on_monitor_changed` in `LauncherInstance`     | `smearor-swipe-launcher/src/instance.rs`     | Small  |
| 5.4 | Implement `on_workspace_lifecycle` in `LauncherInstance` | `smearor-swipe-launcher/src/instance.rs`     | Small  |
| 5.5 | Register JSON converters for new event types             | `smearor-swipe-launcher/src/`                | Small  |
| 5.6 | Update `services.toml` with new config keys              | `services.toml`                              | Small  |
| 5.7 | Build and verify                                         | —                                            | Small  |

### Phase 6: Testing & Validation

| #   | Task                                       | Effort |
|-----|--------------------------------------------|--------|
| 6.1 | Test Wayland monitor hotplug event on sway | Medium |
| 6.2 | Test Wayland workspace creation/deletion   | Medium |
| 6.3 | Test GNOME monitor hotplug event           | Medium |
| 6.4 | Test GNOME workspace creation/deletion     | Medium |
| 6.5 | Test Hyprland monitor hotplug event        | Medium |
| 6.6 | Test Hyprland workspace creation/deletion  | Medium |
| 6.7 | Test monitor-index resolution accuracy     | Medium |
| 6.8 | Test launcher core handles all new events  | Small  |

## 14. Dependencies

- **`model/workspace`** — All three services depend on the new event types. No new model crate is
  needed.
- **`WORKSPACE_SWITCHING_CONCEPT.md`** — This concept extends the workspace switching architecture
  with monitor and workspace lifecycle events.
- **`LAYOUT_PROFILE_CONCEPT.md`** — Monitor hotplug events may trigger layout profile
  re-evaluation with `MonitorIndex` or `MonitorWorkspace` triggers.
- **`LAYER_SHELL_WINDOW_CONCEPT.md`** — The monitor index from the layer shell config is used in
  `on_monitor_changed()` to filter events by monitor.
- **`SERVICE_PLUGIN_CONCEPT.md`** — All services follow the SOA architecture with
  `service_plugin!` macro, `MessageBroadcaster`, and `FfiCoreContext`.

## 15. Comparison Matrix

| Feature                   | Hyprland (IPC)                | Wayland (ext-workspace + xdg-output) | GNOME (D-Bus)                   |
|---------------------------|-------------------------------|--------------------------------------|---------------------------------|
| Monitor hotplug detection | `EventListener` events        | `wl_output` global add/remove        | `MonitorsChanged` signal        |
| Monitor connector name    | `hyprctl monitors`            | `xdg_output.name`                    | `DisplayConfig.GetResources`    |
| Monitor-index resolution  | `hyprctl monitors` (ID)       | `xdg_output` name → GDK matching     | `DisplayConfig` primary monitor |
| Workspace creation event  | Infer from `WorkspaceChanged` | `ManagerEvent::Workspace`            | Workspace count polling         |
| Workspace deletion event  | Infer from workspace list     | `WorkspaceHandleEvent::Removed`      | Workspace count polling         |
| Latency                   | Low (event-driven)            | Low (event-driven)                   | Medium (polling, 500ms)         |
| Extra dependencies        | `hyprland` crate              | `wayland-protocols` (xdg-output)     | `zbus`                          |
