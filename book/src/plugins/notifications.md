# notifications (Plugin)

Notification widget that displays a badge counter and slide-in banners for new notifications.

## Description

The notifications widget communicates with the [notifications service](../services/notifications.md) which listens on the `org.freedesktop.Notifications` DBus
interface. The widget shows a count of unread notifications and can display banners.

## Configuration

```toml
[notifications]
icon_size = 32
show_icons = true
```

| Field        | Type   | Description                        |
|--------------|--------|------------------------------------|
| `icon_size`  | `i32`  | Icon size in pixels                |
| `show_icons` | `bool` | Whether to show notification icons |

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Swipe-to-dismiss is supported for notification banners.

## Related Service

- [notifications (service)](../services/notifications.md) — DBus notification daemon listener

## Crate

- **Path**: `plugins/notifications/`
- **Library**: `libsmearor_notifications_widget.so`
- **Model**: `model/notifications/`
