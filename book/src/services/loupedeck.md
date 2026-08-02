# loupedeck (Service)

Loupedeck MacroPad driver service. Handles USB HID communication with Loupedeck devices (CT, Live, Live S, Razer Stream Controller).

## Description

The loupedeck service connects to Loupedeck devices via USB using the `loupedeck-driver` crate. It receives button press events and sends LCD key images. When a
device connects, it triggers loading of a headless instance; when it disconnects, the instance is stopped.

## Topics

| Topic                       | Direction          | Description                   |
|-----------------------------|--------------------|-------------------------------|
| `macropad.connection`       | Service → Host     | Device connected/disconnected |
| `macropad.input`            | Service → Instance | Button press/release events   |
| `macropad.set_button_image` | Instance → Service | Send RGBA pixels to a key     |

## Configuration

```toml
[[services]]
id = "loupedeck"
path = "target/release/libsmearor_loupedeck_service.so"
```

## Udev Rules

The project includes udev rules at `resources/udev/52-loupedeck.rules` for proper USB permissions.

## Crate

- **Path**: `services/loupedeck/`
- **Library**: `libsmearor_loupedeck_service.so`
- **Model**: `model/macropad/`
