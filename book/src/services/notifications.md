# notifications (Service)

D-Bus notification daemon listener service that receives desktop notifications and forwards them to widgets.

## Description

The notifications service listens on the `org.freedesktop.Notifications` D-Bus interface and captures incoming notifications. It forwards them to
the [notifications widget](../plugins/notifications.md) for display as banners and badges.

## Topics

| Topic                           | Direction         | Description                          |
|---------------------------------|-------------------|--------------------------------------|
| `service.notifications.status`  | Service → Widgets | New notification, badge count update |
| `service.notifications.command` | Widget → Service  | Dismiss, clear all                   |

## Configuration

```toml
[[services]]
id = "notifications"
path = "target/release/libsmearor_notifications_service.so"
```

## Crate

- **Path**: `services/notifications/`
- **Library**: `libsmearor_notifications_service.so`
- **Model**: `model/notifications/`
