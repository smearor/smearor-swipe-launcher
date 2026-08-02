# mpris (Service)

MPRIS (Media Player Remote Interfacing Specification) service for controlling media players via D-Bus.

## Description

The MPRIS service connects to the D-Bus session bus and discovers media players that implement the MPRIS interface. It tracks playback state, metadata (title,
artist, album art), and provides control commands. It broadcasts status updates to all instances when playback state changes.

## Topics

| Topic                   | Direction         | Description                             |
|-------------------------|-------------------|-----------------------------------------|
| `service.mpris.command` | Widget → Service  | Play, pause, next, previous             |
| `service.mpris.status`  | Service → Widgets | Current track, playback state, metadata |

## MCP Tools

| Tool               | Description                 |
|--------------------|-----------------------------|
| `mpris_play`       | Start playback              |
| `mpris_pause`      | Pause playback              |
| `mpris_next`       | Next track                  |
| `mpris_previous`   | Previous track              |
| `mpris_get_status` | Get current playback status |

## Configuration

```toml
[[services]]
id = "mpris"
path = "target/release/libsmearor_mpris_service.so"
```

## Crate

- **Path**: `services/mpris/`
- **Library**: `libsmearor_mpris_service.so`
- **Model**: `model/mpris/`
