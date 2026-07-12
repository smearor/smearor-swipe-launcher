# Hyprland Workspace Tracking Concept

## 1. Goal

Track the active workspace on each monitor via Hyprland's IPC event listener, and broadcast workspace
change events to launcher instances so they can react (e.g. switch layout profiles, update widget
state). This feature is **Hyprland-only** — it relies on the `hyprland` crate's `EventListener` API
and Hyprland's socket protocol.

## 2. Scope

- **In scope:** Detecting workspace changes per monitor, broadcasting events to launcher instances,
  enabling `LayoutTrigger::Workspace` and `MonitorIndexWorkspace` from `LAYOUT_PROFILE_CONCEPT.md`.
- **Out of scope:** Workspace tracking on other compositors (sway, river, etc.). The architecture is
  designed to allow future compositor-specific implementations, but only Hyprland is implemented here.

## 3. Current State

### 3.1 Hyprland Service

The existing `HyprlandService` (`services/hyprland/src/service.rs`) uses the `hyprland` crate for
**outbound** dispatch commands (exec, workspace switch, move focus, etc.). It does not currently
listen for **inbound** events from Hyprland.

The service already has:

- An async worker thread with a tokio runtime
- A `command_sender` / `command_receiver` channel for dispatch commands
- `MessageBroadcaster` trait implementation (currently empty — `impl MessageBroadcaster for HyprlandService {}`)
- Access to `FfiCoreContext` for broadcasting messages to launcher instances

### 3.2 Model Crate

`model/hyprland` defines dispatch messages and their stabby-compatible types. There are no event
message types yet — only dispatch (outbound) messages exist.

### 3.3 Layout Trigger Infrastructure

`LayoutTrigger` in `smearor-swipe-launcher/src/config/layout/trigger.rs` already has `Workspace(i32)`
and `MonitorWorkspace { monitor: String, workspace: i32 }` variants. The `get_layout_for_context`
function in `smearor-swipe-launcher/src/config/launcher.rs` already matches these triggers, but is
never called at runtime (see `LAYOUT_PROFILE_CONCEPT.md`).

## 4. Architecture

### 4.1 Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         HyprlandService                             │
│                                                                     │
│  ┌──────────────┐        ┌──────────────────┐                      │
│  │  Event       │        │  Dispatch        │                      │
│  │  Listener    │        │  Worker          │                      │
│  │  (async)     │        │  (existing)      │                      │
│  │              │        │                  │                      │
│  │  workspace   │        │  command_receiver│                      │
│  │  change ─────┼───┐    │                  │                      │
│  │  monitor     │   │    └──────────────────┘                      │
│  │  added ──────┼───┤                                              │
│  │  monitor     │   │    ┌──────────────────┐                      │
│  │  removed ────┼───┤    │  Event Channel   │                      │
│  └──────────────┘   │    │  (mpsc)          │                      │
│                     └───►│  event_sender ──►│                      │
│                          └────────┬─────────┘                      │
│                                   │                                  │
│                                   ▼                                  │
│                          ┌──────────────────┐                      │
│                          │  Event Worker     │                      │
│                          │  (async loop)     │                      │
│                          │                  │                      │
│                          │  broadcast to    │                      │
│                          │  launcher core   │                      │
│                          └──────────────────┘                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 Event Flow

1. Hyprland sends an event via its UNIX socket.
2. The `hyprland` crate's `EventListener` receives and parses it.
3. The event handler converts it to a `WorkspaceChangedEvent` message.
4. The event is sent through an `mpsc` channel to the event worker.
5. The event worker broadcasts the message via `FfiCoreContext` to all launcher instances.
6. Launcher instances receive the message and re-evaluate their layout profile.

## 5. Proposed Implementation

### 5.1 Model: Event Messages

Add event message types to `model/hyprland/src/messages/`. Create a new `event` module:

```rust
// model/hyprland/src/messages/event.rs

use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for workspace change events broadcast by the Hyprland service.
pub const TOPIC_WORKSPACE_CHANGED: &str = "hyprland::workspace_changed";

/// Information about a workspace change event.
///
/// Broadcast by the Hyprland service when the active workspace changes on a monitor.
/// Launcher instances use this to re-evaluate layout profiles.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceChangedEvent {
    /// The workspace name or number that became active.
    pub workspace_name: stabby::string::String,
    /// The workspace ID (numeric, as reported by Hyprland).
    pub workspace_id: i32,
    /// The monitor name on which the workspace change occurred.
    pub monitor_name: stabby::string::String,
}

impl TypedMessage for WorkspaceChangedEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceChangedEvent");
}

impl MessageTopic for WorkspaceChangedEvent {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_CHANGED
    }
}

impl SharedMessage for WorkspaceChangedEvent {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_CHANGED
    }
}
```

