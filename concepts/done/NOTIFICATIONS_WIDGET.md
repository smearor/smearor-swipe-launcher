# Notifications Widget Architecture

## Overview

This document describes the architecture for the notifications widget following the workspace's plugin-service-model pattern. The implementation consists of
three separate crates that communicate via typed message passing.

## Crate Structure

```
model/notifications/       # Shared message types
services/notifications/    # Backend service plugin
plugins/notifications/     # GTK4 UI widget plugin
```

---

## 1. model/notifications

**Purpose:** Defines all message types exchanged between the widget and the service. This crate has no plugin dependencies and serves as the contract between UI
and backend.

### Files

- `Cargo.toml` — Dependencies: `serde`, `smearor_swipe_launcher_plugin_api`
- `src/lib.rs` — Exports the messages module
- `src/messages/mod.rs` — Declares `command` and `status` sub-modules
- `src/messages/command.rs` — Actions the widget can send to the service
- `src/messages/status.rs` — Status updates broadcast by the service

### Message Topics

- `service.notifications.command` — Widget → Service (user actions)
- `service.notifications.status` — Service → Widget (state updates)

### Command Message Types

```rust
pub enum NotificationCommandAction {
    Dismiss,        // Dismiss a single notification by ID
    DismissAll,     // Dismiss all visible notifications
    DismissLast,    // Dismiss the most recent notification
    InvokeAction,    // Invoke an action button on a notification
    ToggleDoNotDisturb, // Toggle DND mode
}
```

### Status Message Types

```rust
pub struct NotificationStatusMessage {
    pub do_not_disturb: bool,
    pub notifications: Vec<NotificationInfo>,
    pub unread_count: u32,
}

pub struct NotificationInfo {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: Option<String>,
    pub urgency: UrgencyLevel,
    pub actions: Vec<NotificationAction>,
    pub timestamp: u64,
    pub timeout_ms: i32,
}

pub enum UrgencyLevel {
    Low,
    Normal,
    Critical,
}
```

---

## 2. services/notifications

**Purpose:** Backend service that connects to the D-Bus notification daemon (`org.freedesktop.Notifications`), receives system notifications, maintains state,
and broadcasts updates to all listening widgets.

### Files

- `Cargo.toml` — Dependencies: `zbus`, `tokio`, `serde_json`, `tracing`, `model/notifications`
- `src/lib.rs` — Declares modules, implements `service_plugin!(NotificationService)`
- `src/service.rs` — Core service logic
- `src/config.rs` — `NotificationServiceConfig` struct with `parse` method

### Service Responsibilities

1. **D-Bus Connection:** Register as a notification client on `org.freedesktop.Notifications`
2. **Notification Reception:** Handle `Notify` signals from the D-Bus daemon
3. **Action Handling:** Process user-triggered actions (dismiss, invoke)
4. **State Management:** Maintain the list of active notifications
5. **Broadcasting:** Send `NotificationStatusMessage` updates to widgets

### Service Struct

```rust
pub struct NotificationService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NotificationServiceConfig,
    pub command_sender: Sender<NotificationCommand>,
    pub status_receiver: Arc<Mutex<Receiver<NotificationStatusMessage>>>,
}
```

### Implemented Traits

- `MessageHandler<FfiEnvelopePayload<NotificationCommandMessage>>`
- `MessageBroadcaster<NotificationStatusMessage>`
- `MessageTopicBroadcaster<NotificationStatusMessage>`
- `PluginMetaGetter`
- `AsRef<Option<FfiCoreContext>>`

---

## 3. plugins/notifications

**Purpose:** GTK4 UI widget that displays notifications, handles user interactions, and sends commands to the service via message broadcasting.

### Files

- `Cargo.toml` — Dependencies: `gtk4`, `glib`, `adw`, `model/notifications`
- `src/lib.rs` — Declares `config` and `widget` modules, implements `widget_plugin!(NotificationWidget)`
- `src/widget.rs` — Widget struct and UI logic
- `src/config.rs` — `NotificationWidgetConfig` struct with `parse` method

