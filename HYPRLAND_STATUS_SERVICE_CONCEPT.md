# Concept: Hyprland Status Service Plugin (Phase 2)

This document describes the second implementation phase of the **Hyprland Service** in the *Smearor Swipe Launcher*. It builds on top of the command/dispatch
service defined in `HYPRLAND_SERVICE_CONCEPT.md` and adds live status broadcasting from the Hyprland compositor.

## Scope of this Phase

Phase 1 sends commands to Hyprland. Phase 2 listens for Hyprland events and broadcasts them as typed, FFI-stable messages to subscribed widgets. This enables UI
widgets such as workspace bars, active-window indicators, and monitor switchers.

## 1. Architecture

The existing `HyprlandService` is extended with an additional async task that runs a `hyprland::event_listener::AsyncEventListener`. Every Hyprland event is
converted into a stabby status message and published to a topic. Widgets subscribe to the topic and update their UI accordingly.

```
+----------------------------+                 +----------------------------+
| Hyprland Compositor        |                 | Hyprland Service           |
| (socket2 event stream)     |                 | (Phase 1 + event listener) |
+----------------------------+                 +----------------------------+
             |                                             |
             | 1. Event (e.g., workspace>>9)                |
             |===========================================> |
             |                                             | 2. Convert to
             |                                             |    HyprlandStatusEvent
             |                                             |
             |                                             | 3. Broadcast
             | <===========================================| Topic: service.hyprland.status
             |                                               Payload: WorkspaceChanged
+----------------------------+
| Workspace Widget           |
+----------------------------+
```

## 2. Crate Structure

This phase extends the existing `smearor-hyprland-model` and `smearor-hyprland-service` crates. New types are added in their own files per `AGENTS.md`.

```
model/hyprland/
  src/
    messages/
      status/
        mod.rs
        workspace_changed.rs        # WorkspaceChangedStatusMessage
        workspace_added.rs            # WorkspaceAddedStatusMessage
        workspace_destroyed.rs        # WorkspaceDestroyedStatusMessage
        workspace_moved.rs            # WorkspaceMovedStatusMessage
        active_monitor_changed.rs     # ActiveMonitorChangedStatusMessage
        active_window_changed.rs      # ActiveWindowChangedStatusMessage
        fullscreen_state_changed.rs   # FullscreenStateChangedStatusMessage
        monitor_added.rs              # MonitorAddedStatusMessage
        monitor_removed.rs            # MonitorRemovedStatusMessage
        window_opened.rs              # WindowOpenedStatusMessage
        window_closed.rs              # WindowClosedStatusMessage
        window_moved.rs               # WindowMovedStatusMessage
        keyboard_layout_changed.rs    # KeyboardLayoutChangedStatusMessage
        sub_map_changed.rs            # SubMapChangedStatusMessage
        layer_opened.rs               # LayerOpenedStatusMessage
        layer_closed.rs               # LayerClosedStatusMessage
        float_state_changed.rs        # FloatStateChangedStatusMessage
        urgent_state_changed.rs       # UrgentStateChangedStatusMessage
        minimize_state_changed.rs     # MinimizeStateChangedStatusMessage
        window_title_changed.rs       # WindowTitleChangedStatusMessage
      status_event.rs                 # HyprlandStatusEvent (unified enum)
      status_message.rs               # HyprlandStatusMessage (envelope)
      shared/
        workspace_type.rs             # HyprlandWorkspaceType
        window_event_data.rs          # HyprlandWindowEventData
        monitor_event_data.rs         # HyprlandMonitorEventData
        window_open_event.rs          # HyprlandWindowOpenEvent
        window_move_event.rs          # HyprlandWindowMoveEvent
        window_float_event_data.rs    # HyprlandWindowFloatEventData
        minimize_event_data.rs        # HyprlandMinimizeEventData
        layout_event.rs               # HyprlandLayoutEvent

services/hyprland/
  src/
    event_listener.rs                 # AsyncEventListener setup and task
    status_broadcaster.rs             # Conversion from hyprland events to messages
```

## 3. Shared Status Types (`messages/shared/`)

These mirror the supporting types from `hyprland::event_listener` and `hyprland::shared`.