Register the JSON converter for this type in `model/hyprland/src/json_converters.rs` so the launcher
core can deserialize it.

### 5.2 Service: Event Listener

Extend `HyprlandService` with an event listener thread. Add a new `HyprlandEvent` enum for the
event channel:

```rust
// services/hyprland/src/service.rs

/// Internal union of all event types the service listens for.
pub enum HyprlandEvent {
    WorkspaceChanged(WorkspaceChangedEvent),
    MonitorAdded(String),
    MonitorRemoved(String),
}
```

Extend `HyprlandService::new` to spawn an event listener thread:

```rust
impl HyprlandService {
    pub(crate) fn new(
        config: PluginConfig,
        core_context: Option<FfiCoreContext>,
    ) -> Result<Self, PluginConstructionErrorWrapper> {
        // ... existing setup ...

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<HyprlandEvent>();

        // Spawn event listener thread
        let event_core_context = core_context.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(error) => {
                    error!("Hyprland Service: failed to create event listener runtime: {error}");
                    return;
                }
            };

            rt.block_on(async move {
                let mut listener = match hyprland::event_listener::EventListener::new_async().await {
                    Ok(listener) => listener,
                    Err(error) => {
                        error!("Hyprland Service: failed to create event listener: {error}");
                        return;
                    }
                };

                // Workspace change handler
                let ws_sender = event_sender.clone();
                listener.add_workspace_change_handler(move |workspace_type| {
                    let workspace_name = match &workspace_type {
                        hyprland::shared::WorkspaceType::Regular(name) => name.clone(),
                        hyprland::shared::WorkspaceType::Special(name) => name.clone().unwrap_or_default(),
                    };
                    let workspace_id = match &workspace_type {
                        hyprland::shared::WorkspaceType::Regular(name) => name.parse().unwrap_or(-1),
                        hyprland::shared::WorkspaceType::Special(_) => -1,
                    };

                    let event = WorkspaceChangedEvent {
                        workspace_name: workspace_name.clone().into(),
                        workspace_id,
                        monitor_name: String::new().into(), // Filled below
                    };

                    // Query current monitor for the active workspace
                    // This requires a separate async call to hyprctl
                    let _ = ws_sender.send(HyprlandEvent::WorkspaceChanged(event));
                });

                // Monitor added handler
                let mon_sender = event_sender.clone();
                listener.add_monitor_added_handler(move |data| {
                    let _ = mon_sender.send(HyprlandEvent::MonitorAdded(data.to_string()));
                });

                // Monitor removed handler
                let mon_sender2 = event_sender.clone();
                listener.add_monitor_removed_handler(move |data| {
                    let _ = mon_sender2.send(HyprlandEvent::MonitorRemoved(data.to_string()));
                });

                // Start listening (this blocks the async runtime)
                if let Err(error) = listener.start_listener_async().await {
                    error!("Hyprland event listener stopped: {error}");
                }
            });
        });

        // Spawn event processing thread
        let event_core_context = core_context.clone();
        let service_meta = service.meta.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(error) => {
                    error!("Hyprland Service: failed to create event processor runtime: {error}");
                    return;
                }
            };

            rt.block_on(async move {
                while let Some(event) = event_receiver.recv().await {
                    match event {
                        HyprlandEvent::WorkspaceChanged(event) => {
                            debug!("Workspace changed: {:?}", event);
                            broadcast_event(&event_core_context, &service_meta, event);
                        }
                        HyprlandEvent::MonitorAdded(name) => {
                            debug!("Monitor added: {}", name);
                        }
                        HyprlandEvent::MonitorRemoved(name) => {
                            debug!("Monitor removed: {}", name);
                        }
                    }
                }
            });
        });

        Ok(service)
    }
}
```

### 5.3 Broadcasting Events

The event worker broadcasts `WorkspaceChangedEvent` messages to all launcher instances via the
`FfiCoreContext`:

```rust
fn broadcast_event(
    core_context: &Option<FfiCoreContext>,
    meta: &PluginMeta,
    event: WorkspaceChangedEvent,
) {
    let Some(ctx) = core_context else {
        return;
    };

    let broadcaster = MessageBroadcasterInner {
        core_context: ctx.clone(),
        source_id: meta.id.clone(),
    };

    broadcaster.broadcast_message_to_topic(event);
}
```

