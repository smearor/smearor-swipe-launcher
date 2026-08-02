# mpris (Plugin)

Media player control widget. Shows album art, track title, and artist, with controls for play/pause, next, and previous.

## Description

The MPRIS widget communicates with the [MPRIS service](../services/mpris.md) which connects to active media players via DBus. The widget displays album art
(when available), track info, and provides touch-optimized controls.

## Configuration

```toml
[mpris]
icon_only = false
mode = "wide"
max_width = 200
max_width_chars = 18
```

| Field             | Type          | Description                      |
|-------------------|---------------|----------------------------------|
| `icon_size`       | `i32`         | Icon size in pixels              |
| `icon_only`       | `bool`        | Show only the icon               |
| `mode`            | `WidgetMode`  | `compact` or `wide`              |
| `max_width`       | `Option<i32>` | Maximum widget width             |
| `max_width_chars` | `Option<i32>` | Maximum text width in characters |

## Dynamic Icons

- No player: idle icon
- Playing: play icon
- Paused: pause icon

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Default fallbacks:

- **Click**: Play/pause toggle
- **Scroll Up**: Next track
- **Scroll Down**: Previous track

## Related Service

- [mpris (service)](../services/mpris.md) — DBus MPRIS integration, player tracking, MCP tools

## Crate

- **Path**: `plugins/mpris/`
- **Library**: `libsmearor_mpris_widget.so`
- **Model**: `model/mpris/`
