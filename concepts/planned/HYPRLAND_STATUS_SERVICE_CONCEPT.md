# Concept: Hyprland Status Service Plugin (Phase 2)

This document describes the second implementation phase of the **Hyprland Service** in the *Smearor Swipe Launcher*. It builds on top of the command/dispatch
service defined in `HYPRLAND_SERVICE_CONCEPT.md` and adds live status broadcasting from the Hyprland compositor.

## Scope of this Phase

Phase 1 sends commands to Hyprland. Phase 2 listens for Hyprland events and broadcasts them as typed, FFI-stable messages to subscribed widgets. This enables UI
widgets such as workspace bars, active-window indicators, and monitor switchers. The consuming UI components are described in
[`HYPRLAND_WIDGET_CONCEPT.md`](./HYPRLAND_WIDGET_CONCEPT.md).

## 1. Architecture: Two-Tier Event Model

The launcher supports multiple compositors (Hyprland, GNOME). To avoid coupling widgets to a specific compositor, status events use a **two-tier model**:

1. **Compositor-unified events** — defined in `model/workspace` (`smearor-model-compositor`). These cover events that every compositor can produce: workspace
   changes, workspace lifecycle (created/destroyed), and monitor hotplug. Widgets like the Workspace Switcher subscribe to `compositor::*` topics and work
   identically under Hyprland or GNOME.

2. **Hyprland-specific events** — defined in `model/hyprland` (`smearor-hyprland-model`). These cover events that only Hyprland exposes (active window,
   fullscreen state, keyboard layout, submap, layer shell, float state, urgent, window title, workspace renamed, special workspaces, screencast, config reload,
   group lock, window pinning, window groups). They are broadcast on `service.hyprland.status` and are only relevant for Hyprland-aware widgets.

The `HyprlandService` translates raw `hyprland::event_listener` events into the appropriate tier: compositor-unified types for workspace/monitor events,
Hyprland-specific types for everything else.

```
+----------------------------+                 +----------------------------+
| Hyprland Compositor        |                 | Hyprland Service           |
| (socket2 event stream)     |                 | (Phase 1 + event listener) |
+----------------------------+                 +----------------------------+
             |                                             |
             | 1. Event (e.g., workspace>>9)                |
             |===========================================> |
             |                                             | 2a. Compositor-unified?
             |                                             |     -> WorkspaceChangedEvent
             |                                             |     -> Topic: compositor::workspace_changed
             |                                             |
             |                                             | 2b. Hyprland-specific?
             |                                             |     -> HyprlandStatusMessage
             |                                             |     -> Topic: service.hyprland.status
             |                                             |
             | 3. Broadcast                                 |
             | <===========================================|
+----------------------------+    +----------------------------+
| Workspace Switcher Widget  |    | Hyprland Window Widget     |
| (compositor-unified)       |    | (Hyprland-specific)        |
+----------------------------+    +----------------------------+
```

### 1.1 Compositor-Unified Topics (already implemented)

| Topic                             | Message Type               | Source Model Crate         |
|-----------------------------------|----------------------------|----------------------------|
| `compositor::workspace_changed`   | `WorkspaceChangedEvent`    | `smearor-model-compositor` |
| `compositor::workspace_lifecycle` | `WorkspaceLifecycleEvent`  | `smearor-model-compositor` |
| `compositor::monitor_changed`     | `MonitorChangedEvent`      | `smearor-model-compositor` |
| `compositor::workspace_snapshot`  | `WorkspaceSnapshotMessage` | `smearor-model-compositor` |

### 1.2 Hyprland-Specific Topics (to be implemented)

| Topic                              | Message Type                  | Source Model Crate       |
|------------------------------------|-------------------------------|--------------------------|
| `service.hyprland.status`          | `HyprlandStatusMessage`       | `smearor-hyprland-model` |
| `service.hyprland.status.request`  | `HyprlandStateRequestMessage` | `smearor-hyprland-model` |
| `service.hyprland.status.response` | `HyprlandStateMessage`        | `smearor-hyprland-model` |

## 2. Crate Structure

### 2.1 Compositor-Unified Model (`model/workspace` — already implemented)

The `smearor-model-compositor` crate provides compositor-agnostic message types. Both the Hyprland and GNOME services use these for workspace and monitor
events. **No changes needed in this crate for Phase 2.**

```
model/workspace/
  src/
    workspace.rs    # WorkspaceChangedEvent, WorkspaceLifecycleEvent
    monitor.rs      # MonitorChangedEvent
    switcher.rs     # SwitchWorkspaceMessage, CreateWorkspaceMessage, WorkspaceSnapshotMessage
    lib.rs          # re-exports, register_json_converters()
```

### 2.2 Hyprland-Specific Model (`model/hyprland` — to be extended)

Hyprland-specific status types go in `model/hyprland`. These are events that have no compositor-unified equivalent. One file per struct per `AGENTS.md`.

```
model/hyprland/
  src/
    messages/
      status/
        mod.rs                          # module declarations + re-exports
        active_window_changed.rs        # ActiveWindowChangedStatusMessage
        fullscreen_state_changed.rs     # FullscreenStateChangedStatusMessage
        window_opened.rs                # WindowOpenedStatusMessage
        window_closed.rs                # WindowClosedStatusMessage
        window_moved.rs                 # WindowMovedStatusMessage
        keyboard_layout_changed.rs      # KeyboardLayoutChangedStatusMessage
        sub_map_changed.rs              # SubMapChangedStatusMessage
        layer_opened.rs                 # LayerOpenedStatusMessage
        layer_closed.rs                 # LayerClosedStatusMessage
        float_state_changed.rs          # FloatStateChangedStatusMessage
        urgent_state_changed.rs         # UrgentStateChangedStatusMessage
        window_title_changed.rs         # WindowTitleChangedStatusMessage
        workspace_renamed.rs            # WorkspaceRenamedStatusMessage
        special_removed.rs              # SpecialRemovedStatusMessage
        changed_special.rs              # ChangedSpecialStatusMessage
        screencast.rs                   # ScreencastStatusMessage
        config_reloaded.rs              # ConfigReloadedStatusMessage
        ignore_group_lock_changed.rs    # IgnoreGroupLockStateChangedStatusMessage
        lock_groups_changed.rs          # LockGroupsStateChangedStatusMessage
        window_pinned.rs                # WindowPinnedStatusMessage
        group_toggled.rs                # GroupToggledStatusMessage
        window_moved_into_group.rs      # WindowMovedIntoGroupStatusMessage
        window_moved_out_of_group.rs    # WindowMovedOutOfGroupStatusMessage
      status_event.rs                   # HyprlandStatusEvent (unified enum)
      status_message.rs                 # HyprlandStatusMessage (envelope)
      state_request.rs                  # HyprlandStateRequestMessage + HyprlandStateMessage
      shared/
        mod.rs
        window_event_data.rs            # HyprlandWindowEventData
        window_open_event.rs            # HyprlandWindowOpenEvent
        window_move_event.rs            # HyprlandWindowMoveEvent
        window_float_event_data.rs      # HyprlandWindowFloatEventData
        layout_event.rs                 # HyprlandLayoutEvent
        window_title_event_data.rs      # HyprlandWindowTitleEventData
        non_special_workspace.rs        # HyprlandNonSpecialWorkspaceEventData
        changed_special.rs              # HyprlandChangedSpecialEventData
        screencast.rs                   # HyprlandScreencastEventData
        window_pin.rs                   # HyprlandWindowPinEventData
        group_toggled.rs                # HyprlandGroupToggledEventData
```

**Note**: Workspace and monitor events are **not** in this crate — they use `smearor-model-compositor` types. The following events from the original concept are
dropped because they are already covered by compositor-unified types:

- ~~`WorkspaceChangedStatusMessage`~~ → use `WorkspaceChangedEvent` from `smearor-model-compositor`
- ~~`WorkspaceAddedStatusMessage`~~ → use `WorkspaceLifecycleEvent` with `WorkspaceLifecycleType::Created`
- ~~`WorkspaceDestroyedStatusMessage`~~ → use `WorkspaceLifecycleEvent` with `WorkspaceLifecycleType::Destroyed`
- ~~`MonitorAddedStatusMessage`~~ → use `MonitorChangedEvent` with `MonitorChangeType::Connected`
- ~~`MonitorRemovedStatusMessage`~~ → use `MonitorChangedEvent` with `MonitorChangeType::Disconnected`
- ~~`WorkspaceMovedStatusMessage`~~ → use `WorkspaceChangedEvent` (workspace moved = workspace changed on a monitor)
- ~~`ActiveMonitorChangedStatusMessage`~~ → use `WorkspaceChangedEvent` (active monitor change implies workspace change)

### 2.3 Hyprland Service (`services/hyprland` — partially implemented, to be extended)

