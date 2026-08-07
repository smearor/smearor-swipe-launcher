# DoA — Direction of Arrival

The launcher includes a **Direction of Arrival (DoA)** feature that integrates with the ReSpeaker XVF3800 USB 4-Mic Array to provide real-time voice direction
detection, hardware VAD (Voice Activity Detection), and cross-service coordination with the Voice Assistant and Audio services.

## Overview

The DoA feature consists of three crates:

- **`model/doa`** — Shared types: `DoaDirection`, `DoaStatusMessage`, `DoaCommandMessage`, `DoaView`, `DoaDirectionResponse`, VAD transition logic
- **`services/doa`** — Service plugin: USB communication, async control loop, MCP tools, status broadcasting
- **`plugins/doa`** — Widget plugin: Compass, Direction, and DeviceInfo views with locale-aware labels

## Widget Views

The DoA widget cycles through three views on click:

### Compass View

Displays the calibrated angle in degrees and the mapped compass direction.

- **Icon:** `nf-md-compass`
- **Main text:** `{calibrated_angle}°`
- **Info text:** `Compass {Direction}` (localized)

### Direction View

Shows the mapped compass direction with speech/silence indicator.

- **Icon:** Direction-specific (`nf-md-arrow_up`, `nf-md-arrow_right`, `nf-md-arrow_down`, `nf-md-arrow_left`)
- **Main text:** Direction label (localized)
- **Info text:** `Direction {Speech|Silence}` (localized)

### DeviceInfo View

Shows USB device vendor/product IDs with speech activity icon.

- **Icon:** `nf-md-microphone` (speech) or `nf-md-chip` (silence)
- **Main text:** `Device` (localized)
- **Info text:** `VID:0x{vendor_id} PID:0x{product_id}`

## Configuration

### Widget Configuration

Configured in `configs/launcher/config.toml` within the `plugins` array:

```toml
[[plugins]]
id = "doa"
path = "target/release/libsmearor_doa_widget.so"
```

| Field                  | Type        | Default                            | Description                                 |
|------------------------|-------------|------------------------------------|---------------------------------------------|
| `icon_compass`         | `String`    | `nf-md-compass`                    | Icon for compass view                       |
| `icon_direction_north` | `String`    | `nf-md-arrow_up`                   | Icon for north direction                    |
| `icon_direction_east`  | `String`    | `nf-md-arrow_right`                | Icon for east direction                     |
| `icon_direction_south` | `String`    | `nf-md-arrow_down`                 | Icon for south direction                    |
| `icon_direction_west`  | `String`    | `nf-md-arrow_left`                 | Icon for west direction                     |
| `icon_disconnected`    | `String`    | `nf-md-connection`                 | Icon when device is disconnected or paused  |
| `icon_device`          | `String`    | `nf-md-chip`                       | Icon for device info view (silence)         |
| `icon_speech`          | `String`    | `nf-md-microphone`                 | Icon for device info view (speech detected) |
| `views`                | `[DoaView]` | `[Compass, Direction, DeviceInfo]` | Ordered list of views to cycle              |

### Service Configuration

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

## Direction Mapping

Angles are mapped to four compass directions:

| Direction | Angle Range |
|-----------|-------------|
| North     | 315° – 45°  |
| East      | 45° – 135°  |
| South     | 135° – 225° |
| West      | 225° – 315° |

The `rotation_offset` is applied after optional ceiling-mode mirroring, allowing the calibrated angle to align with the physical table orientation.

## Locale Support

All widget labels are localized for:

- English (en-US)
- German (de-DE)
- French (fr-FR)
- Spanish (es-ES)
- Italian (it-IT)

Unsupported locales fall back to English.

## udev Rules

USB access requires udev rules for non-root access. The rules file `52-respeaker.rules` is installed at `/usr/lib/udev/rules.d/`:

```
# ReSpeaker XVF3800 USB 4-Mic Array (Seeed Studio)
SUBSYSTEM=="usb", ATTR{idVendor}=="2886", ATTR{idProduct}=="0021", TAG+="uaccess", MODE="0666"

# XMOS vendor ID fallback
SUBSYSTEM=="usb", ATTR{idVendor}=="20b1", TAG+="uaccess", MODE="0666"
```

After installation, reload udev rules:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Cross-Service Integration

### Voice Assistant — VAD-Triggered Listening

When `doa_vad.enabled` is set in the Voice Assistant configuration, the Voice Assistant uses the hardware VAD flag from `DoaStatusMessage` to activate and
deactivate the listening pipeline with near-zero latency:

- **Rising edge** (speech starts): Records onset timestamp
- **Continuous speech** (min_speech_duration_ms elapsed): Activates listening mode
- **Falling edge** (speech stops): Schedules grace period exit before deactivating

The TTS-Mute-Window prevents self-triggering from TTS output when AEC mirroring is not configured.

### Audio Service — VAD-Triggered Ducking

When `ducking_enabled` is set in the Audio service configuration, the Audio service uses the VAD flag to duck (lower) the system volume during speech and
restore it after a grace period:

- **Rising edge**: Records onset timestamp
- **Continuous speech** (min_speech_duration_ms elapsed): Ducks volume to `ducking_volume`
- **Falling edge**: Schedules grace period restore with fade ramp

See [Voice Assistant DoA Integration](../architecture/doa.md#voice-assistant-integration) for architecture details.
