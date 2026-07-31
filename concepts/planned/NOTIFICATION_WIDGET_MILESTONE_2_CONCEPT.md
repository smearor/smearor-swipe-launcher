# Concept: Notifications Widget — Milestone 2 (Compact View-Based Tile)

This document describes the concept for the **Milestone 2** redesign of the Notifications Widget in the *Smearor Swipe Launcher*. Milestone 1 implemented a
list-based widget with full notification cards inside a `ScrolledWindow` (200px height). Milestone 2 replaces this with a **compact, view-based tile** (max
100px height) that mirrors the Weather Widget's swipe-to-cycle interaction pattern, combined with a **Layer Shell Overlay** for full notification details on
single-click.

The existing `model/notifications` and `services/notifications` crates remain unchanged. Only the widget crate (`plugins/notifications`) and its configuration
are reworked.

---

## 1. Motivation

The Milestone 1 widget occupies too much vertical space (200px `ScrolledWindow` with multiple notification cards). In the launcher's tile-based layout, space is
scarce — especially on the right static area where the widget is placed. The Weather Widget demonstrates an effective pattern for displaying multiple data
points in a compact tile via **view rotation** with swipe gestures. Milestone 2 applies this same pattern to notifications:

- **One view per unread notification** instead of a scrollable list
- **Swipe Up / Swipe Down** to cycle through notification views
- **Long-press** on a view to dismiss (mark as read) the notification
- **Single-click** to open a Layer Shell Overlay showing the full notification content
- **Max 100px height** — only the truncated summary (heading) is shown per view

---

## 2. Comparison: Milestone 1 vs Milestone 2

| Aspect              | Milestone 1                                  | Milestone 2                                    |
|---------------------|----------------------------------------------|------------------------------------------------|
| Layout              | `ScrolledWindow` with notification cards     | Compact tile, one view at a time               |
| Height              | 200px (fixed)                                | Max 100px (configurable, default 80px)         |
| Content per view    | Full card: icon, app, summary, body, actions | Truncated summary only (single `Label`)        |
| Navigation          | Scroll                                       | Swipe Up / Swipe Down (like Weather Widget)    |
| Dismiss interaction | Right-click or long-press on card            | Long-press on view                             |
| Full details        | Always visible in card                       | Single-click opens Layer Shell Overlay         |
| Empty state         | "No notifications" label in list             | "No notifications" label in tile               |
| DND indicator       | Badge in header                              | Icon overlay or dimmed state                   |
| Gesture pattern     | Widget-specific                              | Consistent with Weather Widget (`GestureDrag`) |

---

## 3. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Notification Widget      |                 | Notification Service       |
| (Milestone 2)            |                 | (Singleton, unchanged)     |
| (subscribed to           |                 |                            |
|  service.notifications   |                 |                            |
|  .status)                |                 |                            |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Command Message                         |
             |  (dismiss by ID)                            |
             |===========================================> |
             |  Topic: "service.notifications.command"    |
             |                                             |
             |  2. Status Broadcast                        |
             | <===========================================|
             |     Topic: "service.notifications.status"  |
             |     Payload: NotificationStatusMessage      |
+--------------------------+
| View Engine              |
| - current_view index     |
| - swipe up/down cycling  |
| - truncated summary      |
| - long-press dismiss     |
| - single-click overlay   |
+-----------+--------------+
            |
            | Single-click
            v