The service keeps its existing module structure for **atomic event processing**. Each domain (workspace, monitor, status) has its own worker module with
focused, small functions. Only the **listener** is consolidated — it is a thin dispatcher that converts raw `hyprland` events into typed messages and sends them
through a single channel. The worker side remains decomposed per domain.

```
services/hyprland/
  src/
    service.rs                    # HyprlandService (Phase 1 + consolidated listener spawn)
    config.rs                     # HyprlandServiceConfig
    workspace/
      mod.rs                      # workspace worker (compositor-unified) — atomic processing
    monitor/
      mod.rs                      # monitor worker (compositor-unified) — atomic processing
      event.rs
      worker.rs
    status/                       # NEW: Hyprland-specific status processing
      mod.rs                      # module declarations
      rate_limiter.rs             # per-variant rate limiting utility (small, focused)
      worker.rs                   # status worker: receives HyprlandStatusEvent, applies rate limiting, broadcasts
    event_listener/               # NEW: single consolidated socket listener (thin dispatcher)
      mod.rs                      # module declarations + HyprlandEvent dispatch enum
      listener.rs                 # single EventListener, registers ALL handlers, sends typed events to channel
```

**Design principle**: The listener is the only consolidated component. It does no processing — it only converts raw `hyprland` events to typed messages and
sends them through a channel. All processing logic (rate limiting, workspace lifecycle detection, monitor index resolution, broadcasting) stays in the existing
per-domain worker modules. This avoids monster-classes, monster-enums, and monster-functions.

## 3. Shared Status Types (`messages/shared/`)

These mirror the supporting types from `hyprland::event_listener` that have no compositor-unified equivalent. All carry `#[stabby::stabby]`,
`#[derive(Clone, Debug, Default, Serialize, Deserialize)]`.

| Rust Type                         | Source                                                   |
|-----------------------------------|----------------------------------------------------------|
| `HyprlandWindowEventData`         | `hyprland::event_listener::WindowEventData`              |
| `HyprlandWindowOpenEvent`         | `hyprland::event_listener::WindowOpenEvent`              |
| `HyprlandWindowMoveEvent`         | `hyprland::event_listener::WindowMoveEvent`              |
| `HyprlandWindowFloatEventData`    | `hyprland::event_listener::WindowFloatEventData`         |
| `HyprlandLayoutEvent`             | `hyprland::event_listener::LayoutEvent`                  |
| `HyprlandWindowTitleEventData`    | `hyprland::event_listener::WindowTitleEventData`         |
| `HyprlandNonSpecialWorkspaceData` | `hyprland::event_listener::NonSpecialWorkspaceEventData` |
| `HyprlandChangedSpecialEventData` | `hyprland::event_listener::ChangedSpecialEventData`      |
| `HyprlandScreencastEventData`     | `hyprland::event_listener::ScreencastEventData`          |
| `HyprlandWindowPinEventData`      | `hyprland::event_listener::WindowPinEventData`           |
| `HyprlandGroupToggledEventData`   | `hyprland::event_listener::GroupToggledEventData`        |

## 4. Hyprland-Specific Status Message Types (`messages/status/`)

One message struct per `AsyncEventListener` handler event. All status messages are broadcast on the topic `service.hyprland.status`. Only events that have **no
compositor-unified equivalent** are listed here.

| Message Type                               | Event Source                                  | Payload Fields                           |
|--------------------------------------------|-----------------------------------------------|------------------------------------------|
| `ActiveWindowChangedStatusMessage`         | `add_active_window_change_handler`            | `data: Option<HyprlandWindowEventData>`  |
| `FullscreenStateChangedStatusMessage`      | `add_fullscreen_state_change_handler`         | `is_fullscreen: bool`                    |
| `WindowOpenedStatusMessage`                | `add_window_open_handler`                     | `data: HyprlandWindowOpenEvent`          |
| `WindowClosedStatusMessage`                | `add_window_close_handler`                    | `window_address: stabby::string::String` |
| `WindowMovedStatusMessage`                 | `add_window_moved_handler`                    | `data: HyprlandWindowMoveEvent`          |
| `KeyboardLayoutChangedStatusMessage`       | `add_keyboard_layout_change_handler`          | `data: HyprlandLayoutEvent`              |
| `SubMapChangedStatusMessage`               | `add_sub_map_change_handler`                  | `sub_map: stabby::string::String`        |
| `LayerOpenedStatusMessage`                 | `add_layer_open_handler`                      | `layer_name: stabby::string::String`     |
| `LayerClosedStatusMessage`                 | `add_layer_closed_handler`                    | `layer_name: stabby::string::String`     |
| `FloatStateChangedStatusMessage`           | `add_float_state_handler`                     | `data: HyprlandWindowFloatEventData`     |
| `UrgentStateChangedStatusMessage`          | `add_urgent_state_handler`                    | `window_address: stabby::string::String` |
| `WindowTitleChangedStatusMessage`          | `add_window_title_change_handler`             | `data: HyprlandWindowTitleEventData`     |
| `WorkspaceRenamedStatusMessage`            | `add_workspace_renamed_handler`               | `data: HyprlandNonSpecialWorkspaceData`  |
| `SpecialRemovedStatusMessage`              | `add_special_removed_handler`                 | `monitor_name: stabby::string::String`   |
| `ChangedSpecialStatusMessage`              | `add_changed_special_handler`                 | `data: HyprlandChangedSpecialEventData`  |
| `ScreencastStatusMessage`                  | `add_screencast_handler`                      | `data: HyprlandScreencastEventData`      |
| `ConfigReloadedStatusMessage`              | `add_config_reloaded_handler`                 | (no payload)                             |
| `IgnoreGroupLockStateChangedStatusMessage` | `add_ignore_group_lock_state_changed_handler` | `is_enabled: bool`                       |
| `LockGroupsStateChangedStatusMessage`      | `add_lock_groups_state_changed_handler`       | `is_locked: bool`                        |
| `WindowPinnedStatusMessage`                | `add_window_pinned_handler`                   | `data: HyprlandWindowPinEventData`       |
| `GroupToggledStatusMessage`                | `add_group_toggled_handler`                   | `data: HyprlandGroupToggledEventData`    |
| `WindowMovedIntoGroupStatusMessage`        | `add_window_moved_into_group_handler`         | `window_address: stabby::string::String` |
| `WindowMovedOutOfGroupStatusMessage`       | `add_window_moved_out_of_group_handler`       | `window_address: stabby::string::String` |

All message structs derive `Clone, Debug, Default, Serialize, Deserialize` and carry `#[stabby::stabby]`. The `Default` derive is required for
`serde_json::from_value(json).unwrap_or_default()` fallback deserialization (see Section 5.4).

## 5. Unified Status Event Enum and State Request

### 5.1 HyprlandStatusEvent

A single enum wraps every **Hyprland-specific** status message so the service can broadcast one type through the plugin message system. Workspace and monitor
events are excluded — they use compositor-unified types directly.

```rust
/// Unified enum for all Hyprland-specific status events.
#[stabby::stabby]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum HyprlandStatusEvent {
    /// No event (default for `unwrap_or_default()` fallback).
    #[default]
    None,
    ActiveWindowChanged(ActiveWindowChangedStatusMessage),
    FullscreenStateChanged(FullscreenStateChangedStatusMessage),
    WindowOpened(WindowOpenedStatusMessage),
    WindowClosed(WindowClosedStatusMessage),
    WindowMoved(WindowMovedStatusMessage),
    KeyboardLayoutChanged(KeyboardLayoutChangedStatusMessage),
    SubMapChanged(SubMapChangedStatusMessage),
    LayerOpened(LayerOpenedStatusMessage),
    LayerClosed(LayerClosedStatusMessage),
    FloatStateChanged(FloatStateChangedStatusMessage),
    UrgentStateChanged(UrgentStateChangedStatusMessage),
    WindowTitleChanged(WindowTitleChangedStatusMessage),
    WorkspaceRenamed(WorkspaceRenamedStatusMessage),
    SpecialRemoved(SpecialRemovedStatusMessage),
    ChangedSpecial(ChangedSpecialStatusMessage),
    Screencast(ScreencastStatusMessage),
    ConfigReloaded(ConfigReloadedStatusMessage),
    IgnoreGroupLockStateChanged(IgnoreGroupLockStateChangedStatusMessage),
    LockGroupsStateChanged(LockGroupsStateChangedStatusMessage),
    WindowPinned(WindowPinnedStatusMessage),
    GroupToggled(GroupToggledStatusMessage),
    WindowMovedIntoGroup(WindowMovedIntoGroupStatusMessage),
    WindowMovedOutOfGroup(WindowMovedOutOfGroupStatusMessage),
}

/// The main status envelope broadcast by the service on `service.hyprland.status`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HyprlandStatusMessage {
    pub event: HyprlandStatusEvent,
}

impl TypedMessage for HyprlandStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandStatusMessage");
}

impl MessageTopic for HyprlandStatusMessage {
    fn topic() -> &'static str { "service.hyprland.status" }
}
```

