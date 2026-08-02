# audio (Service)

PulseAudio integration service for volume control, mute toggling, and sink management.

## Description

The audio service connects to PulseAudio via `libpulse-binding` and provides volume control commands to the [audio widget](../plugins/audio.md). It tracks the
default sink and broadcasts status updates when volume or mute state changes.

## Topics

| Topic                   | Direction         | Description                   |
|-------------------------|-------------------|-------------------------------|
| `service.audio.command` | Widget → Service  | Volume up/down, toggle mute   |
| `service.audio.status`  | Service → Widgets | Current volume and mute state |

## MCP Tools

| Tool                | Description                    |
|---------------------|--------------------------------|
| `audio_volume_up`   | Increase volume by a step      |
| `audio_volume_down` | Decrease volume by a step      |
| `audio_toggle_mute` | Toggle mute state              |
| `audio_set_volume`  | Set volume to a specific level |

## Configuration

Configured in `configs/services/services.toml`:

```toml
[[services]]
id = "audio"
path = "target/release/libsmearor_audio_service.so"
```

## Crate

- **Path**: `services/audio/`
- **Library**: `libsmearor_audio_service.so`
- **Model**: `model/audio/`