+--------------------------+
| Layer Shell Overlay      |
| (gtk4_layer_shell)       |
| Layer: Overlay           |
| - Full notification      |
|   summary + body         |
| - App name + icon        |
| - Timestamp              |
| - Action buttons         |
| - Dismiss button         |
+--------------------------+
```

---

## 4. Widget Crate (`plugins/notifications`) — Redesign

### 4.1 File Structure

| File         | Responsibility                                                 |
|--------------|----------------------------------------------------------------|
| `lib.rs`     | `widget_plugin!` macro invocation (unchanged)                  |
| `config.rs`  | `NotificationWidgetConfig` struct — reworked for Milestone 2   |
| `widget.rs`  | `NotificationWidget` struct and UI logic — completely reworked |
| `overlay.rs` | Layer Shell Overlay window for full notification display (new) |

### 4.2 Widget Configuration

```rust
/// Configuration for the notifications widget (Milestone 2).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NotificationWidgetConfig {
    /// Widget width in pixels.
    pub width: i32,
    /// Widget height in pixels (max 100).
    pub height: i32,
    /// Maximum number of characters to display in the truncated summary.
    pub max_summary_chars: usize,
    /// Spacing between child widgets inside the tile.
    pub spacing: i32,
    /// Whether to show the notification count badge.
    pub show_count: bool,
    /// Whether to show the DND indicator.
    pub show_dnd_indicator: bool,
    /// Swipe threshold in pixels for view switching.
    pub swipe_threshold: f64,
    /// Whether to show the app name in the tile (below summary).
    pub show_app_name: bool,
    /// Overlay width in pixels.
    pub overlay_width: i32,
    /// Overlay height in pixels.
    pub overlay_height: i32,
}

impl Default for NotificationWidgetConfig {
    fn default() -> Self {
        Self {
            width: 120,
            height: 80,
            max_summary_chars: 30,
            spacing: 2,
            show_count: true,
            show_dnd_indicator: true,
            swipe_threshold: 50.0,
            show_app_name: true,
            overlay_width: 400,
            overlay_height: 300,
        }
    }
}
```

### 4.3 Widget Struct

```rust
/// Compact view-based notifications widget (Milestone 2).
pub struct NotificationWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NotificationWidgetConfig,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<NotificationStatusMessage>>>,
    pub summary_label: Rc<RefCell<Option<Label>>>,
    pub app_label: Rc<RefCell<Option<Label>>>,
    pub count_label: Rc<RefCell<Option<Label>>>,
    pub status_sender: tokio::sync::mpsc::UnboundedSender<NotificationStatusMessage>,
    pub status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<NotificationStatusMessage>>,
}
```

> **GTK widget references:** As with the Weather Widget, GTK4 widgets are not `Send` or `Sync`. Widget references (`summary_label`, `app_label`, `count_label`)
> are stored as `Rc<RefCell<Option<Label>>>` and only accessed inside `glib::MainContext::spawn_local` closures.

### 4.4 Trait Implementations

- `MessageHandler<FfiEnvelopePayload<NotificationStatusMessage>>` — Receives status updates
- `MessageBroadcaster` — Sends dismiss commands to the service
- `MessageTopicBroadcaster<NotificationCommandMessage>` — Topic-scoped broadcasting
- `PluginMetaGetter` — Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` — Provides access to the core context
- `WidgetBuilder` — Builds the GTK4 widget UI

### 4.5 View Engine

The widget maintains a `current_view` index into the `notifications` vector from the latest `NotificationStatusMessage`. Each view corresponds to one unread
notification.

#### View Navigation

| Gesture      | Action                            |
|--------------|-----------------------------------|
| Swipe Up     | Advance to next notification view |
| Swipe Down   | Go to previous notification view  |
| Single-click | Open Layer Shell Overlay          |
| Long-press   | Dismiss current notification      |

```rust
fn next_view(&self) {
    let current_view = self.current_view.clone();
    let latest_status = self.latest_status.clone();
    let summary_label = self.summary_label.clone();
    let app_label = self.app_label.clone();
    let count_label = self.count_label.clone();
    let config = self.config.clone();

    MainContext::default().spawn_local(async move {
        let status = latest_status.borrow().clone();
        let Some(status) = status else { return };
        if status.notifications.is_empty() {
            return;
        }
        let mut idx = current_view.borrow_mut();
        *idx = (*idx + 1) % status.notifications.len();
        let view_index = *idx;
        drop(idx);

        render_view(&status, view_index, &config, &summary_label, &app_label, &count_label);
    });
}

fn prev_view(&self) {
    let current_view = self.current_view.clone();
    let latest_status = self.latest_status.clone();
    let summary_label = self.summary_label.clone();
    let app_label = self.app_label.clone();
    let count_label = self.count_label.clone();
    let config = self.config.clone();

    MainContext::default().spawn_local(async move {
        let status = latest_status.borrow().clone();
        let Some(status) = status else { return };
        if status.notifications.is_empty() {
            return;
        }
        let mut idx = current_view.borrow_mut();
        if *idx == 0 {
            *idx = status.notifications.len() - 1;
        } else {
            *idx -= 1;
        }
        let view_index = *idx;
        drop(idx);

        render_view(&status, view_index, &config, &summary_label, &app_label, &count_label);
    });
}
```