### 5.2 HyprlandStateRequestMessage and HyprlandStateMessage

Widgets need the current Hyprland-specific state on startup, before any event fires. The `HyprlandStateRequestMessage` lets a widget query the current state.
The service responds with a `HyprlandStateMessage` broadcast on `service.hyprland.status.response`.

Both topics follow the **service topic naming convention** (see Section 5.3):

```rust
/// Topic for Hyprland state requests (Widget -> Service).
pub const TOPIC_HYPRLAND_STATE_REQUEST: &str = "service.hyprland.status.request";

/// Topic for Hyprland state responses (Service -> Widget).
pub const TOPIC_HYPRLAND_STATE: &str = "service.hyprland.status.response";

/// Request the current Hyprland-specific state from the service.
#[stabby::stabby]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HyprlandStateRequestMessage {}

impl TypedMessage for HyprlandStateRequestMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandStateRequestMessage");
}

impl MessageTopic for HyprlandStateRequestMessage {
    fn topic() -> &'static str { TOPIC_HYPRLAND_STATE_REQUEST }
}

/// Current Hyprland-specific state, broadcast by the service in response to a state request.
/// Contains all Hyprland-specific state that a widget needs on startup or reload.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HyprlandStateMessage {
    /// The currently active window, if any.
    pub active_window: Option<HyprlandWindowEventData>,
    /// Whether the active window is fullscreen.
    pub is_fullscreen: bool,
    /// Currently active keyboard layout name, if queryable.
    pub keyboard_layout: Option<stabby::string::String>,
    /// Currently active submap name, if any (empty string = default submap).
    pub sub_map: stabby::string::String,
    /// Whether ignore-group-lock is currently enabled.
    pub ignore_group_lock: bool,
    /// Whether groups are currently locked.
    pub groups_locked: bool,
}

impl TypedMessage for HyprlandStateMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandStateMessage");
}

impl MessageTopic for HyprlandStateMessage {
    fn topic() -> &'static str { TOPIC_HYPRLAND_STATE }
}
```

### 5.3 Service Topic Naming Convention

To avoid modifying the launcher core for every new service or topic, all service outbound messages follow a unified naming convention:

| Pattern                          | Direction         | Purpose                           |
|----------------------------------|-------------------|-----------------------------------|
| `service.<name>.status`          | Service → Widgets | Event stream (continuous updates) |
| `service.<name>.status.request`  | Widget → Service  | Request current state             |
| `service.<name>.status.response` | Service → Widgets | State response to a request       |
| `service.<name>.status.<sub>`    | Service → Widgets | Sub-category event stream         |
| `service.<name>.command`         | Widget → Service  | Command (already established)     |
| `service.<name>.command.<sub>`   | Widget → Service  | Sub-category command              |

The launcher core routes all `service.*` topics containing `.status` or `.command` to all plugins. Plugins filter by `type_id` via `MessageHandler<T>`.

**Launcher core change** (one-time):

```rust
// Replace the fragmented routing checks with:
if topic.starts_with("service.") & & (topic.contains(".status") | | topic.contains(".command")) {
for r in self.plugin_manager.plugins.iter() {
let plugin = r.value();
unsafe {
plugin.on_message(envelope.clone());
}
}
}
```

This replaces the current hardcoded suffix list (`.status`, `.scan_results`, `.vpn_profiles`, `.response.`, `TOPIC_MACROPAD_INPUT`, etc.).

#### Migration: Existing Non-Conforming Topics

The following 7 topics do not conform to the convention and must be migrated. The migration is **backward-compatible**: the launcher core keeps the old routing
checks during the transition period, and both old and new topics work simultaneously. Once all services and widgets are updated, the old routing checks are
removed.

| Current Topic                  | Migrated Topic                        | Model Crate      | Affected Service    |
|--------------------------------|---------------------------------------|------------------|---------------------|
| `service.http.request`         | `service.http.status.request`         | `model/http`     | `services/http`     |
| `service.hyprland.ctl`         | `service.hyprland.command.ctl`        | `model/hyprland` | `services/hyprland` |
| `service.hyprland.dispatch`    | `service.hyprland.command.dispatch`   | `model/hyprland` | `services/hyprland` |
| `service.macropad.connection`  | `service.macropad.status.connection`  | `model/macropad` | `services/macropad` |
| `service.macropad.input`       | `service.macropad.status.input`       | `model/macropad` | `services/macropad` |
| `service.network.scan_results` | `service.network.status.scan_results` | `model/network`  | `services/network`  |
| `service.network.vpn_profiles` | `service.network.status.vpn_profiles` | `model/network`  | `services/network`  |

**Already conforming topics** (no migration needed):

| Topic                              | Pattern                       |
|------------------------------------|-------------------------------|
| `service.audio.status`             | `service.<name>.status`       |
| `service.audio.command`            | `service.<name>.command`      |
| `service.mpris.status`             | `service.<name>.status`       |
| `service.mpris.command`            | `service.<name>.command`      |
| `service.network.status`           | `service.<name>.status`       |
| `service.network.command`          | `service.<name>.command`      |
| `service.power.status`             | `service.<name>.status`       |
| `service.power.command`            | `service.<name>.command`      |
| `service.notifications.status`     | `service.<name>.status`       |
| `service.notifications.command`    | `service.<name>.command`      |
| `service.personalization.status`   | `service.<name>.status`       |
| `service.personalization.command`  | `service.<name>.command`      |
| `service.sysinfo.cpu.status`       | `service.<name>.status.<sub>` |
| `service.sysinfo.memory.status`    | `service.<name>.status.<sub>` |
| `service.sysinfo.battery.status`   | `service.<name>.status.<sub>` |
| `service.sysinfo.disks.status`     | `service.<name>.status.<sub>` |
| `service.sysinfo.network.status`   | `service.<name>.status.<sub>` |
| `service.sysinfo.uptime.status`    | `service.<name>.status.<sub>` |
| `service.wallpaper.status`         | `service.<name>.status`       |
| `service.wallpaper.command`        | `service.<name>.command`      |
| `service.weather.status`           | `service.<name>.status`       |
| `service.weather.command`          | `service.<name>.command`      |
| `service.voice_assistant.status`   | `service.<name>.status`       |
| `service.voice_assistant.command`  | `service.<name>.command`      |
| `service.terminal_command.status`  | `service.<name>.status`       |
| `service.terminal_command.command` | `service.<name>.command`      |
| `service.app_launcher.status`      | `service.<name>.status`       |
| `service.app_launcher.command`     | `service.<name>.command`      |

#### Migration Steps

1. **Launcher core**: Add `topic.contains(".status") || topic.contains(".command")` routing alongside existing checks (backward-compatible).
2. **Model crates**: Update `TOPIC_*` constants for the 7 non-conforming topics.
3. **Services**: No changes needed — services use the constants from model crates.
4. **Widgets**: No changes needed — widgets use `MessageHandler<T>` with `MessageTopic::topic()`, which references the same constants.
5. **Launcher core cleanup**: Remove old hardcoded suffix checks (`.scan_results`, `.vpn_profiles`, `.response.`, `TOPIC_MACROPAD_*`).

**Tracking**: To ensure no service is forgotten, the migration can be tracked via a grep for non-conforming topic patterns:

```bash
grep -rn 'pub const TOPIC_.*=.*"service\.' model/ | grep -v '\.status\|\.command'
```

This should return zero results after migration is complete.

### 5.4 JSON Serialization (impl_json_convertible!)

All message types, shared types, and the `HyprlandStatusEvent` enum derive `Serialize, Deserialize` from `serde`. The `stabby` dependency in
`model/hyprland` must include the `serde` feature (`stabby = { workspace = true, features = ["serde"] }`).

No manual `parse_*` or `serialize_*` functions. JSON conversion is handled exclusively via `impl_json_convertible!` macros in `lib.rs`, following the same
pattern as all other model crates (`model/audio`, `model/workspace`, `model/notifications`, etc.).

**`HyprlandStatusEvent`** needs a `#[default]` variant (`None`) for `unwrap_or_default()` fallback, consistent with all other enums in the codebase (e.g.
`NotificationCommandAction`, `AudioCommandAction`, `DesktopFileStatus`).

**`HyprlandStatusMessage`** (the envelope struct) also derives `Default` — `unwrap_or_default()` produces an envelope with `HyprlandStatusEvent::None`.

**`HyprlandStateRequestMessage`** and **`HyprlandStateMessage`** both derive `Default` for the same reason.