| Rust Type                      | `#[stabby::stabby]` | Source                                           |
|--------------------------------|---------------------|--------------------------------------------------|
| `HyprlandWorkspaceType`        | yes                 | `hyprland::shared::WorkspaceType`                |
| `HyprlandWindowEventData`      | yes                 | `hyprland::event_listener::WindowEventData`      |
| `HyprlandMonitorEventData`     | yes                 | `hyprland::event_listener::MonitorEventData`     |
| `HyprlandWindowOpenEvent`      | yes                 | `hyprland::event_listener::WindowOpenEvent`      |
| `HyprlandWindowMoveEvent`      | yes                 | `hyprland::event_listener::WindowMoveEvent`      |
| `HyprlandWindowFloatEventData` | yes                 | `hyprland::event_listener::WindowFloatEventData` |
| `HyprlandMinimizeEventData`    | yes                 | `hyprland::event_listener::MinimizeEventData`    |
| `HyprlandLayoutEvent`          | yes                 | `hyprland::event_listener::LayoutEvent`          |

## 4. Status Message Types (`messages/status/`)

One message struct per `AsyncEventListener` handler event. All status messages are broadcast on the topic `service.hyprland.status`.

| Message Type                          | Event Source                          | Payload Fields                           |
|---------------------------------------|---------------------------------------|------------------------------------------|
| `WorkspaceChangedStatusMessage`       | `add_workspace_change_handler`        | `workspace: HyprlandWorkspaceType`       |
| `WorkspaceAddedStatusMessage`         | `add_workspace_added_handler`         | `workspace: HyprlandWorkspaceType`       |
| `WorkspaceDestroyedStatusMessage`     | `add_workspace_destroy_handler`       | `workspace: HyprlandWorkspaceType`       |
| `WorkspaceMovedStatusMessage`         | `add_workspace_moved_handler`         | `data: HyprlandMonitorEventData`         |
| `ActiveMonitorChangedStatusMessage`   | `add_active_monitor_change_handler`   | `data: HyprlandMonitorEventData`         |
| `ActiveWindowChangedStatusMessage`    | `add_active_window_change_handler`    | `data: Option<HyprlandWindowEventData>`  |
| `FullscreenStateChangedStatusMessage` | `add_fullscreen_state_change_handler` | `is_fullscreen: bool`                    |
| `MonitorAddedStatusMessage`           | `add_monitor_added_handler`           | `monitor_name: stabby::string::String`   |
| `MonitorRemovedStatusMessage`         | `add_monitor_removed_handler`         | `monitor_name: stabby::string::String`   |
| `WindowOpenedStatusMessage`           | `add_window_open_handler`             | `data: HyprlandWindowOpenEvent`          |
| `WindowClosedStatusMessage`           | `add_window_close_handler`            | `window_address: stabby::string::String` |
| `WindowMovedStatusMessage`            | `add_window_moved_handler`            | `data: HyprlandWindowMoveEvent`          |
| `KeyboardLayoutChangedStatusMessage`  | `add_keyboard_layout_change_handler`  | `data: HyprlandLayoutEvent`              |
| `SubMapChangedStatusMessage`          | `add_sub_map_change_handler`          | `sub_map: stabby::string::String`        |
| `LayerOpenedStatusMessage`            | `add_layer_open_handler`              | `layer_name: stabby::string::String`     |
| `LayerClosedStatusMessage`            | `add_layer_closed_handler`            | `layer_name: stabby::string::String`     |
| `FloatStateChangedStatusMessage`      | `add_float_state_handler`             | `data: HyprlandWindowFloatEventData`     |
| `UrgentStateChangedStatusMessage`     | `add_urgent_state_handler`            | `window_address: stabby::string::String` |
| `MinimizeStateChangedStatusMessage`   | `add_minimize_handler`                | `data: HyprlandMinimizeEventData`        |
| `WindowTitleChangedStatusMessage`     | `add_window_title_change_handler`     | `window_address: stabby::string::String` |

## 5. Unified Status Event Enum

A single enum wraps every status message so the service can broadcast one type through the plugin message system.