#### View Rendering

Each view renders a single truncated summary line and optionally the app name. The summary is truncated to `max_summary_chars` characters with an ellipsis (`…`)
suffix when exceeded.

```rust
fn render_view(
    status: &NotificationStatusMessage,
    view_index: usize,
    config: &NotificationWidgetConfig,
    summary_label: &Rc<RefCell<Option<Label>>>,
    app_label: &Rc<RefCell<Option<Label>>>,
    count_label: &Rc<RefCell<Option<Label>>>,
) {
    if status.do_not_disturb {
        if let Some(ref label) = *summary_label.borrow() {
            label.set_text("Do Not Disturb");
        }
        if let Some(ref label) = *app_label.borrow() {
            label.set_text("");
        }
        if let Some(ref label) = *count_label.borrow() {
            label.set_text("");
        }
        return;
    }

    if status.notifications.is_empty() {
        if let Some(ref label) = *summary_label.borrow() {
            label.set_text("No notifications");
        }
        if let Some(ref label) = *app_label.borrow() {
            label.set_text("");
        }
        if let Some(ref label) = *count_label.borrow() {
            label.set_text("");
        }
        return;
    }

    let notification = &status.notifications[view_index];
    let summary = truncate_summary(&notification.summary, config.max_summary_chars);

    if let Some(ref label) = *summary_label.borrow() {
        label.set_text(&summary);
    }
    if config.show_app_name {
        if let Some(ref label) = *app_label.borrow() {
            label.set_text(notification.app_name.as_str());
        }
    }
    if config.show_count {
        if let Some(ref label) = *count_label.borrow() {
            label.set_text(&format!("{}/{}", view_index + 1, status.notifications.len()));
        }
    }
}

fn truncate_summary(summary: &str, max_chars: usize) -> String {
    if summary.chars().count() <= max_chars {
        summary.to_string()
    } else {
        let truncated: String = summary.chars().take(max_chars).collect();
        format!("{truncated}\u{2026}")
    }
}
```

### 4.6 Gesture Handling

The gesture handling follows the Weather Widget pattern exactly, using `GestureDrag` for swipe detection, `GestureClick` for single-click, and
`GestureLongPress` for dismiss.

#### Swipe (GestureDrag)

```rust
let drag_gesture = GestureDrag::new();
drag_gesture.set_propagation_phase(PropagationPhase::Capture);
let widget_for_drag = widget_self.clone();
drag_gesture.connect_drag_end( move | gesture, offset_x, offset_y| {
let threshold = widget_for_drag.config.swipe_threshold;
if offset_y.abs() > offset_x.abs() & & offset_y.abs() > threshold {
gesture.set_state(EventSequenceState::Claimed);
if offset_y < 0.0 {
widget_for_drag.next_view();
} else {
widget_for_drag.prev_view();
}
}
});
outer_box.add_controller(drag_gesture);
```

#### Single-Click (GestureClick)

Single-click opens the Layer Shell Overlay (see Section 5) with the full notification content. The click must not fire if the gesture was claimed by the drag or
long-press handler.