```rust
// model/hyprland/src/lib.rs (extended)

use smearor_swipe_launcher_plugin_api::impl_json_convertible;

// Only 3 top-level messages need JSON converters.
// The 24 individual status message structs are NOT top-level messages — they are
// nested inside HyprlandStatusEvent inside HyprlandStatusMessage, and are
// serialized/deserialized automatically via serde's derived Serialize/Deserialize.
// No individual impl_json_convertible! calls needed for them.

// --- Status envelope converter ---
impl_json_convertible!(HyprlandStatusMessageConverter, HyprlandStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

// --- State request/response converters ---
impl_json_convertible!(HyprlandStateRequestConverter, HyprlandStateRequestMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandStateConverter, HyprlandStateMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
```

The existing `register_json_converters(context)` function in `lib.rs` is extended to register the 3 new converters:

```rust
// model/hyprland/src/lib.rs (extended)

pub fn register_json_converters(context: Option<FfiCoreContext>) {
    // --- Phase 1 (already implemented) ---
    HyprlandDispatchMessageConverter::register_in_host(context);
    KillCommandMessageConverter::register_in_host(context);
    // ... existing Phase 1 converters ...
    SwitchXkbLayoutCommandMessageConverter::register_in_host(context);
    // --- Phase 2 (new) ---
    HyprlandStatusMessageConverter::register_in_host(context);
    HyprlandStateRequestConverter::register_in_host(context);
    HyprlandStateConverter::register_in_host(context);
}
```

The service already calls `register_json_converters(core_context)` during initialization (Phase 1). No additional call needed — the extended function registers
all Phase 2 converters automatically.

## 6. Service Extension

### 6.1 Current Service Struct (already implemented)

The `HyprlandService` already has workspace and monitor listeners that broadcast compositor-unified events:

```rust
pub struct HyprlandService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub command_sender: mpsc::UnboundedSender<HyprlandCommand>,
    pub config: Arc<HyprlandServiceConfig>,
}
```

During construction, the service currently spawns:

- `spawn_workspace_listener` + `spawn_workspace_worker` — broadcasts `WorkspaceChangedEvent` / `WorkspaceLifecycleEvent` on `compositor::*` topics
- `spawn_monitor_listener` + `spawn_monitor_worker` — broadcasts `MonitorChangedEvent` on `compositor::monitor_changed`

### 6.2 Refactor: Single Consolidated Event Listener (Thin Dispatcher)

The current architecture uses two separate `EventListener` instances (workspace + monitor), each opening its own socket to Hyprland. Phase 2 consolidates all
event listening into a **single listener** on a single socket. The listener is a **thin dispatcher**: it only registers handlers, converts raw
`hyprland` events to typed messages, and sends them through a single channel. It contains **no processing logic**.

The `HyprlandEvent` dispatch enum has only 4 variants — one per domain — not one per event type:

```rust
// services/hyprland/src/event_listener/mod.rs

/// Dispatch enum for routing events from the single listener to domain workers.
/// Small by design: one variant per domain, not per event type.
pub enum HyprlandEvent {
    Workspace(WorkspaceEvent),
    Monitor(MonitorEvent),
    Status(HyprlandStatusEvent),
}
```

The listener registers handlers conditionally based on config flags. Each handler is a one-liner that converts and sends:

```rust
// services/hyprland/src/event_listener/listener.rs

pub fn spawn_event_listener(
    event_sender: mpsc::UnboundedSender<HyprlandEvent>,
    enable_workspace_tracking: bool,
    enable_monitor_events: bool,
    enable_status_events: bool,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            ensure_hyprland_instance_signature();
            let mut reconnect_attempts: u32 = 0;
            loop {
                let mut listener = hyprland::event_listener::EventListener::new();

                if enable_workspace_tracking {
                    crate::workspace::register_handlers(&mut listener, event_sender.clone());
                }
                if enable_monitor_events {
                    crate::monitor::register_handlers(&mut listener, event_sender.clone());
                }
                if enable_status_events {
                    crate::status::register_handlers(&mut listener, event_sender.clone());
                }

                match listener.start_listener_async().await {
                    Ok(()) => {
                        reconnect_attempts = 0;
                    }
                    Err(error) => {
                        reconnect_attempts += 1;
                        if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                            // Switch to slow backoff: retry every 30s indefinitely.
                            // This handles Hyprland crashes/restarts that take longer
                            // than the fast-retry window (10 × 5s = 50s).
                            debug!(
                                "Hyprland listener: {} fast retries exhausted, switching to 30s backoff",
                                reconnect_attempts
                            );
                            tokio::time::sleep(Duration::from_secs(30)).await;
                            continue;
                        }
                    }
                }
                // Fast retry: 5s between attempts.
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    });
}
```

Each domain module exposes a `register_handlers` function that registers its own handlers on the shared listener. This keeps the listener file thin and the
domain logic in its own module:

```rust
// services/hyprland/src/status/mod.rs

/// Register all Hyprland-specific status handlers on the shared listener.
pub fn register_handlers(
    listener: &mut hyprland::event_listener::EventListener,
    sender: mpsc::UnboundedSender<HyprlandEvent>,
) {
    let s = sender.clone();
    listener.add_active_window_change_handler(move |data| {
        let _ = s.send(HyprlandEvent::Status(
            HyprlandStatusEvent::ActiveWindowChanged(
                ActiveWindowChangedStatusMessage {
                    data: data.map(convert_window_event_data),
                },
            ),
        ));
    });

    let s = sender.clone();
    listener.add_fullscreen_state_change_handler(move |is_fullscreen| {
        let _ = s.send(HyprlandEvent::Status(
            HyprlandStatusEvent::FullscreenStateChanged(
                FullscreenStateChangedStatusMessage { is_fullscreen },
            ),
        ));
    });

    // ... remaining handlers, each a one-liner convert + send
}
```

```rust
// services/hyprland/src/workspace/mod.rs (refactored)

/// Register workspace handlers on the shared listener.
pub fn register_handlers(
    listener: &mut hyprland::event_listener::EventListener,
    sender: mpsc::UnboundedSender<HyprlandEvent>,
) {
    let s = sender.clone();
    listener.add_workspace_changed_handler(move |workspace_data| {
        let event = convert_workspace_changed(workspace_data);
        let _ = s.send(HyprlandEvent::Workspace(WorkspaceEvent::Changed(event)));
    });
}
```

```rust
// services/hyprland/src/monitor/mod.rs (refactored)

/// Register monitor handlers on the shared listener.
pub fn register_handlers(
    listener: &mut hyprland::event_listener::EventListener,
    sender: mpsc::UnboundedSender<HyprlandEvent>,
) {
    let s = sender.clone();
    listener.add_monitor_added_handler(move |data| {
        let _ = s.send(HyprlandEvent::Monitor(MonitorEvent::Added(data.name)));
    });

    let s = sender.clone();
    listener.add_monitor_removed_handler(move |data| {
        let _ = s.send(HyprlandEvent::Monitor(MonitorEvent::Removed(data)));
    });
}
```

### 6.3 Thin Dispatch Worker + Atomic Domain Workers

The worker side stays **decomposed per domain**. A thin dispatch loop routes events to the appropriate domain worker. Each domain worker is a small, focused
function in its own module — no monster-function, no giant match block.

```rust
// services/hyprland/src/event_listener/worker.rs

/// Thin dispatch loop: routes events to domain workers. No processing logic here.
/// Uses `tokio::select!` to simultaneously wait for incoming events and a periodic
/// flush interval for the rate limiter's trailing-edge debounce.
pub fn spawn_event_worker(
    mut event_receiver: mpsc::UnboundedReceiver<HyprlandEvent>,
    core_context: Option<FfiCoreContext>,
    meta: PluginMeta,
    enable_workspace_lifecycle: bool,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            // Domain-specific state lives in the domain module, not here.
            let mut workspace_state = crate::workspace::WorkspaceState::new();
            let mut status_rate_limiter = crate::status::RateLimiter::new();

            // Periodic flush interval for trailing-edge debounce.
            // Runs every RATE_LIMIT_MS (50ms) to ensure trailing events
            // are flushed even when no new events arrive.
            let mut flush_interval = tokio::time::interval(
                Duration::from_millis(crate::status::RATE_LIMIT_MS)
            );
            flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    maybe_event = event_receiver.recv() => {
                        let Some(event) = maybe_event else { break; };
                        match event {
                            HyprlandEvent::Workspace(e) => {
                                crate::workspace::process_event(
                                    &mut workspace_state, e, &core_context, &meta, enable_workspace_lifecycle,
                                ).await;
                            }
                            HyprlandEvent::Monitor(e) => {
                                crate::monitor::process_event(e, &core_context, &meta).await;
                            }
                            HyprlandEvent::Status(e) => {
                                crate::status::process_event(
                                    &mut status_rate_limiter, e, &core_context, &meta,
                                ).await;
                            }
                        }
                    }
                    _ = flush_interval.tick() => {
                        // Flush any trailing events whose debounce window has expired.
                        // This ensures the last event in a burst is always delivered,
                        // even if no further events arrive.
                        while let Some(pending) = status_rate_limiter.flush_trailing() {
                            let message = HyprlandStatusMessage { event: pending };
                            crate::service::broadcast_event(&core_context, &meta, message);
                        }
                    }
                }
            }
        });
    });
}
```

