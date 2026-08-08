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

# DoA service config
[doa]
poll_interval_ms = 150
mcp_enabled = true
product_id = 0x0021
reconnect_delay_ms = 1000
rotation_offset = 0
ceiling_mode = false
```

See individual [service pages](../services/audio.md) for service-specific configuration fields.

## MCP Server Configuration

The `[mcp]` section configures the built-in MCP (Model Context Protocol) server:

```toml
[mcp]
bind_address = "127.0.0.1"
port = 8765
# auth_token = "my-secret-token"
# log_buffer_enabled = true
# log_buffer_capacity = 10000
```

| Field                 | Type             | Default     | Description                                                                                        |
|-----------------------|------------------|-------------|----------------------------------------------------------------------------------------------------|
| `bind_address`        | `String`         | `127.0.0.1` | Address to bind the HTTP server to. Use `0.0.0.0` for network access.                              |
| `port`                | `u16`            | `8765`      | TCP port to listen on.                                                                             |
| `auth_token`          | `Option<String>` | `None`      | Optional bearer token for authentication.                                                          |
| `log_buffer_enabled`  | `bool`           | `true`      | Whether the tracing log buffer and `launcher_get_logs` tool are enabled.                           |
| `log_buffer_capacity` | `usize`          | `10000`     | Maximum number of log entries in the ring buffer. Set to `0` to disable. ~2MB at default capacity. |

### Log Buffer Disable

Log capture can be disabled in two ways (both have the same effect — no `LogBufferLayer` is installed, zero overhead):

1. `log_buffer_enabled = false`
2. `log_buffer_capacity = 0`

When disabled, the `launcher_get_logs` MCP tool returns an error.

See [MCP Server and AI Integration](../features/mcp-server.md) for the feature overview.

## Web Server Configuration

The `[web]` section configures the embedded web server for browser-based launcher instances:

```toml
[web]
enabled = true
# bind_address = "127.0.0.1"
# port = 8080
# allowed_origins = []
```

| Field             | Type             | Default     | Description                                                       |
|-------------------|------------------|-------------|-------------------------------------------------------------------|
| `enabled`         | `bool`           | `false`     | Whether the web server is enabled.                                |
| `bind_address`    | `String`         | `127.0.0.1` | Address to bind to. Use `0.0.0.0` for network access.             |
| `port`            | `u16`            | `8080`      | TCP port to listen on.                                            |
| `auth_token`      | `Option<String>` | `None`      | Optional bearer token for authentication.                         |
| `allowed_origins` | `Vec<String>`    | `[]`        | Allowed CORS origins. Use `["*"]` to allow all (not recommended). |

See [Web Interface](../features/web-interface.md) for the feature overview.

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
- `service.doa.command` → routed to the `doa` service
- `service.hyprland.dispatch` → routed to the `hyprland` service
- `service.weather.command` → routed to the `weather` service

The `ServiceManager` matches the topic prefix to the service ID and routes the message accordingly.