```rust
let click_gesture = GestureClick::builder()
.button(0)
.propagation_phase(PropagationPhase::Capture)
.build();
let widget_for_click = widget_self.clone();
click_gesture.connect_released( move | gesture, _n_press, _x, _y| {
if let Some(seq) = gesture.current_sequence() {
let state = gesture.sequence_state( & seq);
if state == EventSequenceState::Claimed | | state == EventSequenceState::Denied {
return;
}
}
let status = widget_for_click.latest_status.borrow().clone();
let view_index = * widget_for_click.current_view.borrow();
if let Some(status) = status {
if ! status.notifications.is_empty() {
if let Some(notification) = status.notifications.get(view_index) {
let config = widget_for_click.config.clone();
show_notification_overlay(notification, & config);
}
}
}
gesture.set_state(EventSequenceState::Claimed);
});
outer_box.add_controller(click_gesture);
```

#### Long-Press (GestureLongPress)

Long-press dismisses the currently displayed notification by sending a `NotificationCommandMessage::dismiss_id(id)` to the service.

```rust
let longpress_gesture = GestureLongPress::builder()
.button(0)
.propagation_phase(PropagationPhase::Capture)
.build();
let widget_for_longpress = widget_self.clone();
let broadcaster = message_broadcaster.clone();
longpress_gesture.connect_pressed( move | gesture, _x, _y| {
let status = widget_for_longpress.latest_status.borrow().clone();
let view_index = * widget_for_longpress.current_view.borrow();
if let Some(status) = status {
if let Some(notification) = status.notifications.get(view_index) {
let notification_id = notification.id;
broadcaster.broadcast_message_to_topic(
NotificationCommandMessage::dismiss_id(notification_id)
);
}
}
gesture.set_state(EventSequenceState::Claimed);
});
outer_box.add_controller(longpress_gesture);
```

### 4.7 Status Update Handling

When a new `NotificationStatusMessage` arrives, the widget stores it and re-renders the current view. If the current view index is out of bounds (e.g., after a
dismissal reduced the notification count), it is clamped to the last valid index.

```rust
impl MessageHandler<FfiEnvelopePayload<NotificationStatusMessage>> for NotificationWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<NotificationStatusMessage>, _sender_id: &str) {
        let status = message.0;
        *self.latest_status.borrow_mut() = Some(status.clone());

        // Clamp current view index
        let mut idx = self.current_view.borrow_mut();
        if !status.notifications.is_empty() && *idx >= status.notifications.len() {
            *idx = status.notifications.len() - 1;
        } else if status.notifications.is_empty() {
            *idx = 0;
        }
        drop(idx);

        self.update_ui(&status);
    }
}
```

### 4.8 UI Layout

The tile consists of a vertical `GtkBox` with:

1. **Header row** (horizontal `GtkBox`): count badge (`1/3`) and DND indicator
2. **Summary label**: truncated notification summary, single line, ellipsized
3. **App name label**: small text showing the application name (optional)

```
+---------------------------+
| 1/3                       |  <- count badge (optional)
| New message from Alice... |  <- truncated summary
| Telegram                  |  <- app name (optional)
+---------------------------+
```

When there are no notifications:

```
+---------------------------+
|                           |
| No notifications          |
|                           |
+---------------------------+
```

When DND is active:

```
+---------------------------+
|                           |
| Do Not Disturb            |
|                           |
+---------------------------+
```

### 4.9 Widget Build

```rust
impl WidgetBuilder for NotificationWidget {
    fn build_widget(&mut self) -> Widget {
        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(self.config.spacing)
            .css_classes(["notifications-widget"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        outer_box.set_width_request(self.config.width);
        outer_box.set_height_request(self.config.height);

        let count_label = Label::builder()
            .css_classes(["notification-count"])
            .halign(Align::Start)
            .build();
        let summary_label = Label::builder()
            .css_classes(["notification-summary"])
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .single_line_mode(true)
            .build();
        let app_label = Label::builder()
            .css_classes(["notification-app"])
            .halign(Align::Start)
            .build();

        summary_label.set_text("Loading...");
        app_label.set_text("");
        count_label.set_text("");

        outer_box.append(&count_label);
        outer_box.append(&summary_label);
        if self.config.show_app_name {
            outer_box.append(&app_label);
        }

        *self.summary_label.borrow_mut() = Some(summary_label);
        *self.app_label.borrow_mut() = Some(app_label);
        *self.count_label.borrow_mut() = Some(count_label);

        // ... gesture controllers (see 4.6) ...

        outer_box.upcast::<Widget>()
    }
}
```