Each domain module contains its own atomic processing function and state:

```rust
// services/hyprland/src/status/worker.rs

/// Process a single Hyprland-specific status event.
/// Applies rate limiting for high-frequency variants, then broadcasts.
/// If an event is dropped by the rate limiter, it is stored as the pending trailing
/// event. The trailing event is flushed by the periodic flush interval in the
/// worker's `tokio::select!` loop (see `spawn_event_worker`), not here.
pub async fn process_event(
    rate_limiter: &mut RateLimiter,
    event: HyprlandStatusEvent,
    core_context: &Option<FfiCoreContext>,
    meta: &PluginMeta,
) {
    if let Some(pending) = rate_limiter.try_event(event) {
        let message = HyprlandStatusMessage { event: pending };
        broadcast_event(core_context, meta, message);
    }
    // Note: flush_trailing() is NOT called here. It is called periodically
    // by the worker's tokio::select! loop to ensure trailing events are
    // delivered even when no further events arrive.
}
```

```rust
// services/hyprland/src/status/rate_limiter.rs

/// Per-variant rate limiter with trailing-edge debounce for high-frequency status events.
/// Small, focused utility — no domain logic here.
///
/// High-frequency variants use a throttle with trailing edge: if an event arrives
/// within the debounce window, it is stored as the trailing event. After the window
/// expires, the trailing event is flushed automatically. This prevents stale UI state
/// that would occur with a pure drop/throttle approach.
pub struct RateLimiter {
    last_broadcast: HashMap<StatusVariant, Instant>,
    /// Pending trailing event per variant, to be flushed after the debounce window.
    trailing: HashMap<StatusVariant, HyprlandStatusEvent>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_broadcast: HashMap::new(),
            trailing: HashMap::new(),
        }
    }

    /// Try to broadcast an event immediately. If rate-limited, store it as trailing.
    /// Returns `Some(event)` if it should be broadcast now, `None` if it was stored as trailing.
    pub fn try_event(&mut self, event: HyprlandStatusEvent) -> Option<HyprlandStatusEvent> {
        let variant = StatusVariant::from(&event);
        if !variant.is_high_frequency() {
            return Some(event);
        }
        let now = Instant::now();
        if let Some(last) = self.last_broadcast.get(&variant) {
            if now.duration_since(*last) < Duration::from_millis(RATE_LIMIT_MS) {
                // Within debounce window — store as trailing event (replaces any previous trailing).
                self.trailing.insert(variant, event);
                return None;
            }
        }
        self.last_broadcast.insert(variant, now);
        self.trailing.remove(&variant);
        Some(event)
    }

    /// Check if any trailing events are ready to be flushed after their debounce window expired.
    /// Returns `Some(event)` if a trailing event should be broadcast, `None` otherwise.
    pub fn flush_trailing(&mut self) -> Option<HyprlandStatusEvent> {
        let now = Instant::now();
        for (variant, event) in self.trailing.iter() {
            if let Some(last) = self.last_broadcast.get(variant) {
                if now.duration_since(*last) >= Duration::from_millis(RATE_LIMIT_MS) {
                    // Found a trailing event whose debounce window has expired.
                    let variant = *variant;
                    let event = self.trailing.remove(&variant)?;
                    self.last_broadcast.insert(variant, now);
                    return Some(event);
                }
            }
        }
        None
    }
}

const RATE_LIMIT_MS: u64 = 50;

/// Lightweight classification of status event variants for rate limiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum StatusVariant {
    None,
    ActiveWindowChanged,
    FullscreenStateChanged,
    WindowOpened,
    WindowClosed,
    WindowMoved,
    KeyboardLayoutChanged,
    SubMapChanged,
    LayerOpened,
    LayerClosed,
    FloatStateChanged,
    UrgentStateChanged,
    WindowTitleChanged,
    WorkspaceRenamed,
    SpecialRemoved,
    ChangedSpecial,
    Screencast,
    ConfigReloaded,
    IgnoreGroupLockStateChanged,
    LockGroupsStateChanged,
    WindowPinned,
    GroupToggled,
    WindowMovedIntoGroup,
    WindowMovedOutOfGroup,
}

impl From<&HyprlandStatusEvent> for StatusVariant {
    fn from(event: &HyprlandStatusEvent) -> Self {
        match event {
            HyprlandStatusEvent::None => StatusVariant::None,
            HyprlandStatusEvent::ActiveWindowChanged(_) => StatusVariant::ActiveWindowChanged,
            HyprlandStatusEvent::FullscreenStateChanged(_) => StatusVariant::FullscreenStateChanged,
            HyprlandStatusEvent::WindowOpened(_) => StatusVariant::WindowOpened,
            HyprlandStatusEvent::WindowClosed(_) => StatusVariant::WindowClosed,
            HyprlandStatusEvent::WindowMoved(_) => StatusVariant::WindowMoved,
            HyprlandStatusEvent::KeyboardLayoutChanged(_) => StatusVariant::KeyboardLayoutChanged,
            HyprlandStatusEvent::SubMapChanged(_) => StatusVariant::SubMapChanged,
            HyprlandStatusEvent::LayerOpened(_) => StatusVariant::LayerOpened,
            HyprlandStatusEvent::LayerClosed(_) => StatusVariant::LayerClosed,
            HyprlandStatusEvent::FloatStateChanged(_) => StatusVariant::FloatStateChanged,
            HyprlandStatusEvent::UrgentStateChanged(_) => StatusVariant::UrgentStateChanged,
            HyprlandStatusEvent::WindowTitleChanged(_) => StatusVariant::WindowTitleChanged,
            HyprlandStatusEvent::WorkspaceRenamed(_) => StatusVariant::WorkspaceRenamed,
            HyprlandStatusEvent::SpecialRemoved(_) => StatusVariant::SpecialRemoved,
            HyprlandStatusEvent::ChangedSpecial(_) => StatusVariant::ChangedSpecial,
            HyprlandStatusEvent::Screencast(_) => StatusVariant::Screencast,
            HyprlandStatusEvent::ConfigReloaded(_) => StatusVariant::ConfigReloaded,
            HyprlandStatusEvent::IgnoreGroupLockStateChanged(_) => StatusVariant::IgnoreGroupLockStateChanged,
            HyprlandStatusEvent::LockGroupsStateChanged(_) => StatusVariant::LockGroupsStateChanged,
            HyprlandStatusEvent::WindowPinned(_) => StatusVariant::WindowPinned,
            HyprlandStatusEvent::GroupToggled(_) => StatusVariant::GroupToggled,
            HyprlandStatusEvent::WindowMovedIntoGroup(_) => StatusVariant::WindowMovedIntoGroup,
            HyprlandStatusEvent::WindowMovedOutOfGroup(_) => StatusVariant::WindowMovedOutOfGroup,
        }
    }
}

impl StatusVariant {
    fn is_high_frequency(&self) -> bool {
        matches!(
            self,
            StatusVariant::ActiveWindowChanged | StatusVariant::WindowTitleChanged
        )
    }
}
```

```rust
// services/hyprland/src/workspace/mod.rs (refactored, atomic processing)

/// Workspace-specific state for the worker loop.
pub struct WorkspaceState {
    known_workspaces: HashSet<i32>,
}

impl WorkspaceState {
    pub fn new() -> Self { Self { known_workspaces: HashSet::new() } }
}

/// Process a single workspace event. Atomic, focused, no status logic mixed in.
pub async fn process_event(
    state: &mut WorkspaceState,
    event: WorkspaceEvent,
    core_context: &Option<FfiCoreContext>,
    meta: &PluginMeta,
    enable_workspace_lifecycle: bool,
) {
    match event {
        WorkspaceEvent::Changed(mut event) => {
            // ... existing workspace lifecycle detection + broadcast logic
        }
    }
}
```

```rust
// services/hyprland/src/monitor/mod.rs (refactored, atomic processing)

/// Process a single monitor event. Atomic, focused.
pub async fn process_event(
    event: MonitorEvent,
    core_context: &Option<FfiCoreContext>,
    meta: &PluginMeta,
) {
    match event {
        MonitorEvent::Added(name) => { /* ... */ }
        MonitorEvent::Removed(name) => { /* ... */ }
    }
}
```

### 6.4 Config Extension

Add `enable_status_events: bool` to `HyprlandServiceConfig`:

```rust
pub struct HyprlandServiceConfig {
    pub socket_path: Option<String>,
    #[serde(default)]
    pub enable_workspace_tracking: bool,
    #[serde(default = "default_enable_monitor_events")]
    pub enable_monitor_events: bool,
    #[serde(default = "default_enable_workspace_lifecycle")]
    pub enable_workspace_lifecycle: bool,
    /// Enable Hyprland-specific status events (active window, fullscreen, etc.).
    #[serde(default)]
    pub enable_status_events: bool,
}
```

