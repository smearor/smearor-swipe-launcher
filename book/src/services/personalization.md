# personalization (Service)

System personalization service that reads GNOME/desktop settings for adaptive theming.

## Description

The personalization service reads desktop settings (color scheme, accent color, font settings) via the XDG Desktop Portal (`ashpd`) and GNOME settings. It
provides this information to widgets for adaptive theming — for example, adjusting icon colors based on the system accent color.

## Topics

| Topic                             | Direction         | Description                           |
|-----------------------------------|-------------------|---------------------------------------|
| `service.personalization.status`  | Service → Widgets | Theme settings (color scheme, accent) |
| `service.personalization.command` | Widget → Service  | Query current settings                |

## Configuration

```toml
[[services]]
id = "personalization"
path = "target/release/libsmearor_personalization_service.so"
```

## Crate

- **Path**: `services/personalization/`
- **Library**: `libsmearor_personalization_service.so`
- **Model**: `model/personalization/`