---

## 5. Layer Shell Overlay (`overlay.rs`)

### 5.1 Purpose

When the user single-clicks a notification view, a **Layer Shell Overlay window** appears showing the full notification content. This window is a separate
`gtk4::ApplicationWindow` that uses `gtk4_layer_shell` to position itself as an overlay above the launcher.

### 5.2 Overlay Window Structure

```rust
/// Shows a Layer Shell Overlay window with the full notification content.
fn show_notification_overlay(notification: &NotificationInfo, config: &NotificationWidgetConfig) {
    let window = ApplicationWindow::builder()
        .application(&Application::default().expect("No application"))
        .default_width(config.overlay_width)
        .default_height(config.overlay_height)
        .build();

    window.init_layer_shell();
    window.set_layer(gtk4_layer_shell::Layer::Overlay);
    window.set_anchor(gtk4_layer_shell::Edge::Top, true);
    window.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
    window.set_anchor(gtk4_layer_shell::Edge::Left, true);
    window.set_anchor(gtk4_layer_shell::Edge::Right, true);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    window.set_exclusive_zone(0);

    let main_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .css_classes(["notification-overlay"])
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    // Header: app name + icon
    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();

    if let Some(icon) = &notification.icon {
        let icon_label = Label::builder()
            .label(icon.as_str())
            .css_classes(["notification-overlay-icon"])
            .build();
        header.append(&icon_label);
    }

    let app_label = Label::builder()
        .label(notification.app_name.as_str())
        .css_classes(["notification-overlay-app"])
        .halign(Align::Start)
        .build();
    header.append(&app_label);

    // Close button
    let close_button = Button::builder()
        .icon_name("window-close-symbolic")
        .css_classes(["flat", "circular"])
        .halign(Align::End)
        .hexpand(true)
        .build();
    let window_clone = window.clone();
    close_button.connect_clicked(move |_| {
        window_clone.close();
    });
    header.append(&close_button);

    main_box.append(&header);

    // Summary (full, not truncated)
    let summary_label = Label::builder()
        .label(notification.summary.as_str())
        .css_classes(["notification-overlay-summary"])
        .halign(Align::Start)
        .wrap(true)
        .build();
    main_box.append(&summary_label);

    // Body (full)
    if !notification.body.is_empty() {
        let body_label = Label::builder()
            .label(notification.body.as_str())
            .css_classes(["notification-overlay-body"])
            .halign(Align::Start)
            .wrap(true)
            .build();
        main_box.append(&body_label);
    }

    // Timestamp
    let timestamp_label = Label::builder()
        .label(&format_timestamp(notification.timestamp))
        .css_classes(["notification-overlay-timestamp"])
        .halign(Align::Start)
        .build();
    main_box.append(&timestamp_label);

    // Action buttons
    if !notification.actions.is_empty() {
        let actions_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .build();
        for action in &notification.actions {
            let action_button = Button::builder()
                .label(action.label.as_str())
                .css_classes(["pill"])
                .build();
            // Note: Action invocation requires broadcaster access.
            // In the overlay, action buttons close the overlay.
            // Action invocation is handled via a callback or shared broadcaster.
            let window_clone = window.clone();
            action_button.connect_clicked(move |_| {
                // TODO: Broadcast NotificationCommandMessage::invoke_action
                window_clone.close();
            });
            actions_box.append(&action_button);
        }
        main_box.append(&actions_box);
    }

    // Dismiss button
    let dismiss_button = Button::builder()
        .label("Dismiss")
        .css_classes(["destructive-action"])
        .build();
    // Note: Dismiss requires broadcaster access.
    let window_clone = window.clone();
    dismiss_button.connect_clicked(move |_| {
        // TODO: Broadcast NotificationCommandMessage::dismiss_id
        window_clone.close();
    });
    main_box.append(&dismiss_button);

    window.set_child(Some(&main_box));
    window.present();
}
```