### 6.5 Service Construction Update

The existing `spawn_workspace_listener` + `spawn_monitor_listener` calls are replaced by a single `spawn_event_listener` call. The existing
`spawn_workspace_worker` and `spawn_monitor_worker` are replaced by a single `spawn_event_worker` that dispatches to the per-domain `process_event`
functions:

```rust
// Replace the existing workspace + monitor listener spawns with:
let (event_sender, event_receiver) = mpsc::unbounded_channel::<HyprlandEvent>();
spawn_event_listener(
event_sender,
service.config.enable_workspace_tracking,
service.config.enable_monitor_events,
service.config.enable_status_events,
);
spawn_event_worker(
event_receiver,
service.core_context.clone(),
service.meta.clone(),
service.config.enable_workspace_lifecycle,
);
```

### 6.6 HyprlandCommand Enum Extension

The existing `HyprlandCommand` enum (Phase 1) is extended with a `StateRequest` variant. This follows the same pattern as all other services (`PulseCommand`,
`MprisCommand`, `WallpaperCommand`, etc.): `MessageHandler` sends a command via `command_sender`, the async worker loop receives and processes it.

```rust
// services/hyprland/src/service.rs (existing enum, extended)

/// Internal union of all command types the service handles.
pub enum HyprlandCommand {
    // --- Phase 1 (already implemented) ---
    Dispatch(HyprlandDispatchMessage),
    SwitchWorkspace(SwitchWorkspaceMessage),
    CreateWorkspace(CreateWorkspaceMessage),
    SnapshotRequest(WorkspaceSnapshotRequestMessage),
    CtlKill(KillCommandMessage),
    CtlNotify(NotifyCommandMessage),
    CtlOutputCreate(OutputCreateCommandMessage),
    CtlOutputRemove(OutputRemoveCommandMessage),
    CtlPluginLoad(PluginLoadCommandMessage),
    CtlPluginUnload(PluginUnloadCommandMessage),
    CtlReload(ReloadCommandMessage),
    CtlSetCursor(SetCursorCommandMessage),
    CtlSetError(SetErrorCommandMessage),
    CtlSetProp(SetPropCommandMessage),
    CtlSwitchXkbLayout(SwitchXkbLayoutCommandMessage),
    // --- Phase 2 (new) ---
    /// Query current Hyprland-specific state and broadcast `HyprlandStateMessage`.
    StateRequest,
}
```

### 6.7 Internal Dispatch Enums

The `HyprlandEvent` dispatch enum (Section 6.2) uses per-domain sub-enums. These are internal to the service crate and live in
`event_listener/mod.rs`:

```rust
// services/hyprland/src/event_listener/mod.rs

/// Workspace domain events (internal dispatch).
#[derive(Clone, Debug)]
pub enum WorkspaceEvent {
    Changed(WorkspaceChangedEvent),
}

/// Monitor domain events (internal dispatch).
#[derive(Clone, Debug)]
pub enum MonitorEvent {
    Added(String),
    Removed(String),
}
```

### 6.8 Helper Functions

**`broadcast_event`** — generic broadcast helper, already used in Phase 1 for compositor-unified events. Lives in `service.rs`:

```rust
// services/hyprland/src/service.rs (already implemented in Phase 1)

fn broadcast_event<T: MessageTopic + Serialize>(
    core_context: &Option<FfiCoreContext>,
    meta: &PluginMeta,
    event: T,
) {
    if let Some(context) = core_context {
        let envelope = FfiEnvelopePayload::new(event, meta.id.clone());
        context.broadcast_message(envelope);
    }
}
```

**`convert_*` functions** — each domain module has its own conversion functions that translate raw `hyprland` crate types into typed model types. These live in
the respective domain module and are `pub(crate)`:

```rust
// services/hyprland/src/workspace/mod.rs
pub(crate) fn convert_workspace_changed(data: WorkspaceEventData) -> WorkspaceChangedEvent { ... }

// services/hyprland/src/monitor/mod.rs
pub(crate) fn convert_monitor_added(data: MonitorAddedEventData) -> MonitorChangedEvent { ... }
pub(crate) fn convert_monitor_removed(name: String) -> MonitorChangedEvent { ... }

// services/hyprland/src/status/mod.rs
pub(crate) fn convert_window_event_data(data: WindowEventData) -> HyprlandWindowEventData { ... }
pub(crate) fn convert_window_open(data: WindowOpenEvent) -> HyprlandWindowOpenEvent { ... }
pub(crate) fn convert_window_move(data: WindowMoveEvent) -> HyprlandWindowMoveEvent { ... }
pub(crate) fn convert_float_state(data: WindowFloatEventData) -> HyprlandWindowFloatEventData { ... }
pub(crate) fn convert_layout(data: LayoutEvent) -> HyprlandLayoutEvent { ... }
pub(crate) fn convert_window_title(data: WindowTitleEventData) -> HyprlandWindowTitleEventData { ... }
pub(crate) fn convert_non_special_workspace(data: NonSpecialWorkspaceEventData) -> HyprlandNonSpecialWorkspaceData { ... }
pub(crate) fn convert_changed_special(data: ChangedSpecialEventData) -> HyprlandChangedSpecialEventData { ... }
pub(crate) fn convert_screencast(data: ScreencastEventData) -> HyprlandScreencastEventData { ... }
pub(crate) fn convert_window_pin(data: WindowPinEventData) -> HyprlandWindowPinEventData { ... }
pub(crate) fn convert_group_toggled(data: GroupToggledEventData) -> HyprlandGroupToggledEventData { ... }
```

### 6.9 Unknown Event Handling

The `hyprland` crate exposes an `Unknown` catch-all event (`add_unknown_handler`). The service logs it at `debug!` level and does not broadcast it:

```rust
// services/hyprland/src/status/mod.rs (inside register_handlers)

let s = sender.clone();
listener.add_unknown_handler( move | data| {
debug ! ("Hyprland: unknown event received: {:?}", data);
// Not forwarded — no HyprlandStatusEvent variant for unknown events.
});
```

### 6.10 HyprlandStateRequestMessage Handler

The service handles `HyprlandStateRequestMessage` by querying `hyprland::data` modules for the current state and broadcasting a `HyprlandStateMessage`. The
synchronous IPC calls to Hyprland (`Clients::get()`, `Devices::get()`, `Submap::get()`) are wrapped in `tokio::task::spawn_blocking` to prevent blocking the
async event worker thread when the compositor is under load:

```rust
async fn handle_state_request(core_context: Option<FfiCoreContext>, meta: &PluginMeta) {
    ensure_hyprland_instance_signature();

    // Synchronous Hyprland IPC calls run in a blocking thread to avoid
    // stalling the async event worker when the compositor is under load.
    let state = tokio::task::spawn_blocking(|| {
        let active_window = hyprland::data::Clients::get()
            .ok()
            .and_then(|clients| clients.iter().find(|c| c.focus_history_id == 0))
            .map(|c| HyprlandWindowEventData {
                window_class: c.class.clone().into(),
                window_title: c.title.clone().into(),
                window_address: c.address.to_string().into(),
                workspace_id: c.workspace.id,
            });

        let is_fullscreen = active_window.as_ref().is_some_and(|w| /* query fullscreen state */);

        let keyboard_layout = hyprland::data::Devices::get()
            .ok()
            .and_then(|devices| devices.keyboards.first().map(|k| k.active_keymap.clone().into()));

        let sub_map = hyprland::data::Submap::get()
            .ok()
            .map(|s| s.0.into())
            .unwrap_or_default();

        let ignore_group_lock = /* query via hyprland::data */ false;
        let groups_locked = /* query via hyprland::data */ false;

        HyprlandStateMessage {
            active_window,
            is_fullscreen,
            keyboard_layout,
            sub_map,
            ignore_group_lock,
            groups_locked,
        }
    })
        .await
        .unwrap_or_default();

    broadcast_event(&core_context, meta, state);
}

impl MessageHandler<FfiEnvelopePayload<HyprlandStateRequestMessage>> for HyprlandService {
    fn handle_message(&self, _message: FfiEnvelopePayload<HyprlandStateRequestMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::StateRequest);
    }
}
```

## 7. Widget Integration

The UI components that consume the events defined in this concept are described in [`HYPRLAND_WIDGET_CONCEPT.md`](./HYPRLAND_WIDGET_CONCEPT.md). This section
shows the message handler patterns widgets use to subscribe to the topics defined above.

### 7.1 Compositor-Unified Widgets (e.g. Workspace Switcher)

The Workspace Switcher widget subscribes to `compositor::*` topics. It works identically under Hyprland and GNOME. No Hyprland-specific knowledge needed.

