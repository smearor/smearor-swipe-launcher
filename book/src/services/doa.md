# doa (Service)

USB integration service for the ReSpeaker XVF3800 4-Mic Array, providing real-time direction-of-arrival detection and hardware VAD.

## Description

The DoA service connects to the ReSpeaker XVF3800 USB device via `rusb` and performs periodic control transfers to read the current angle (0–359°) and VAD flag.
It broadcasts `DoaStatusMessage` updates to all subscribers and accepts commands for reconnection, pause/resume, and poll interval changes.

A dedicated OS thread handles blocking USB reads, while an async control loop on the Tokio runtime processes commands, applies calibration, and broadcasts
status updates.

See [DoA Architecture](../architecture/doa.md) for internal details.

## Topics

| Topic                 | Direction         | Description                                       |
|-----------------------|-------------------|---------------------------------------------------|
| `service.doa.command` | Widget → Service  | Reconnect, pause, resume, set poll interval       |
| `service.doa.status`  | Service → Widgets | Current direction, angle, VAD, connection, paused |

## MCP Tools

| Tool                    | Arguments | Description                                     |
|-------------------------|-----------|-------------------------------------------------|
| `doa_get_direction`     | None      | Returns current direction, angle, and VAD state |
| `doa_set_poll_interval` | `ms: u64` | Changes the USB poll interval (minimum 50 ms)   |
| `doa_reconnect`         | None      | Forces USB device reconnection                  |

## MCP Resources

| Resource       | Description                                         |
|----------------|-----------------------------------------------------|
| `doa://status` | Current DoA status as JSON (`DoaDirectionResponse`) |

## Configuration

Configured in `configs/services/services.toml`:

```toml
[[services]]
id = "doa"
path = "target/release/libsmearor_doa_service.so"

[doa]
poll_interval_ms = 150
mcp_enabled = true
product_id = 0x0021
reconnect_delay_ms = 1000
rotation_offset = 0
ceiling_mode = false
```

| Field                | Type          | Default | Description                                         |
|----------------------|---------------|---------|-----------------------------------------------------|
| `poll_interval_ms`   | `u64`         | `150`   | USB poll interval in milliseconds (minimum 50)      |
| `mcp_enabled`        | `bool`        | `true`  | Whether MCP tools and resources are registered      |
| `product_id`         | `Option<u16>` | `None`  | Filter by product ID (e.g. `0x0021` for XVF3800)    |
| `reconnect_delay_ms` | `u64`         | `1000`  | Delay between reconnection attempts in milliseconds |
| `rotation_offset`    | `i16`         | `0`     | Calibration offset in degrees (±360)                |
| `ceiling_mode`       | `bool`        | `false` | Mirror angle for ceiling-mounted installation       |

## udev Rules

USB access requires udev rules for non-root access. See [DoA Feature — udev Rules](../features/doa.md#udev-rules).