### 5.3 Overlay Interaction

| Element           | Action                                  |
|-------------------|-----------------------------------------|
| Close button (✕) | Close overlay window                    |
| Dismiss button    | Send dismiss command + close overlay    |
| Action button     | Invoke action + close overlay           |
| Escape key        | Close overlay (keyboard mode: OnDemand) |
| Click outside     | Close overlay (compositor-dependent)    |

### 5.4 Broadcaster Access in Overlay

The overlay window is created outside the widget struct's lifetime. To enable dismiss and action invocation from the overlay, the `MessageBroadcasterInner` is
cloned and passed into the `show_notification_overlay` function:

```rust
fn show_notification_overlay(
    notification: &NotificationInfo,
    config: &NotificationWidgetConfig,
    broadcaster: MessageBroadcasterInner,
) {
    // ... overlay creation ...

    let dismiss_notification_id = notification.id;
    let dismiss_broadcaster = broadcaster.clone();
    dismiss_button.connect_clicked(move |_| {
        dismiss_broadcaster.broadcast_message_to_topic(
            NotificationCommandMessage::dismiss_id(dismiss_notification_id)
        );
        window_clone.close();
    });

    // Similarly for action buttons:
    for action in &notification.actions {
        let action_notification_id = notification.id;
        let action_key = action.key.clone();
        let action_broadcaster = broadcaster.clone();
        let window_clone = window.clone();
        action_button.connect_clicked(move |_| {
            action_broadcaster.broadcast_message_to_topic(
                NotificationCommandMessage::invoke_action(action_notification_id, action_key.clone())
            );
            window_clone.close();
        });
    }
}
```

### 5.5 Overlay CSS Classes

| CSS Class                         | Element                          |
|-----------------------------------|----------------------------------|
| `.notification-overlay`           | Main overlay container           |
| `.notification-overlay-icon`      | App icon in overlay header       |
| `.notification-overlay-app`       | App name label in overlay header |
| `.notification-overlay-summary`   | Full summary (not truncated)     |
| `.notification-overlay-body`      | Full body text                   |
| `.notification-overlay-timestamp` | Formatted timestamp              |

---

## 6. CSS Styling

### 6.1 Tile CSS Classes

| CSS Class               | Element                         |
|-------------------------|---------------------------------|
| `.notifications-widget` | Outer container box             |
| `.notification-count`   | Count badge label (e.g., `1/3`) |
| `.notification-summary` | Truncated summary label         |
| `.notification-app`     | App name label                  |

### 6.2 Example CSS

```css
.notifications-widget {
    padding: 4px 8px;
    border-radius: 8px;
}

.notification-count {
    font-size: 10px;
    opacity: 0.6;
}

.notification-summary {
    font-size: 13px;
    font-weight: bold;
}

.notification-app {
    font-size: 10px;
    opacity: 0.7;
}

.notification-overlay {
    background-color: alpha(@theme_bg_color, 0.95);
    border-radius: 12px;
    padding: 16px;
}

.notification-overlay-summary {
    font-size: 16px;
    font-weight: bold;
}

.notification-overlay-body {
    font-size: 14px;
}

.notification-overlay-timestamp {
    font-size: 11px;
    opacity: 0.5;
}
```

---

## 7. Configuration Example

### 7.1 Widget Configuration in `config.toml`

```toml
[[right_area.plugins]]
id = "notifications"
path = "target/release/libsmearor_notifications_widget.so"

[notifications]
width = 120
height = 80
max_summary_chars = 30
spacing = 2
show_count = true
show_dnd_indicator = true
show_app_name = true
swipe_threshold = 50.0
overlay_width = 400
overlay_height = 300
```

### 7.2 Minimal Configuration

```toml
[notifications]
# All fields have sensible defaults — no configuration required
```

---