```rust
// Handles WorkspaceChangedEvent from any compositor service
impl MessageHandler<FfiEnvelopePayload<WorkspaceChangedEvent>> for WorkspaceSwitcherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceChangedEvent>, _sender_id: &str) {
        let event = message.into_inner();
        self.update_workspace_display(event.workspace_id, event.monitor_index);
    }
}
```

### 7.2 Hyprland-Specific Widgets (e.g. Active Window Label)

A Hyprland-aware widget subscribes to `service.hyprland.status` and matches on `HyprlandStatusEvent` variants:

```rust
fn on_message(&self, message: FfiEnvelopePayload<HyprlandStatusMessage>) {
    match message.into_inner().event {
        HyprlandStatusEvent::ActiveWindowChanged(payload) => {
            if let Some(data) = payload.data {
                self.label.set_label(&format!("{} - {}", data.window_class, data.window_title));
            } else {
                self.label.set_label("No active window");
            }
        }
        _ => {}
    }
}
```

## 8. Dependencies

- `smearor-hyprland-model` — already depends on `stabby`, `serde`, `smearor-swipe-launcher-plugin-api`. No new dependency needed.
- `smearor-model-compositor` — already exists, no changes needed.
- `smearor-hyprland-service` — already depends on `hyprland`, `tokio`, `smearor-hyprland-model`, `smearor-model-compositor`. No new dependency needed.

### 8.1 Configuration Example

```toml
# configs/services/hyprland.toml

[hyprland]
enable_workspace_tracking = true
enable_workspace_lifecycle = true
enable_monitor_events = true
enable_status_events = true       # NEW: enables Hyprland-specific status events
```

## 9. Open Questions & Risks

### Resolved

1. ~~**Event listener lifecycle**~~ — **Resolved**: The pattern is already established in `workspace/mod.rs` and `monitor/listener.rs`: dedicated thread,
   `tokio::runtime::Builder::new_current_thread`, reconnect loop. The new consolidated listener will copy this pattern with a two-phase reconnection strategy
   (see point 2).
2. ~~**Socket connection / backoff**~~ — **Resolved**: Two-phase reconnection strategy: fast retries (5s delay, max 10 attempts = 50s) followed by slow backoff
   (30s delay, indefinite). This handles Hyprland crashes/restarts that take longer than 50s. See Section 6.2 for the implementation. The existing
   `workspace/mod.rs` and `monitor/listener.rs` use the old hard-break approach and will be refactored to match.
3. ~~**Launcher core routing**~~ — **Resolved**: `compositor.*` topics are broadcast to all plugins (`messages/mod.rs:183-190`). `service.*.status` topics are
   routed to all plugins (`messages/mod.rs:115-129`). Type-ID-based handling for `WorkspaceChangedEvent`, `MonitorChangedEvent`, and
   `WorkspaceLifecycleEvent` exists (`messages/mod.rs:193-247`).
4. ~~**Launcher core routing for state request/response**~~ — **Resolved via topic naming convention**: `HyprlandStateRequestMessage` uses
   `service.hyprland.status.request` and `HyprlandStateMessage` uses `service.hyprland.status.response`. Both match the `service.*.status*` routing pattern
   after the launcher core is updated to use `topic.contains(".status")` instead of `topic.ends_with(".status")`. See Section 5.3 for the naming convention and
   migration plan.

### Decided

1. **Rate limiting** — **Decided**: Rate limiting is applied in the unified worker **before broadcasting**. High-frequency event variants (`WindowTitleChanged`,
   `ActiveWindowChanged`) are rate-limited to a minimum interval of 50ms per variant using a **trailing-edge debounce**:
   if an event arrives within the debounce window, it is stored as the trailing event and automatically flushed after the window expires. This prevents stale UI
   state that would occur with a pure drop/throttle approach. Low-frequency events are broadcast immediately. See section 6.3 for implementation details.
2. **Single listener** — **Decided**: All event handlers (workspace, monitor, Hyprland-specific) are consolidated into a **single `EventListener`**
   instance on a single thread. This avoids multiple socket connections to Hyprland. The listener dispatches raw events to a unified worker via a single
   channel. The existing `spawn_workspace_listener` and `spawn_monitor_listener` are replaced by `spawn_event_listener`. See section 6.2.
3. **Initial state** — **Decided**: A `HyprlandStateRequestMessage` is added to `model/hyprland`. Widgets send this request on startup to query the current
   Hyprland-specific state (active window, fullscreen). The service responds with a `HyprlandStateMessage` broadcast on
   `service.hyprland.status.response`. See sections 2.2, 5.2, 5.3, and 6.10.
4. **Service topic naming convention** — **Decided**: All service outbound topics follow the pattern `service.<name>.status[.<sub>]`. This allows the launcher
   core to route all service outbound messages with a single `topic.starts_with("service.") && topic.contains(".status")` check, eliminating the need to modify
   the launcher core for each new service or topic. See Section 5.3 for the full convention and migration plan.

## 10. Relationship to Phase 1

This document is intentionally additive. The command service from `HYPRLAND_SERVICE_CONCEPT.md` remains unchanged. Phase 2 adds the event listener, status
messages, and broadcast loop to the same service crate. The service can be implemented and deployed incrementally: first commands, then status events.

## 11. Implementation Status

### 11.1 Already Implemented (Compositor-Unified Tier)

The following is **already working** in the codebase:

| Component                                     | File                                         | Status |
|-----------------------------------------------|----------------------------------------------|--------|
| `WorkspaceChangedEvent`                       | `model/workspace/src/workspace.rs`           | Done   |
| `WorkspaceLifecycleEvent`                     | `model/workspace/src/workspace.rs`           | Done   |
| `MonitorChangedEvent`                         | `model/workspace/src/monitor.rs`             | Done   |
| `SwitchWorkspaceMessage`                      | `model/workspace/src/switcher.rs`            | Done   |
| `CreateWorkspaceMessage`                      | `model/workspace/src/switcher.rs`            | Done   |
| `WorkspaceSnapshotMessage`                    | `model/workspace/src/switcher.rs`            | Done   |
| `WorkspaceSnapshotRequestMessage`             | `model/workspace/src/switcher.rs`            | Done   |
| `register_json_converters()`                  | `model/workspace/src/lib.rs`                 | Done   |
| Workspace listener (Hyprland)                 | `services/hyprland/src/workspace/mod.rs`     | Done   |
| Workspace worker (Hyprland)                   | `services/hyprland/src/workspace/mod.rs`     | Done   |
| Monitor listener (Hyprland)                   | `services/hyprland/src/monitor/listener.rs`  | Done   |
| Monitor worker (Hyprland)                     | `services/hyprland/src/monitor/worker.rs`    | Done   |
| `HyprlandServiceConfig`                       | `services/hyprland/src/config.rs`            | Done   |
| Service construction (workspace + monitor)    | `services/hyprland/src/service.rs`           | Done   |
| `SwitchWorkspaceMessage` handler              | `services/hyprland/src/service.rs`           | Done   |
| `CreateWorkspaceMessage` handler              | `services/hyprland/src/service.rs`           | Done   |
| `WorkspaceSnapshotRequestMessage` handler     | `services/hyprland/src/service.rs`           | Done   |
| Launcher core routing (`compositor.*`)        | `smearor-swipe-launcher/src/messages/mod.rs` | Done   |
| Launcher core routing (`service.*.status`)    | `smearor-swipe-launcher/src/messages/mod.rs` | Done   |
| GNOME service (same compositor-unified model) | `services/gnome/src/service.rs`              | Done   |

### 11.2 Not Yet Implemented (Hyprland-Specific Tier)