### 5.4 Workspace-to-Monitor Resolution

When a workspace change event fires, Hyprland reports the workspace name but not which monitor it
is on. To get the monitor, query `hyprctl monitors` after the event:

```rust
async fn resolve_monitor_for_workspace(workspace_id: i32) -> Option<String> {
    let monitors = match hyprland::data::Monitors::get_async().await {
        Ok(monitors) => monitors,
        Err(error) => {
            warn!("Failed to query monitors: {error}");
            return None;
        }
    };

    for monitor in monitors {
        if monitor.active_workspace.id == workspace_id {
            return Some(monitor.name);
        }
    }
    None
}
```

This query should be done in the event worker before broadcasting, so the `WorkspaceChangedEvent`
includes the monitor name. The event listener handler sends a preliminary event, and the event
worker enriches it:

```rust
// In the event worker loop:
HyprlandEvent::WorkspaceChanged( mut event) => {
if let Some(monitor_name) = resolve_monitor_for_workspace(event.workspace_id).await {
event.monitor_name = monitor_name.into();
}
broadcast_event( & event_core_context, & service_meta, event);
}
```

### 5.5 Launcher Instance: Receiving Events

The launcher core (`LauncherHost`) receives the `WorkspaceChangedEvent` via the broker and routes
it to instances. Each instance checks if the event is relevant (matches its monitor) and triggers
a layout re-evaluation:

```rust
// instance.rs — new method on LauncherInstance

impl LauncherInstance {
    pub fn on_workspace_changed(&self, event: &WorkspaceChangedEvent) {
        let monitor_index = self.config.launcher.layer.monitor;

        // Check if this event is relevant to this instance
        // The monitor_name in the event is a connector name (e.g. "DP-1"),
        // while the instance uses a monitor index. Resolution depends on
        // whether the instance's monitor matches the event's monitor.
        //
        // For now, all instances receive the event and re-evaluate their layout.
        // A future optimization can filter by monitor.

        let (areas, entries) = self.config.get_layout_for_context(
            Some(&event.monitor_name.to_string()),
            monitor_index,
            Some(event.workspace_id),
        );

        self.rebuild_areas(areas, entries);
    }
}
```

### 5.6 Message Handler Registration

The `LauncherHost` must register a handler for `WorkspaceChangedEvent` in its broker loop:

```rust
// application.rs — in the broker loop

id if id == FfiEnvelopePayload::<WorkspaceChangedEvent>::TYPE_ID => {
debug ! ("WorkspaceChangedEvent received");
MessageHandler::< FfiEnvelopePayload< WorkspaceChangedEvent > >::handle_envelope_message( self, envelope);
}
```

Implement `MessageHandler` on `LauncherHost`:

```rust
impl MessageHandler<FfiEnvelopePayload<WorkspaceChangedEvent>> for LauncherHost {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceChangedEvent>, _sender_id: &str) {
        let event = message.0;
        if let Ok(instances) = self.instances.lock() {
            for instance in instances.values() {
                instance.on_workspace_changed(&event);
            }
        }
    }
}
```

## 6. Configuration

### 6.1 Service Configuration

Add an optional `enable_workspace_tracking` flag to `HyprlandServiceConfig`:

```rust
// services/hyprland/src/config.rs

/// Configuration for the Hyprland service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct HyprlandServiceConfig {
    /// Optional path override for the Hyprland socket.
    pub socket_path: Option<String>,
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
}
```

In `services.toml`:

```toml
[[services]]
id = "hyprland"
path = "target/release/libsmearor_hyprland_service.so"

[hyprland]
enable_workspace_tracking = true
```

When `enable_workspace_tracking` is `false`, the event listener thread is not spawned, saving
resources on systems where workspace tracking is not needed.

### 6.2 Layout Profile Configuration

With workspace tracking enabled, users can define layout profiles that react to workspace changes:

```toml
# Different layout when workspace 2 is active on monitor 0
[[profiles]]
trigger = { monitor_index = 0, workspace = 2 }
areas = ["workspace_2_layout"]

[workspace_2_layout]
type = "scroll"
plugins = [{ id = "clock_ws2", path = "target/debug/libsmearor_clock_widget.so" }]

# Default layout for monitor 0
[[profiles]]
trigger = { monitor_index = 0 }
areas = ["default_layout"]

[default_layout]
type = "scroll"
plugins = [
    { id = "clock", path = "target/debug/libsmearor_clock_widget.so" },
    { id = "app_launcher", path = "target/debug/libsmearor_app_launcher_widget.so" }
]
```

