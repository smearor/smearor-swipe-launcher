# doa (Plugin)

Direction of Arrival widget for the ReSpeaker XVF3800 USB 4-Mic Array. Displays real-time voice direction detection with compass, direction, and device info
views.

## Description

The DoA widget communicates with the [DoA service](../services/doa.md) via the message broker. It subscribes to `DoaStatusMessage` updates and renders the
current direction, angle, and speech detection state. Clicking the widget cycles through three views: Compass, Direction, and DeviceInfo.

## Views

### Compass

Shows the calibrated angle in degrees and the mapped compass direction.

- **Icon:** `nf-md-compass`
- **Main text:** `{calibrated_angle}°`
- **Info text:** `Compass {Direction}` (localized)

### Direction

Shows the mapped compass direction with speech/silence indicator.

- **Icon:** Direction-specific (`nf-md-arrow_up`, `nf-md-arrow_right`, `nf-md-arrow_down`, `nf-md-arrow_left`)
- **Main text:** Direction label (localized)
- **Info text:** `Direction {Speech|Silence}` (localized)

### DeviceInfo

Shows USB device vendor/product IDs with speech activity icon.

- **Icon:** `nf-md-microphone` (speech) or `nf-md-chip` (silence)
- **Main text:** `Device` (localized)
- **Info text:** `VID:0x{vendor_id} PID:0x{product_id}`

## Configuration

```toml
[[plugins]]
id = "doa"
path = "target/release/libsmearor_doa_widget.so"
```

Widget appearance can be customized in the plugin config:

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

## Topics

| Topic                 | Direction        | Description                               |
|-----------------------|------------------|-------------------------------------------|
| `service.doa.status`  | Service → Widget | Current direction, angle, VAD, connection |
| `service.doa.command` | Widget → Service | Reconnect, pause, resume, set interval    |

## Locale Support

Labels are localized for English, German, French, Spanish, and Italian. Unsupported locales fall back to English.