| Component                                                                               | File                                                              | Status  |
|-----------------------------------------------------------------------------------------|-------------------------------------------------------------------|---------|
| `HyprlandWindowEventData`                                                               | `model/hyprland/src/messages/shared/window_event_data.rs`         | Pending |
| `HyprlandWindowOpenEvent`                                                               | `model/hyprland/src/messages/shared/window_open_event.rs`         | Pending |
| `HyprlandWindowMoveEvent`                                                               | `model/hyprland/src/messages/shared/window_move_event.rs`         | Pending |
| `HyprlandWindowFloatEventData`                                                          | `model/hyprland/src/messages/shared/window_float_event_data.rs`   | Pending |
| `HyprlandLayoutEvent`                                                                   | `model/hyprland/src/messages/shared/layout_event.rs`              | Pending |
| `HyprlandWindowTitleEventData`                                                          | `model/hyprland/src/messages/shared/window_title_event_data.rs`   | Pending |
| `HyprlandNonSpecialWorkspaceData`                                                       | `model/hyprland/src/messages/shared/non_special_workspace.rs`     | Pending |
| `HyprlandChangedSpecialEventData`                                                       | `model/hyprland/src/messages/shared/changed_special.rs`           | Pending |
| `HyprlandScreencastEventData`                                                           | `model/hyprland/src/messages/shared/screencast.rs`                | Pending |
| `HyprlandWindowPinEventData`                                                            | `model/hyprland/src/messages/shared/window_pin.rs`                | Pending |
| `HyprlandGroupToggledEventData`                                                         | `model/hyprland/src/messages/shared/group_toggled.rs`             | Pending |
| `ActiveWindowChangedStatusMessage`                                                      | `model/hyprland/src/messages/status/active_window_changed.rs`     | Pending |
| `FullscreenStateChangedStatusMessage`                                                   | `model/hyprland/src/messages/status/fullscreen_state_changed.rs`  | Pending |
| `WindowOpenedStatusMessage`                                                             | `model/hyprland/src/messages/status/window_opened.rs`             | Pending |
| `WindowClosedStatusMessage`                                                             | `model/hyprland/src/messages/status/window_closed.rs`             | Pending |
| `WindowMovedStatusMessage`                                                              | `model/hyprland/src/messages/status/window_moved.rs`              | Pending |
| `KeyboardLayoutChangedStatusMessage`                                                    | `model/hyprland/src/messages/status/keyboard_layout_changed.rs`   | Pending |
| `SubMapChangedStatusMessage`                                                            | `model/hyprland/src/messages/status/sub_map_changed.rs`           | Pending |
| `LayerOpenedStatusMessage`                                                              | `model/hyprland/src/messages/status/layer_opened.rs`              | Pending |
| `LayerClosedStatusMessage`                                                              | `model/hyprland/src/messages/status/layer_closed.rs`              | Pending |
| `FloatStateChangedStatusMessage`                                                        | `model/hyprland/src/messages/status/float_state_changed.rs`       | Pending |
| `UrgentStateChangedStatusMessage`                                                       | `model/hyprland/src/messages/status/urgent_state_changed.rs`      | Pending |
| `WindowTitleChangedStatusMessage`                                                       | `model/hyprland/src/messages/status/window_title_changed.rs`      | Pending |
| `WorkspaceRenamedStatusMessage`                                                         | `model/hyprland/src/messages/status/workspace_renamed.rs`         | Pending |
| `SpecialRemovedStatusMessage`                                                           | `model/hyprland/src/messages/status/special_removed.rs`           | Pending |
| `ChangedSpecialStatusMessage`                                                           | `model/hyprland/src/messages/status/changed_special.rs`           | Pending |
| `ScreencastStatusMessage`                                                               | `model/hyprland/src/messages/status/screencast.rs`                | Pending |
| `ConfigReloadedStatusMessage`                                                           | `model/hyprland/src/messages/status/config_reloaded.rs`           | Pending |
| `IgnoreGroupLockStateChangedStatusMessage`                                              | `model/hyprland/src/messages/status/ignore_group_lock_changed.rs` | Pending |
| `LockGroupsStateChangedStatusMessage`                                                   | `model/hyprland/src/messages/status/lock_groups_changed.rs`       | Pending |
| `WindowPinnedStatusMessage`                                                             | `model/hyprland/src/messages/status/window_pinned.rs`             | Pending |
| `GroupToggledStatusMessage`                                                             | `model/hyprland/src/messages/status/group_toggled.rs`             | Pending |
| `WindowMovedIntoGroupStatusMessage`                                                     | `model/hyprland/src/messages/status/window_moved_into_group.rs`   | Pending |
| `WindowMovedOutOfGroupStatusMessage`                                                    | `model/hyprland/src/messages/status/window_moved_out_of_group.rs` | Pending |
| `HyprlandStatusEvent` enum                                                              | `model/hyprland/src/messages/status_event.rs`                     | Pending |
| `HyprlandStatusMessage` envelope                                                        | `model/hyprland/src/messages/status_message.rs`                   | Pending |
| `status/` module declarations                                                           | `model/hyprland/src/messages/status/mod.rs`                       | Pending |
| `shared/` module declarations                                                           | `model/hyprland/src/messages/shared/mod.rs`                       | Pending |
| Consolidated event listener (thin dispatcher)                                           | `services/hyprland/src/event_listener/listener.rs`                | Pending |
| Thin dispatch worker                                                                    | `services/hyprland/src/event_listener/worker.rs`                  | Pending |
| `event_listener/` module declarations                                                   | `services/hyprland/src/event_listener/mod.rs`                     | Pending |
| Status `register_handlers`                                                              | `services/hyprland/src/status/mod.rs`                             | Pending |
| Status `process_event`                                                                  | `services/hyprland/src/status/worker.rs`                          | Pending |
| Status `RateLimiter`                                                                    | `services/hyprland/src/status/rate_limiter.rs`                    | Pending |
| Refactor: workspace `register_handlers` + `process_event`                               | `services/hyprland/src/workspace/mod.rs`                          | Pending |
| Refactor: monitor `register_handlers` + `process_event`                                 | `services/hyprland/src/monitor/mod.rs`                            | Pending |
| Refactor: remove `monitor/listener.rs`                                                  | `services/hyprland/src/monitor/listener.rs`                       | Pending |
| `HyprlandStateRequestMessage`                                                           | `model/hyprland/src/messages/state_request.rs`                    | Pending |
| `HyprlandStateMessage`                                                                  | `model/hyprland/src/messages/state_request.rs`                    | Pending |
| `HyprlandStateRequestMessage` handler                                                   | `services/hyprland/src/service.rs`                                | Pending |
| `HyprlandCommand::StateRequest` variant                                                 | `services/hyprland/src/service.rs`                                | Pending |
| `WorkspaceEvent` / `MonitorEvent` dispatch enums                                        | `services/hyprland/src/event_listener/mod.rs`                     | Pending |
| `convert_*` functions (workspace, monitor, status)                                      | `services/hyprland/src/{workspace,monitor,status}/mod.rs`         | Pending |
| `broadcast_event` helper (already exists from Phase 1)                                  | `services/hyprland/src/service.rs`                                | Done    |
| Unknown event handler (`add_unknown_handler`)                                           | `services/hyprland/src/status/mod.rs`                             | Pending |
| `enable_status_events` config flag                                                      | `services/hyprland/src/config.rs`                                 | Pending |
| Service construction (status listener)                                                  | `services/hyprland/src/service.rs`                                | Pending |
| JSON converters: 3 top-level (`impl_json_convertible!`)                                 | `model/hyprland/src/lib.rs` (Section 5.4)                         | Pending |
| `stabby` `serde` feature in `model/hyprland/Cargo.toml`                                 | `model/hyprland/Cargo.toml`                                       | Pending |
| `HyprlandStatusEvent::None` `#[default]` variant                                        | `model/hyprland/src/messages/status_event.rs`                     | Pending |
| Launcher core: unified `service.*.status*` / `service.*.command*` routing               | `smearor-swipe-launcher/src/messages/mod.rs`                      | Pending |
| Topic migration: `service.hyprland.ctl` → `service.hyprland.command.ctl`                | `model/hyprland/src/messages/command/kill.rs`                     | Pending |
| Topic migration: `service.hyprland.dispatch` → `service.hyprland.command.dispatch`      | `model/hyprland/src/messages/dispatch/workspace.rs`               | Pending |
| Topic migration: `service.http.request` → `service.http.status.request`                 | `model/http/src/topics.rs`                                        | Pending |
| Topic migration: `service.macropad.connection` → `service.macropad.status.connection`   | `model/macropad/src/topics.rs`                                    | Pending |
| Topic migration: `service.macropad.input` → `service.macropad.status.input`             | `model/macropad/src/topics.rs`                                    | Pending |
| Topic migration: `service.network.scan_results` → `service.network.status.scan_results` | `model/network/src/messages/scan_results.rs`                      | Pending |
| Topic migration: `service.network.vpn_profiles` → `service.network.status.vpn_profiles` | `model/network/src/messages/vpn_profiles_message.rs`              | Pending |

### 11.3 Summary

- **Compositor-unified tier** (workspace change, workspace lifecycle, monitor hotplug, workspace snapshot, switch/create commands): **100% implemented** for
  both Hyprland and GNOME.
- **Hyprland-specific tier** (24 event types: active window, fullscreen, window open/close/move, keyboard layout, submap, layer open/close, float, urgent,
  window title, workspace renamed, special removed, changed special, screencast, config reloaded, ignore group lock, lock groups, window pinned, group toggled,
  window moved into/out of group; plus state request/response): **0% implemented** — all 24 message types, 11 shared types, the unified enum, state
  request/response types, consolidated listener, and unified worker with rate limiting are pending.
- **Refactor**: existing `workspace/mod.rs` listener and `monitor/listener.rs` are replaced by the consolidated `event_listener/` module. Processing logic stays
  atomic in per-domain `process_event` functions. The listener is a thin dispatcher only — no monster-classes, monster-enums, or monster-functions. The
  `HyprlandEvent` dispatch enum has only 3 variants (one per domain).
