# streamdeck (Service)

Elgato Stream Deck driver service. Handles USB HID communication with all Stream Deck models.

## Description

The streamdeck service connects to Elgato Stream Deck devices via USB HID. It receives button press events and sends LCD key images. When a device connects, it
triggers loading of a headless instance; when it disconnects, the instance is stopped.

## Supported Models

Stream Deck Mini, Neo, MK.2, XL, Plus, Pedal, Studio. See [MacroPad Models](../reference/macropad-models.md) for specifications.

## Topics

| Topic                       | Direction          | Description                   |
|-----------------------------|--------------------|-------------------------------|
| `macropad.connection`       | Service → Host     | Device connected/disconnected |
| `macropad.input`            | Service → Instance | Button press/release events   |
| `macropad.set_button_image` | Instance → Service | Send RGBA pixels to a key     |

## Configuration

```toml
[[services]]
id = "streamdeck"
path = "target/release/libsmearor_streamdeck_service.so"
```

## Udev Rules

The project includes udev rules at `resources/udev/52-streamdeck.rules` for proper USB permissions.

## Crate

- **Path**: `services/streamdeck/`
- **Library**: `libsmearor_streamdeck_service.so`
- **Model**: `model/macropad/`