```rust
/// Unified enum for all Hyprland status events.
#[stabby::stabby]
#[derive(Clone, Debug)]
pub enum HyprlandStatusEvent {
    WorkspaceChanged(WorkspaceChangedStatusMessage),
    WorkspaceAdded(WorkspaceAddedStatusMessage),
    WorkspaceDestroyed(WorkspaceDestroyedStatusMessage),
    WorkspaceMoved(WorkspaceMovedStatusMessage),
    ActiveMonitorChanged(ActiveMonitorChangedStatusMessage),
    ActiveWindowChanged(ActiveWindowChangedStatusMessage),
    FullscreenStateChanged(FullscreenStateChangedStatusMessage),
    MonitorAdded(MonitorAddedStatusMessage),
    MonitorRemoved(MonitorRemovedStatusMessage),
    WindowOpened(WindowOpenedStatusMessage),
    WindowClosed(WindowClosedStatusMessage),
    WindowMoved(WindowMovedStatusMessage),
    KeyboardLayoutChanged(KeyboardLayoutChangedStatusMessage),
    SubMapChanged(SubMapChangedStatusMessage),
    LayerOpened(LayerOpenedStatusMessage),
    LayerClosed(LayerClosedStatusMessage),
    FloatStateChanged(FloatStateChangedStatusMessage),
    UrgentStateChanged(UrgentStateChangedStatusMessage),
    MinimizeStateChanged(MinimizeStateChangedStatusMessage),
    WindowTitleChanged(WindowTitleChangedStatusMessage),
}

/// The main status envelope broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug)]
pub struct HyprlandStatusMessage {
    pub event: HyprlandStatusEvent,
}
```

## 6. Service Extension

### 6.1 Service Struct Update

The `HyprlandService` from Phase 1 is extended with a `status_broadcaster` sender.

```rust
use tokio::sync::mpsc;

pub struct HyprlandService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<HyprlandCommand>,
    pub status_broadcaster: tokio::sync::mpsc::UnboundedSender<HyprlandStatusEvent>,
}
```

### 6.2 Event Listener Task

An additional async task is spawned during service construction.

```rust
async fn run_event_listener(
    status_sender: tokio::sync::mpsc::UnboundedSender<HyprlandStatusEvent>,
) -> hyprland::Result<()> {
    use hyprland::event_listener::AsyncEventListener;

    let mut listener = AsyncEventListener::new();

    listener.add_workspace_change_handler(move |workspace| {
        let sender = status_sender.clone();
        Box::pin(async move {
            let _ = sender.send(HyprlandStatusEvent::WorkspaceChanged(
                WorkspaceChangedStatusMessage {
                    workspace: convert_workspace_type(workspace),
                },
            ));
        })
    });

    listener.add_active_window_change_handler(move |data| {
        let sender = status_sender.clone();
        Box::pin(async move {
            let _ = sender.send(HyprlandStatusEvent::ActiveWindowChanged(
                ActiveWindowChangedStatusMessage {
                    data: data.map(convert_window_event_data),
                },
            ));
        })
    });

    // ... register all remaining handlers

    listener.start_listener_async().await
}
```

### 6.3 Status Broadcast Loop

A second async loop receives converted status events and publishes them through the plugin message system.

```rust
async fn run_status_broadcast_loop(
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<HyprlandStatusEvent>,
    core_context: Option<FfiCoreContext>,
) {
    while let Some(event) = receiver.recv().await {
        let message = HyprlandStatusMessage { event };
        if let Some(context) = &core_context {
            context.broadcast_message(message);
        }
    }
}
```

### 6.4 Required Trait Update

The `MessageBroadcaster` trait is already implemented. The service now uses the `core_context` broadcaster inside the status loop to emit messages.

## 7. Widget Example: Active Window Label

A minimal widget that listens for active-window changes:

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

The `smearor-hyprland-model` crate already depends on the plugin API. No additional dependency is required for the model crate.

The `smearor-hyprland-service` crate already depends on `hyprland` and `tokio`. The `AsyncEventListener` is part of the same `hyprland` crate, so no new
dependency is needed.

## 9. Open Questions & Risks

1. **Event listener lifecycle**: `AsyncEventListener::start_listener_async()` blocks until the listener stops. It must run in a dedicated task that is cancelled
   when the service is shut down.
2. **Socket connection**: The event listener uses the same Hyprland socket as the dispatch commands. If the socket is unavailable, the listener task fails and
   should be restarted with a backoff strategy.
3. **Rate limiting**: Some events (e.g., window title changes) can fire rapidly. Consider coalescing or throttling high-frequency status broadcasts.
4. **Widget subscription model**: The plugin system must support topic subscription so widgets receive only relevant broadcasts. A widget can filter by
   `HyprlandStatusEvent` variant on the client side.
5. **Initial state**: On startup, widgets have no state until the next event. Consider an explicit `RequestStateCommandMessage` in a later phase that fetches
   the current state from `hyprland::data` modules.

## 10. Relationship to Phase 1

This document is intentionally additive. The command service from `HYPRLAND_SERVICE_CONCEPT.md` remains unchanged. Phase 2 adds the event listener, status
messages, and broadcast loop to the same service crate. The service can be implemented and deployed incrementally: first commands, then status events.