### UI Components

- **Notification List:** Scrollable list of active notifications
- **Notification Card:** Individual notification with icon, title, body, and action buttons
- **Dismiss Button:** Per-notification dismiss control
- **DND Toggle:** Do Not Disturb mode indicator/control
- **Clear All Button:** Dismiss all notifications

### Interaction Mapping

| Gesture                 | Action                             |
|-------------------------|------------------------------------|
| Primary click on card   | Invoke default action              |
| Secondary click on card | Dismiss notification               |
| Long press              | Show action buttons (if any)       |
| Swipe left              | Dismiss notification               |
| Scroll up/down          | Navigate through notification list |

### Widget Struct

```rust
pub struct NotificationWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NotificationWidgetConfig,
    pub status_receiver: Option<Receiver<NotificationStatusMessage>>,
    pub last_command_time: Arc<Mutex<Instant>>,
}
```

### Implemented Traits

- `MessageHandler<FfiEnvelopePayload<NotificationStatusMessage>>`
- `MessageBroadcaster<NotificationCommandMessage>`
- `MessageTopicBroadcaster<NotificationCommandMessage>`
- `PluginMetaGetter`
- `AsRef<Option<FfiCoreContext>>`
- `WidgetBuilder`

---

## Workspace Integration

### Cargo.toml (workspace root)

Add the new crates to workspace members:

```toml
[workspace]
members = [
    # ... existing members ...
    "model/notifications",
    "services/notifications",
    "plugins/notifications",
]
```

### config.toml (launcher configuration)

```toml
[services.notifications]
enabled = true
# Service-specific configuration

[widgets.notifications]
enabled = true
# Widget-specific configuration (position, styling, etc.)
```

---

## Message Flow

```
+---------------+    +------------------------+    +-------------------+
| D-Bus Daemon  |    | services/notifications |    | plugins/          |
| (freedesktop) |    | (NotificationService)   |    | notifications     |
+-------+-------+    +-----------+------------+    +---------+---------+
        |                        |                          |
        | Notify signal          |                          |
        +----------------------->|                          |
        |                        |                          |
        |                        | Broadcast status update  |
        |                        +------------------------->|
        |                        |                          |
        |                        | <------------------------+
        |                        | Command message          |
        |                        | (dismiss, invoke, etc.) |
        |                        |                          |
        |                        | Send action back to D-Bus|
        | <----------------------+                          |
```

---

## Implementation Order

1. **Create `model/notifications`** — Define all message types first
2. **Create `services/notifications`** — Implement D-Bus notification client
3. **Create `plugins/notifications`** — Build GTK4 UI widget
4. **Update workspace `Cargo.toml`** — Register new crates
5. **Update `config.toml`** — Configure the new widget and service

---

## Notes

- The service uses `zbus` for D-Bus communication with the notification daemon
- Notifications follow the [Desktop Notifications Specification](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html)
- The widget uses GTK4 containers (`ListBox`, `Box`, `Button`) for the notification list
- Action buttons are dynamically generated based on the `actions` field in `NotificationInfo`
- Urgency levels are visually differentiated (color coding, animation for critical)
- The service maintains a bounded history of dismissed notifications
- Debounce logic (150ms) applies to all gesture-based interactions

## Comparison with MPRIS Widget

| Aspect          | MPRIS                          | Notifications                   |
|-----------------|--------------------------------|---------------------------------|
| D-Bus Interface | `org.mpris.MediaPlayer2`       | `org.freedesktop.Notifications` |
| State           | Single active player           | Multiple active notifications   |
| Primary Action  | Play/Pause toggle              | Invoke default action / Dismiss |
| Visual          | Compact (album art + progress) | List-based (cards with actions) |
| Grouping        | Player rotation                | App-based grouping (optional)   |
| Commands        | 12 actions                     | 5 actions (simpler)             |