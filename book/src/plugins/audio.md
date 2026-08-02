# audio (Plugin)

Audio volume control widget. Displays the current volume level and mute state, and allows adjusting the volume via click, scroll, or swipe gestures.

## Description

The audio widget communicates with the [audio service](../services/audio.md) via the message broker. It displays a dynamic icon that changes based on volume
level and mute state. Scroll up/down adjusts the volume; click toggles mute.

## Configuration

```toml
[audio]
icon_only = false
mode = "wide"
max_width = 200
max_width_chars = 18
```

| Field             | Type          | Description                                 |
|-------------------|---------------|---------------------------------------------|
| `icon_size`       | `i32`         | Icon size in pixels                         |
| `icon_only`       | `bool`        | Show only the icon                          |
| `mode`            | `WidgetMode`  | `compact` (vertical) or `wide` (horizontal) |
| `max_width`       | `Option<i32>` | Maximum widget width in pixels              |
| `max_width_chars` | `Option<i32>` | Maximum text width in characters            |

## Dynamic Icons

The icon changes based on state:

- Muted: mute icon
- Volume 0-33%: low volume icon
- Volume 34-66%: medium volume icon
- Volume 67-100%: high volume icon

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Default fallbacks:

- **Click**: Toggle mute
- **Scroll Up**: Volume up
- **Scroll Down**: Volume down

## Related Service

- [audio (service)](../services/audio.md) — PulseAudio integration, volume control, MCP tools

## Crate

- **Path**: `plugins/audio/`
- **Library**: `libsmearor_audio_widget.so`
- **Model**: `model/audio/`