## 7. Hyprland Event Reference

The `hyprland` crate's `EventListener` provides these relevant handlers:

| Handler                             | Event Data        | Use Case                             |
|-------------------------------------|-------------------|--------------------------------------|
| `add_workspace_change_handler`      | `WorkspaceType`   | Active workspace changed             |
| `add_workspace_added_handler`       | `WorkspaceType`   | New workspace created                |
| `add_workspace_destroy_handler`     | `WorkspaceType`   | Workspace destroyed                  |
| `add_workspace_moved_handler`       | `WorkspaceType`   | Workspace moved to different monitor |
| `add_active_monitor_change_handler` | `Option<Monitor>` | Active monitor changed               |
| `add_monitor_added_handler`         | `String`          | Monitor connected                    |
| `add_monitor_removed_handler`       | `String`          | Monitor disconnected                 |

**Primary handler:** `add_workspace_change_handler` — fires when the user switches to a different
workspace. This is the main trigger for layout profile re-evaluation.

**Secondary handlers:** `add_workspace_moved_handler` and `add_active_monitor_change_handler` —
fire when workspaces are moved between monitors or the active monitor changes. These are relevant
for the `MonitorWorkspace` trigger combination.

## 8. Edge Cases

- **Hyprland not running** — The `EventListener::start_listener_async` call fails. The service logs
  an error and continues without workspace tracking. Dispatch commands still work if Hyprland
  starts later.
- **Special workspaces** — Hyprland's `WorkspaceType::Special` does not have a numeric ID. The
  `workspace_id` is set to `-1` for special workspaces. Layout profiles using `Workspace(i32)`
  triggers will not match special workspaces.
- **Multiple Hyprland instances** — The `ensure_hyprland_instance_signature` function (already
  implemented) handles this. The event listener connects to the same instance as the dispatch
  worker.
- **Rapid workspace switching** — Each workspace change triggers a layout rebuild. If the user
  switches rapidly, multiple rebuilds are queued. The `AreaManager::clear_areas()` + rebuild
  sequence is synchronous on the GTK main loop, so there is no race condition, but there may be
  visual flicker. A debounce can be added if needed.
- **Event listener disconnects** — If the Hyprland socket is lost (e.g. Hyprland restarts), the
  listener stops. The service should detect this and attempt to reconnect. A reconnect loop with
  exponential backoff is recommended.

## 9. Reconnection Strategy

If the event listener stops (Hyprland restart, socket error), the service should attempt to
reconnect:

```rust
// In the event listener thread, wrap start_listener_async in a retry loop

loop {
let mut listener = match hyprland::event_listener::EventListener::new_async().await {
Ok(listener) => listener,
Err(error) => {
error ! ("Failed to create event listener: {error}, retrying in 5s");
tokio::time::sleep(Duration::from_secs(5)).await;
continue;
}
};

// ... add handlers ...

if let Err(error) = listener.start_listener_async().await {
error ! ("Event listener stopped: {error}, reconnecting in 5s");
tokio::time::sleep(Duration::from_secs(5)).await;
}
}
```

## 10. Affected Files

| File                                        | Change                                                            |
|---------------------------------------------|-------------------------------------------------------------------|
| `model/hyprland/src/messages/event.rs`      | New file: `WorkspaceChangedEvent` struct with `#[stabby::stabby]` |
| `model/hyprland/src/messages/mod.rs`        | Add `pub mod event;` and re-export `WorkspaceChangedEvent`        |
| `model/hyprland/src/json_converters.rs`     | Register JSON converter for `WorkspaceChangedEvent`               |
| `services/hyprland/src/config.rs`           | Add `enable_workspace_tracking: bool` field                       |
| `services/hyprland/src/service.rs`          | Add event listener thread, event worker, `HyprlandEvent` enum     |
| `smearor-swipe-launcher/src/application.rs` | Add `MessageHandler` for `WorkspaceChangedEvent` in broker loop   |
| `smearor-swipe-launcher/src/instance.rs`    | Add `on_workspace_changed()` method                               |
| `services.toml`                             | Add `enable_workspace_tracking = true` under `[hyprland]`         |

## 11. Implementation Roadmap