## 8. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
| Notification      |<--------|                   |-------->| Notification      |
| Widget (tile)     |  Status |   Event Broker    | Command | Service           |
|                   | Broadcast                  Broadcast +-------------------+
+---------+---------+         +-------------------+         |                   |
          |                                                 | zbus D-Bus       |
          | Swipe Up/Down: next_view() / prev_view()        | org.freedesktop   |
          | Long-press: dismiss_id(current)                 | .Notifications    |
          | Single-click: show_notification_overlay()        |                   |
          v                                                 v
+-------------------+                               +-------------------+
| View Engine       |                               | D-Bus Daemon      |
| (local state)     |                               | (freedesktop)     |
+-------------------+                               +-------------------+
          |
          | Single-click
          v
+-------------------+
| Layer Shell       |
| Overlay Window    |
| - Full summary    |
| - Full body       |
| - Action buttons  |
| - Dismiss button  |
+-------------------+
```

---

## 9. Edge Cases

- **No notifications:** The tile displays "No notifications" and swipe gestures are no-ops.
- **Single notification:** Swipe gestures are no-ops (only one view). Count badge shows `1/1`.
- **Notification dismissed while viewing:** The next status update clamps `current_view` to the last valid index. If the dismissed notification was the last
  one, the index wraps to 0 or the empty state is shown.
- **New notification arrives while viewing:** The notification is appended to the list. The current view does not change unless the user swipes.
- **DND active:** The tile shows "Do Not Disturb" regardless of notification count. Swipe gestures are no-ops.
- **Overlay already open:** A new single-click on a different view replaces the overlay content with the new notification.
- **Overlay and notification dismissal:** If the notification being displayed in the overlay is dismissed (via the overlay's dismiss button or externally), the
  overlay closes automatically on the next status update if the notification is no longer in the list.
- **Very long summary:** Truncated to `max_summary_chars` with ellipsis. The full summary is always available in the overlay.
- **Empty summary:** Falls back to the app name as the summary text. If both are empty, displays "(no title)".

---

## 10. Differences from Weather Widget Pattern

| Aspect       | Weather Widget                      | Notifications Widget (Milestone 2)         |
|--------------|-------------------------------------|--------------------------------------------|
| View source  | Static config list (`config.views`) | Dynamic from `status.notifications` vector |
| View count   | Fixed (configured at startup)       | Dynamic (changes with notification count)  |
| View content | Computed from single status message | One notification per view                  |
| Long-press   | Configurable topic/payload          | Fixed: dismiss current notification        |
| Single-click | Configurable topic/payload          | Fixed: open Layer Shell Overlay            |
| Overlay      | None                                | Layer Shell Overlay for full notification  |
| Empty state  | Shows "--" or error                 | Shows "No notifications"                   |

---

## 11. Implementation Order

### Phase 1: Widget Config Rework

**Goal:** Replace the Milestone 1 config with the Milestone 2 config struct.

**Order:**

1. Rework `NotificationWidgetConfig` in `plugins/notifications/src/config.rs` with the new fields.
2. Update `Default` implementation with sensible defaults (height max 100px).
3. Ensure backward compatibility: if old config fields are present, they are silently ignored.

**Exit criteria:**

- Config struct compiles with all new fields.
- `serde(default)` ensures missing fields don't cause parse errors.

---

### Phase 2: Widget Struct Rework

**Goal:** Replace the Milestone 1 widget struct with the view-based Milestone 2 struct.

**Order:**

1. Replace `NotificationWidget` struct fields with `current_view`, `latest_status`, and `Rc<RefCell<Option<Label>>>` fields for each UI element.
2. Update `new()` constructor to initialize the new fields.
3. Remove the `start_status_listener` method and `status_sender`/`status_receiver` pattern — replaced by direct `MessageHandler` + `update_ui` pattern (like
   Weather Widget).
4. Implement `update_ui(&self, status: &NotificationStatusMessage)` method.
5. Implement `next_view(&self)` and `prev_view(&self)` methods.
6. Implement `render_view()` free function.

**Exit criteria:**

- Widget struct compiles with all new fields.
- View navigation logic is implemented and tested.

---

### Phase 3: Gesture Handling

**Goal:** Implement swipe, click, and long-press gestures following the Weather Widget pattern.

**Order:**

1. Add `GestureDrag` for swipe up/down detection.
2. Add `GestureClick` for single-click overlay opening.
3. Add `GestureLongPress` for notification dismissal.
4. Ensure gesture state claiming works correctly (no conflicting gestures).

**Exit criteria:**

- Swipe up advances to next view.
- Swipe down goes to previous view.
- Long-press dismisses the current notification.
- Single-click opens the overlay (once Phase 4 is complete).

---

### Phase 4: Layer Shell Overlay

**Goal:** Implement the overlay window for full notification display.

**Order:**

1. Create `plugins/notifications/src/overlay.rs` with `show_notification_overlay()`.
2. Add `gtk4-layer-shell` dependency to `plugins/notifications/Cargo.toml`.
3. Implement overlay UI: header, summary, body, timestamp, action buttons, dismiss button.
4. Wire dismiss and action buttons to broadcast `NotificationCommandMessage`.
5. Handle overlay close on Escape key and close button.
6. Call `show_notification_overlay()` from the single-click gesture handler.

**Exit criteria:**

- Overlay window appears on single-click.
- Full notification content is displayed.
- Dismiss button works and closes the overlay.
- Action buttons work and close the overlay.
- Close button and Escape key close the overlay.

---

### Phase 5: CSS Styling

**Goal:** Add CSS classes for the tile and overlay.

**Order:**

1. Add `.notifications-widget`, `.notification-count`, `.notification-summary`, `.notification-app` classes to `resources/style.css`.
2. Add `.notification-overlay-*` classes to `resources/style.css`.
3. Verify styling is consistent with the project's visual language.

**Exit criteria:**

- Tile is visually compact (max 100px height).
- Overlay is visually distinct from the tile.
- Text is readable at the configured font sizes.

---

### Phase 6: Integration & Testing

**Goal:** Verify the widget works end-to-end with the existing service.

**Order:**

1. Update `config.toml` with the new widget configuration.
2. Build the widget crate and load it in the launcher.
3. Send test notifications via `notify-send` and verify:
    - Tile shows truncated summary.
    - Swipe up/down cycles through notifications.
    - Long-press dismisses a notification.
    - Single-click opens the overlay with full content.
    - Overlay dismiss button works.
4. Test edge cases: no notifications, single notification, DND active.
5. Verify the widget does not exceed 100px height.

**Exit criteria:**

- All interactions work correctly.
- Widget height does not exceed 100px.
- No panics or crashes during interaction.
- Overlay appears and disappears cleanly.

---

## 12. Dependencies

### 12.1 New Dependencies

| Crate              | Purpose                                 |
|--------------------|-----------------------------------------|
| `gtk4-layer-shell` | Layer Shell protocol for overlay window |

### 12.2 Existing Dependencies (unchanged)

| Crate                               | Purpose                              |
|-------------------------------------|--------------------------------------|
| `gtk4`                              | GTK4 widget toolkit                  |
| `glib`                              | GLib utilities and main context      |
| `smearor-notifications-model`       | Shared message types                 |
| `smearor-swipe-launcher-plugin-api` | Plugin API and traits                |
| `tokio`                             | Async runtime (channel for messages) |
| `tracing`                           | Logging                              |

---

## 13. Notes

- The `model/notifications` and `services/notifications` crates require **no changes** — all Milestone 2 work is confined to the widget crate.
- The overlay window is a separate `ApplicationWindow` that is created on-demand and destroyed when closed. It does not persist in the widget struct.
- The `gtk4-layer-shell` crate is already a workspace dependency used by the core launcher. The widget crate adds it as a direct dependency.
- The swipe threshold (50px default) matches the Weather Widget for consistency.
- The `max_summary_chars` default of 30 is chosen to fit a 120px-wide tile at 13px font size.
- The overlay uses `Layer::Overlay` and `exclusive_zone(0)` to appear above the launcher without pushing other windows.
- Action button invocation from the overlay requires the `MessageBroadcasterInner` to be passed into the overlay function, since the overlay is created outside
  the widget struct's lifetime.
