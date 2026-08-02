# Services Configuration

Services are configured in `configs/services/services.toml`. Unlike widget plugins, services are loaded once and shared across all launcher instances.

## Structure

```toml
# Using path= (development / from-source)
[[services]]
id = "audio"
path = "target/release/libsmearor_audio_service.so"

# Using name= (system installation / Debian packages)
[[services]]
id = "audio"
name = "audio_service"
```

## Service Entry

| Field      | Type             | Description                                                        |
|------------|------------------|--------------------------------------------------------------------|
| `id`       | `String`         | Unique service ID (used in topic routing)                          |
| `path`     | `Option<String>` | Path to the `.so` file (mutually exclusive with `name`)            |
| `name`     | `Option<String>` | Short name for library resolution (mutually exclusive with `path`) |
| `disabled` | `bool`           | Whether the service is disabled                                    |

### `path` vs `name`

Either `path` or `name` must be specified for each service entry:

- **`path`** — explicit file path to the `.so` file. Used in development configs.
- **`name`** — short name used for library resolution. The host searches for `libsmearor_<name>.so` in:
    1. `~/.local/lib/smearor/` (user-local)
    2. `/usr/lib/smearor/` (system-wide, e.g. Debian package installation)

## Service-Specific Configuration

Some services read additional configuration from the same file or from separate files:

```toml
# App launcher service config
[app_launcher]
smearor_wrot_path = "smearor-wrot"
rotation = 0

# Weather service config
[weather]
latitude = 52.52
longitude = 13.405
```

See individual [service pages](../services/audio.md) for service-specific configuration fields.

## Separate Service Config Files

Some services have their own config files:

- `configs/services/services.toml` — Main services list
- `configs/services/wallpaper.toml` — Wallpaper service config

### Config Discovery

The launcher discovers the services config in this fallback order:

1. CLI `--services-config` argument
2. `services.toml` in the working directory
3. `~/.config/smearor/services/services.toml` (user config)
4. `/usr/share/smearor/services/services.toml` (system default)

The wallpaper config (`wallpaper.toml`) is discovered similarly:

1. `wallpaper.toml` in the working directory
2. `~/.config/smearor/services/wallpaper.toml` (user config)
3. `/usr/share/smearor/services/wallpaper.toml` (system default)

On first run, the launcher copies default configs from `/usr/share/smearor/` to `~/.config/smearor/` if they don't already exist.

## Topic Routing

Service topics follow the pattern `service.{id}.{action}`:

- `service.audio.command` → routed to the `audio` service
- `service.hyprland.dispatch` → routed to the `hyprland` service
- `service.weather.command` → routed to the `weather` service

The `ServiceManager` matches the topic prefix to the service ID and routes the message accordingly.