### Phase 1: Model

| #   | Task                                                     | Files                                   | Effort |
|-----|----------------------------------------------------------|-----------------------------------------|--------|
| 1.1 | Create `WorkspaceChangedEvent` struct with stabby fields | `model/hyprland/src/messages/event.rs`  | Small  |
| 1.2 | Register module and re-exports                           | `model/hyprland/src/messages/mod.rs`    | Small  |
| 1.3 | Register JSON converter                                  | `model/hyprland/src/json_converters.rs` | Small  |

### Phase 2: Service Configuration

| #   | Task                                                       | Files                             | Effort |
|-----|------------------------------------------------------------|-----------------------------------|--------|
| 2.1 | Add `enable_workspace_tracking` to `HyprlandServiceConfig` | `services/hyprland/src/config.rs` | Small  |

### Phase 3: Event Listener

| #   | Task                                             | Files                              | Effort |
|-----|--------------------------------------------------|------------------------------------|--------|
| 3.1 | Add `HyprlandEvent` enum                         | `services/hyprland/src/service.rs` | Small  |
| 3.2 | Spawn event listener thread with `EventListener` | `services/hyprland/src/service.rs` | Medium |
| 3.3 | Add workspace change handler                     | `services/hyprland/src/service.rs` | Small  |
| 3.4 | Add monitor added/removed handlers               | `services/hyprland/src/service.rs` | Small  |
| 3.5 | Spawn event worker thread for broadcasting       | `services/hyprland/src/service.rs` | Medium |
| 3.6 | Implement `resolve_monitor_for_workspace()`      | `services/hyprland/src/service.rs` | Small  |
| 3.7 | Add reconnection loop                            | `services/hyprland/src/service.rs` | Small  |

### Phase 4: Launcher Integration

| #   | Task                                                          | Files            | Effort |
|-----|---------------------------------------------------------------|------------------|--------|
| 4.1 | Add `MessageHandler<WorkspaceChangedEvent>` to `LauncherHost` | `application.rs` | Small  |
| 4.2 | Add `on_workspace_changed()` to `LauncherInstance`            | `instance.rs`    | Small  |
| 4.3 | Wire broker to route `WorkspaceChangedEvent`                  | `application.rs` | Small  |

### Phase 5: Layout Profile Integration

| #   | Task                                                                                               | Files                  | Effort |
|-----|----------------------------------------------------------------------------------------------------|------------------------|--------|
| 5.1 | Implement `rebuild_areas()` in `LauncherInstance` (depends on `LAYOUT_PROFILE_CONCEPT.md` Phase 4) | `instance.rs`          | Medium |
| 5.2 | Implement `clear_areas()` in `AreaManager` (depends on `LAYOUT_PROFILE_CONCEPT.md` Phase 4)        | `area/area_manager.rs` | Medium |

### Phase 6: Testing & Validation

| #   | Task                                                                        | Effort |
|-----|-----------------------------------------------------------------------------|--------|
| 6.1 | Test event listener starts when `enable_workspace_tracking = true`          | Small  |
| 6.2 | Test event listener does not start when `enable_workspace_tracking = false` | Small  |
| 6.3 | Test workspace change broadcasts `WorkspaceChangedEvent`                    | Medium |
| 6.4 | Test `resolve_monitor_for_workspace()` returns correct monitor              | Small  |
| 6.5 | Test layout profile switch on workspace change                              | Medium |
| 6.6 | Test reconnection after Hyprland restart                                    | Medium |
| 6.7 | Test special workspace handling (workspace_id = -1)                         | Small  |

## 12. Dependencies

- **LAYOUT_PROFILE_CONCEPT.md** — Workspace tracking enables the `Workspace(i32)` and
  `MonitorIndexWorkspace` trigger variants. The `rebuild_areas()` and `clear_areas()` methods from
  `LAYOUT_PROFILE_CONCEPT.md` Phase 4 are required for runtime layout switching.
- **LAYER_SHELL_WINDOW_CONCEPT.md** — The monitor index from the layer shell config is used in
  `on_workspace_changed()` to filter events by monitor. The hotplug detection from Section 9
  complements workspace tracking: hotplug handles monitor changes, workspace tracking handles
  workspace changes.
- **`hyprland` crate** — Uses `EventListener` (async API), `data::Monitors::get_async()`, and
  `shared::WorkspaceType`. The crate must be added to `services/hyprland/Cargo.toml` with the
  `tokio` feature if not already enabled.
