# MCP Server Concept for Smearor Swipe Launcher

This document describes the concept for an **MCP Server (Model Context Protocol)** that exposes the *Smearor Swipe Launcher* through a standardized interface
for external AI clients. The MCP server runs as an integrated part of the launcher application and communicates with MCP clients via **Streamable HTTP** and *
*SSE (
Server-Sent Events)** using the `rust-mcp-sdk` and `rust-mcp-axum` crates.

---

## 1. Goal & Motivation

The *Smearor Swipe Launcher* consists of a central event broker, Areas (widgets as windows/popups), services, and widgets. Currently, controlling and querying
these components is limited to the internal application. An MCP server enables AI assistants and external tools to control the launcher directly and query
system state without needing to know proprietary interfaces.

**Benefits:**

* **Standardized AI integration:** Any MCP client (e.g., Claude, Cursor, etc.) can control the launcher.
* **Area automation:** AI clients can open and close launcher areas specifically (e.g., "Open the audio menu").
* **Broker control:** Messages can be sent to topics to trigger widgets and services.
* **Plugin tools:** Services can provide semantically specific tools (e.g., change volume) directly through the MCP server.
* **State queries:** System values such as uptime, volume, or media player status can be queried as resources.

---

## 2. Architecture

The MCP server is implemented as a separate crate `mcp-server` in the workspace. It is always started automatically when the launcher starts and runs as a
dedicated tokio task inside the launcher process. The transport uses `rust-mcp-axum`'s `create_axum_server` with both **Streamable HTTP** and **SSE** support
(`sse_support: true`, `enable_json_response: Some(true)`).

**Process model:** The MCP server runs as a dedicated task inside the launcher process. No separate process is required.

**Permissions:** An optional bearer token can be configured via `McpServerConfig::auth_token`. When set, all endpoints require an
`Authorization: Bearer <token>`
header. Without a token, the server only binds to the configured address (default `127.0.0.1:8765`).

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP CLIENT                              │
│  (e.g., Claude Desktop, Cursor, VS Code Extension)          │
└─────────────────────┬───────────────────────┬───────────────┘
                      │ JSON-RPC / MCP over Streamable HTTP + SSE │
                      ▼                         ▼
┌──────────────────────────────────────────┐  ┌────────────────────────┐
│   MCP Server (rust-mcp-sdk + axum)       │  │  Resource/Tool Registry │
│  ┌────────────────────────────────────┐  │  │  (AreaManager +        │
│  │   Tools: open_area, close_area,    │  │  │   Plugin handlers)     │
│  │   send_message, plugin tools, ...  │  │  └────────────────────────┘
│  └────────────────────────────────────┘  │
│  ┌────────────────────────────────────┐  │
│  │   Notifications: list_changed      │  │
│  └────────────────────────────────────┘  │
└─────────────────────┬────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                 Smearor Swipe Launcher Core                  │
│  ┌─────────────────────┐    ┌─────────────────────────────┐│
│  │   Area Manager      │    │   Central Message Broker      ││
│  │   (open/close/     │    │   (publish/subscribe)         ││
│  │    list areas)      │    │                               ││
│  └─────────────────────┘    └─────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Proposed Tools

Tools are functions callable by the MCP client that trigger actions in the launcher.

### 3.1 Mandatory Tools (from the MVP)

| Tool                  | Description                                                                              | Parameters                                                      |
|-----------------------|------------------------------------------------------------------------------------------|-----------------------------------------------------------------|
| `open_area`           | Opens a defined area by its ID.                                                          | `area_id: string`                                               |
| `close_area`          | Closes a currently visible area.                                                         | `area_id: string`                                               |
| `list_areas`          | Lists all currently managed (opened) areas with ID, position, and visibility.            | –                                                               |
| `list_all_areas`      | Lists all configured areas (including not-yet-opened ones) with their area IDs.          | –                                                               |
| `focus_area`          | Sets focus on an area (e.g., for keyboard navigation).                                   | `area_id: string`                                               |
| `send_message`        | Publishes a message to a topic on the central broker.                                    | `topic: string`, `payload: json`, `target_instance_id?: string` |
| `toggle_area`         | Toggles the visibility of an area.                                                       | `area_id: string`                                               |
| `get_area_config`     | Returns the configuration of an area as JSON.                                            | `area_id: string`                                               |
| `open_transient_area` | Opens an area as a transient overlay on top of a source area (simulates a button click). | `area_id: string`, `source_area_id?: string`                    |

### 3.2 Additional Useful Tools

| Tool               | Description                                                                  | Parameters                                            |
|--------------------|------------------------------------------------------------------------------|-------------------------------------------------------|
| `reload_config`    | Reloads the configuration files (`config.toml`, `services.toml`).            | –                                                     |
| `send_action`      | Sends a typed action to a widget or service (e.g., `AppLaunch`, `VolumeUp`). | `plugin_id: string`, `action: string`, `params: json` |
| `trigger_widget`   | Triggers the primary interaction event of a widget (e.g., tap).              | `widget_id: string`, `event: string`                  |
| `set_global_theme` | Switches the global CSS theme (e.g., light/dark).                            | `theme: string`                                       |
| `play_sound`       | Plays a configured system sound.                                             | `sound_id: string`                                    |

### 3.3 Plugin-Provided Tools

In addition to generic core tools, service and widget plugins can register their own, semantically typed tools via the **Plugin-Tool-Registry**. The MCP server
queries
this registry and exposes the tools dynamically.

**Naming convention:** Implemented plugins use flat, service-prefixed names (e.g., `system_power_action`, `network_toggle_radio`) rather than dotted
`plugin.<service>.<action>` names. The naming convention is not enforced by the registry — each plugin chooses its tool names.

#### Implemented Plugin Tools

| Plugin               | Tool                               | Description                                                            | Parameters                                                                                      |
|----------------------|------------------------------------|------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------|
| `services/power`     | `system_power_action`              | Executes the desired power action immediately.                         | `action: enum [shutdown, reboot, suspend, hibernate, lock, logout]`                             |
| `services/power`     | `system_schedule_power_action`     | Schedules a shutdown or reboot in the future.                          | `action: enum [shutdown, reboot]`, `delay_minutes: integer`                                     |
| `services/power`     | `system_cancel_power_action`       | Cancels a running shutdown timer or scheduled action.                  | –                                                                                               |
| `services/power`     | `system_reboot_to_uefi`            | Sets the firmware reboot flag and reboots directly into BIOS/UEFI.     | –                                                                                               |
| `services/network`   | `network_toggle_radio`             | Toggles WLAN or airplane mode on/off.                                  | `technology: enum [wifi, wwan, all]`, `enabled: boolean`                                        |
| `services/network`   | `network_connect_wifi`             | Connects the system to a specific access point.                        | `ssid: string`, `password?: string`                                                             |
| `services/network`   | `network_toggle_vpn`               | Starts or stops a specific VPN connection.                             | `profile_name: string`, `active: boolean`                                                       |
| `services/network`   | `network_get_public_ip`            | Queries the external IP address and provider (GeoIP) via HTTP service. | –                                                                                               |
| `services/wallpaper` | `add_wallpaper_theme`              | Permanently appends a new wallpaper theme to the configuration store.  | `name: string`, `type: enum [Video, Image, Application]`, `config: object`, `description?`, ... |
| `services/wallpaper` | `remove_wallpaper_theme`           | Deletes a wallpaper theme from the configuration store.                | `name: string`                                                                                  |
| `services/wallpaper` | `select_wallpaper_theme`           | Selects a wallpaper theme by name without starting it.                 | `name: string`                                                                                  |
| `services/wallpaper` | `start_selected_wallpaper_process` | Starts the currently selected wallpaper theme.                         | –                                                                                               |
| `services/wallpaper` | `stop_current_wallpaper_process`   | Stops the currently running wallpaper process immediately.             | –                                                                                               |
| `services/sysinfo`   | `sysinfo_refresh`                  | Force an immediate refresh of all sysinfo metrics.                     | –                                                                                               |
| `services/weather`   | `weather_refresh`                  | Force an immediate refresh of weather data.                            | –                                                                                               |
| `services/weather`   | `weather_get_forecast`             | Get weather forecast for arbitrary coordinates.                        | `latitude: number`, `longitude: number`                                                         |
| `plugins/clock`      | `get_current_time`                 | Returns the current local time as a formatted string.                  | –                                                                                               |
| `plugins/weather`    | `weather_widget_refresh`           | Force the weather widget to request a data refresh.                    | –                                                                                               |

#### Proposed Plugin Tools (not yet implemented)

**Example: Audio Service (based on `PulseCommand`):**

| Tool                    | Description                            | Parameters                |
|-------------------------|----------------------------------------|---------------------------|
| `audio_volume_up`       | Increases volume by a configured step. | –                         |
| `audio_volume_down`     | Decreases volume by a configured step. | –                         |
| `audio_set_volume`      | Sets the volume to an absolute value.  | `volume: f32` (0.0 – 1.0) |
| `audio_toggle_mute`     | Toggles the mute state.                | –                         |
| `audio_mute`            | Enables mute.                          | –                         |
| `audio_unmute`          | Disables mute.                         | –                         |
| `audio_next_device`     | Selects the next audio device.         | –                         |
| `audio_previous_device` | Selects the previous audio device.     | –                         |
| `audio_refresh_status`  | Manually re-reads the status.          | –                         |

Other proposed plugins (e.g., `mpris`, `hyprland`) would register their own tools analogously, e.g., `plugin.mpris.play_pause`,
`plugin.hyprland.switch_workspace`.

---

## 4. Proposed Resources

Resources are values queryable by the MCP client that reflect the current state of the launcher or the system.

### 4.1 Mandatory Resources (from the MVP)

The `AreaManager` and **every service plugin** must register at least their central state resources via the Plugin-Resource-Registry. The MCP server then offers
these resources dynamically.

#### Core Resources (AreaManager)

| URI                      | Description                                            | Format | Status    |
|--------------------------|--------------------------------------------------------|--------|-----------|
| `area://list`            | List of all configured areas with status and position. | JSON   | ✅         |
| `area://<area_id>/state` | Current state of an area (open, focused, visible).     | JSON   | ⏳ Planned |
| `area://current/focus`   | Currently focused area.                                | JSON   | ⏳ Planned |
| `area://current/visible` | Currently visible area(s).                             | JSON   | ⏳ Planned |

#### Implemented Service Plugin Resources

| Service              | URI                         | Description                                                                            | Status |
|----------------------|-----------------------------|----------------------------------------------------------------------------------------|--------|
| `services/sysinfo`   | `sysinfo://cpu`             | CPU usage and temperature.                                                             | ✅      |
| `services/sysinfo`   | `sysinfo://memory`          | RAM usage, total, used, available.                                                     | ✅      |
| `services/sysinfo`   | `sysinfo://disk`            | Mount point usage, read/write throughput.                                              | ✅      |
| `services/sysinfo`   | `sysinfo://temperature`     | CPU and GPU temperature readings.                                                      | ✅      |
| `services/sysinfo`   | `sysinfo://network`         | Incoming/outgoing network throughput.                                                  | ✅      |
| `services/sysinfo`   | `sysinfo://uptime`          | Uptime in seconds and load average.                                                    | ✅      |
| `services/power`     | `power://capabilities`      | System power capabilities as reported by systemd-logind.                               | ✅      |
| `services/power`     | `power://inhibitors`        | List of active inhibitor locks blocking power actions.                                 | ✅      |
| `services/power`     | `power://scheduled_actions` | Currently scheduled power action, if any.                                              | ✅      |
| `services/network`   | `network://status`          | Current network status including primary interface, SSID, signal, IP, and radio state. | ✅      |
| `services/network`   | `network://scan-results`    | List of all WLAN access points in range, including signal strength and encryption.     | ✅      |
| `services/network`   | `network://vpn-profiles`    | List of all VPN connections registered in NetworkManager and their current state.      | ✅      |
| `services/wallpaper` | `wallpaper://status`        | Current wallpaper service status including running theme and configured themes.        | ✅      |
| `services/wallpaper` | `wallpaper://themes`        | List of all configured wallpaper themes with their configurations.                     | ✅      |
| `services/weather`   | `weather://current`         | Current weather conditions for the configured location.                                | ✅      |
| `plugins/clock`      | `clock://time`              | Current time formatted by the clock widget.                                            | ✅      |
| `plugins/weather`    | `weather://widget`          | Current weather data displayed by the weather widget.                                  | ✅      |

#### Proposed Service Plugin Resources (not yet implemented)

| Service         | URI                                  | Description                                                   | Source type                      |
|-----------------|--------------------------------------|---------------------------------------------------------------|----------------------------------|
| `app_launcher`  | `plugin://app_launcher/running_apps` | Status of all monitored `.desktop` files (running / stopped). | `DesktopFileStatusMessageStabby` |
| `audio`         | `plugin://audio/status`              | Complete audio status (volume, mute, devices, active device). | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/volume`              | Current volume (0.0 – 1.0).                                   | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/muted`               | Current mute status.                                          | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/active_sink`         | Active output device with name, index, and channels.          | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/sinks`               | List of all available output devices.                         | `AudioStatusMessage`             |
| `mpris`         | `mpris://status`                     | Active players, playback status, metadata, position, volume.  | `MprisStatusMessage`             |
| `notifications` | `plugin://notifications/status`      | Do-not-disturb, active notifications, unread count.           | `NotificationStatusMessage`      |
| `hyprland`      | `plugin://hyprland/active_workspace` | Current workspace and window list (to be implemented).        | Custom status message            |
| `http`          | `plugin://http/stats`                | Last request statistics or last response (to be implemented). | Custom status message            |

### 4.2 Additional Useful Resources

| URI                   | Description                           | Format    | Status    |
|-----------------------|---------------------------------------|-----------|-----------|
| `launcher://config`   | Entire active launcher configuration. | JSON/TOML | ⏳ Planned |
| `launcher://version`  | Version of the launcher application.  | JSON      | ⏳ Planned |
| `bluetooth://devices` | Paired Bluetooth devices.             | JSON      | ⏳ Planned |

`launcher://config` is provided unfiltered during discovery. Security filters are not enforced in the MVP.

---

## 5. Notifications

The server pushes JSON-RPC notifications over the SSE stream. The following notifications are implemented:

| Notification                           | Trigger                                  | Payload             |
|----------------------------------------|------------------------------------------|---------------------|
| `notifications/tools/list_changed`     | A plugin registers a new tool.           | `{}`                |
| `notifications/resources/list_changed` | A plugin registers a new resource.       | `{}`                |
| `notifications/resources/updated`      | A resource is read and may have changed. | `{ "uri": string }` |

The server advertises `resources.subscribe: true` in its capabilities.

**Session-based responses:** Each SSE connection receives a unique `sessionId` in the `endpoint` event (`/message?sessionId=<id>`). JSON-RPC responses to
requests sent on that connection are routed back to the matching session only, so multiple concurrent clients do not interfere. Server-initiated notifications
remain broadcast to all connected SSE clients.

---

## 6. Transport Robustness

To prevent MCP clients from hanging or entering cascading failures, the SSE transport implements the following safeguards:

### 6.1 SSE Headers and Keep-Alive

The SSE endpoint returns headers that prevent buffering and keep the connection alive:

- `Content-Type: text/event-stream`
- `Cache-Control: no-store`
- `Connection: keep-alive`
- `X-Accel-Buffering: no`
- Periodic SSE comments and `KeepAlive` frames keep the HTTP connection open.

### 6.2 Timeouts

Every tool and resource call that waits for the launcher core uses a **10-second timeout** (`tokio::time::timeout(Duration::from_secs(10))`). If the launcher
core does not respond in time, the server returns
a JSON-RPC error with `JSONRPC_INTERNAL_ERROR` and a clear timeout message, instead of leaving the client waiting indefinitely.

### 6.3 Error Handling

Errors are caught at the transport and handler level:

- Parse errors are returned as JSON-RPC errors with code `-32700`.
- Unknown methods return `-32601`.
- Tool/resource timeouts and launcher-core failures return `-32603` with a descriptive message.
- The SSE stream stays open even when individual requests fail, so one bad request does not disconnect the client.

---

## 7. Tool Implementation Details (MVP)

### 7.1 `open_area`

```json
{
  "name": "open_area",
  "description": "Opens a Smearor area by its configured ID.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Internally, the same function is called as during a swipe event or hotkey: `AreaManager::open(area_id)`.

### 7.2 `close_area`

```json
{
  "name": "close_area",
  "description": "Closes a currently visible Smearor area.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Internally: `AreaManager::close(area_id)`.

### 7.3 `list_areas`

```json
{
  "name": "list_areas",
  "description": "Lists all configured Smearor areas with their current visibility and position.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

Internally: `AreaManager::list()` returns the configured areas with their current state.

### 7.4 `focus_area`

```json
{
  "name": "focus_area",
  "description": "Focuses a Smearor area for keyboard navigation.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Internally: `AreaManager::focus(area_id)`.

### 7.5 `send_message`

```json
{
  "name": "send_message",
  "description": "Publishes a message to a topic on the central message broker.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "topic": {
        "type": "string",
        "description": "Broker topic name"
      },
      "payload": {
        "type": "object",
        "description": "JSON payload to publish"
      },
      "target_instance_id": {
        "type": "string",
        "description": "Optional target widget/service instance ID"
      }
    },
    "required": [
      "topic",
      "payload"
    ]
  }
}
```

`send_message` accepts **strictly standard JSON** (`serde_json`). Binary or stabby layouts are not accepted directly from the client to maintain compatibility
with the MCP ecosystem. Inside the MCP server, the JSON payload is converted into the appropriate internal ABI-stable type using the host's JSON converter
registry and then published via `MessageBrokerHandle::send`.

### 7.6 `toggle_area`

```json
{
  "name": "toggle_area",
  "description": "Toggles the visibility of a Smearor area.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Internally: toggles `AreaManager::open` / `AreaManager::close` depending on current visibility.

### 7.7 `get_area_config`

```json
{
  "name": "get_area_config",
  "description": "Returns the configuration of a Smearor area as JSON.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Internally: serializes the area's TOML configuration as JSON.

### 7.8 `list_all_areas`

```json
{
  "name": "list_all_areas",
  "description": "Lists all configured Smearor areas (including not-yet-opened ones) with their area IDs.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

Internally: returns all configured area IDs from the config, regardless of whether they have been opened yet.

### 7.9 `open_transient_area`

```json
{
  "name": "open_transient_area",
  "description": "Opens a Smearor area as a transient overlay on top of a source area (simulates a button click).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      },
      "source_area_id": {
        "type": "string",
        "description": "ID of the managed area to use as source for the overlay. Defaults to the first scroll area."
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Internally: opens the area as a transient child overlay of the source area, mimicking the behavior of a button click that opens a sub-menu.

---

## 8. Resource Implementation Details (MVP)

### 8.1 Area Status and Current Area

The resource `area://list` is read directly from the `AreaManager` and is the only core resource currently implemented. It is registered as a built-in
resource in `mcp-server/src/resources.rs`. An area status entry contains at least:

```json
{
  "area_id": "audio",
  "visible": true,
  "focused": false,
  "position": "bottom",
  "active": true
}
```

The resources `area://<area_id>/state`, `area://current/focus`, and `area://current/visible` are proposed but not yet implemented.

### 8.2 Service Plugin Resources

Each service plugin keeps its current state and registers the corresponding resources via `RegisterResourceMessage`. Example `sysinfo://cpu`:

```
URI: sysinfo://cpu
MIME type: application/json
Body: { "cpu_usage": 12.5, "cpu_temperature": 45.2 }
```

Example `power://capabilities`:

```
URI: power://capabilities
MIME type: application/json
Body: { "can_power_off": true, "can_reboot": true, "can_suspend": true, "can_hibernate": false }
```

Example `network://status`:

```
URI: network://status
MIME type: application/json
Body: { "primary_interface": "wlan0", "ssid": "MyNetwork", "signal": 75, "ip": "192.168.1.42", "radio_enabled": true }
```

### 8.3 Generic Resources from Plugins

To enable future plugins to provide resources independently without the MCP server knowing them explicitly, a **Plugin-Resource-Registry** is introduced in the
core:

```
┌─────────────────────────────────────────┐
│            MCP Server                   │
│   list_resources() / read_resource()    │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│      Plugin Resource Registry           │
│  (global registry in the core)          │
│                                         │
│  sysinfo://cpu     -> PluginHandler │
│  power://status    -> PluginHandler │
│  clock://time      -> PluginHandler │
└─────────────────────────────────────────┘
```

**Mechanism (implemented):**

1. Every plugin (widget or service) registers resources during initialization by broadcasting a `RegisterResourceMessage` to the topic `mcp.register.resource`.
   The message contains:
    - `uri: stabby::string::String` (e.g., `sysinfo://cpu`)
    - `name: stabby::string::String` (display name)
    - `description: stabby::string::String` (human-readable description)
    - `mime_type: stabby::string::String` (e.g., `application/json`)
2. The host's `route_message` function in `application.rs` intercepts `TOPIC_MCP_REGISTER_RESOURCE` messages and inserts them into the shared `McpRegistry`.
3. The MCP server queries the `McpRegistry` at list-time and dynamically exposes all registered URIs to the MCP client.
4. For `read_resource(<uri>)`, the host sends an `InvokeResourceMessage` to the owning plugin via `TOPIC_MCP_INVOKE_RESOURCE` with a correlation ID.
5. The plugin handles `InvokeResourceMessage`, reads its internal state, and responds with an `InvokeResourceResponse` (containing JSON as a string) broadcast
   to
   `TOPIC_MCP_RESOURCE_RESPONSE`.
6. The host's `McpResponseTracker` resolves the pending response by correlation ID and returns it to the MCP server.

Thus, even plugins developed later can provide resources without requiring changes to the MCP server or the core model. For stabby-FFI types, the plugin
converts internally via the JSON converter registry.

**No last-value cache for MCP:** Because the `AreaManager` and all service plugins expose their state explicitly as resources, the MCP interface **does not need
a `topic://<topic>/last` resource**. The MCP server reads exclusively through registered resource handlers.

The message broker may still maintain an internal last-value cache for widgets/services (late-subscriber initialization), but this cache is not visible to the
MCP server and is not exposed as a resource.

### 8.4 Plugin-Tool-Registry

Analogous to the Plugin-Resource-Registry, there is a **Plugin-Tool-Registry** through which service plugins register their own MCP tools. The MCP server
dynamically extends its tool list without having to implement tool handlers manually for each plugin.

**Registration per plugin (implemented):**

Plugins register tools by broadcasting a `RegisterToolMessage` to the topic `mcp.register.tool`. The message contains:

- `name: stabby::string::String` (e.g., `system_power_action`)
- `description: stabby::string::String`
- `input_schema: stabby::string::String` (JSON schema for the tool parameters)

The host intercepts `TOPIC_MCP_REGISTER_TOOL` messages and inserts them into the `McpRegistry`.

**Flow:**

1. The MCP server reads all registered tools from the `McpRegistry` at list-time.
2. For each tool, it reports `name`, `description`, and `inputSchema` dynamically to the MCP client.
3. When a tool is called, the host sends an `InvokeToolMessage` to the owning plugin via `TOPIC_MCP_INVOKE_TOOL` with a correlation ID and JSON-encoded
   arguments.
4. The plugin handles `InvokeToolMessage` via `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>`, executes the action (e.g., `PowerCommand::Execute`),
   and responds with an `InvokeToolResponse` broadcast to `TOPIC_MCP_TOOL_RESPONSE`.
5. The host's `McpResponseTracker` resolves the pending response by correlation ID and returns it to the MCP server.

**Example JSON schema for `plugin.audio.set_volume`:**

```json
{
  "name": "plugin.audio.set_volume",
  "description": "Sets the audio volume to an absolute value.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "volume": {
        "type": "number",
        "minimum": 0.0,
        "maximum": 1.0,
        "description": "Absolute volume level between 0.0 and 1.0"
      }
    },
    "required": [
      "volume"
    ]
  }
}
```

**Recommended combination:**

- **Generic core tools** (`open_area`, `close_area`, `list_areas`, `list_all_areas`, `focus_area`, `send_message`, `toggle_area`, `get_area_config`,
  `open_transient_area`) for launcher and broker control.
- **Plugin tools** (`system_power_action`, `network_toggle_radio`, `add_wallpaper_theme`, `sysinfo_refresh`, `weather_refresh`, `get_current_time`, etc.)
  for semantically specific actions.
- **Proposed plugin tools** (`plugin.audio.*`, `plugin.mpris.*`, `plugin.hyprland.*`) for future semantically specific actions.

---

## 9. Current Implementation Status

Implemented and building:

* Dedicated `mcp-server` crate using `rust-mcp-sdk` and `rust-mcp-axum` for Streamable HTTP + SSE transport.
* Server starts automatically in the launcher (no CLI flag required).
* Core tools: `open_area`, `close_area`, `list_areas`, `list_all_areas`, `focus_area`, `send_message`, `toggle_area`, `get_area_config`,
  `open_transient_area`.
* Core resources: `area://list`.
* Plugin-Tool-Registry and Plugin-Resource-Registry in `model/mcp` with `RegisterToolMessage`, `RegisterResourceMessage`, `InvokeToolMessage`,
  `InvokeToolResponse`, `InvokeResourceMessage`, `InvokeResourceResponse`.
* Dynamic tool/resource registration by plugins via message broker topics (`mcp.register.tool`, `mcp.register.resource`).
* Plugin tool and resource invocation via `mcp.invoke.tool` / `mcp.invoke.resource` with correlation IDs and `McpResponseTracker`.
* Implemented service plugin tools:
    * `services/power`: `system_power_action`, `system_schedule_power_action`, `system_cancel_power_action`, `system_reboot_to_uefi`.
    * `services/network`: `network_toggle_radio`, `network_connect_wifi`, `network_toggle_vpn`, `network_get_public_ip`.
    * `services/wallpaper`: `add_wallpaper_theme`, `remove_wallpaper_theme`, `select_wallpaper_theme`, `start_selected_wallpaper_process`,
      `stop_current_wallpaper_process`.
    * `services/sysinfo`: `sysinfo_refresh`.
    * `services/weather`: `weather_refresh`, `weather_get_forecast`.
* Implemented widget plugin tools:
    * `plugins/clock`: `get_current_time`.
    * `plugins/weather`: `weather_widget_refresh`.
* Implemented service plugin resources:
    * `services/sysinfo`: `sysinfo://cpu`, `sysinfo://memory`, `sysinfo://disk`, `sysinfo://temperature`, `sysinfo://network`, `sysinfo://uptime`.
    * `services/power`: `power://capabilities`, `power://inhibitors`, `power://scheduled_actions`.
    * `services/network`: `network://status`, `network://scan-results`, `network://vpn-profiles`.
    * `services/wallpaper`: `wallpaper://status`, `wallpaper://themes`.
    * `services/weather`: `weather://current`.
* Implemented widget plugin resources:
    * `plugins/clock`: `clock://time`.
    * `plugins/weather`: `weather://widget`.
* JSON-RPC error code constants, MCP-compliant response formats, `initialize`, `initialized`, and `ping`.
* Optional bearer token authentication via `McpServerConfig::auth_token`.
* Session notifications: `notifications/tools/list_changed`, `notifications/resources/list_changed`, and `notifications/resources/updated` pushed over SSE.
* Server advertises `resources.subscribe: true` and `tools.listChanged: true`.
* Per-session SSE response routing via `sessionId` query parameter.
* SSE response headers: `Cache-Control: no-store`, `Connection: keep-alive`, `X-Accel-Buffering: no`.
* 10-second timeouts on all tool/resource invocations to prevent hanging clients.
* Graceful error handling: parse errors, unknown methods, and timeouts are returned as JSON-RPC errors without closing the SSE stream.
* Unit tests for JSON-RPC types in `mcp-server/src/jsonrpc.rs`.
* Manual smoke test script `test_mcp_server.sh`.

Services and plugins **without** MCP functionality (not yet implemented):

* Services: `app-launcher`, `audio`, `gnome`, `http`, `hyprland`, `notifications`, `wayland`.
* Plugins: `app-launcher`, `audio`, `button`, `mpris`, `network` (dependency only), `notifications`, `power` (dependency only), `sysinfo`, `wallpaper`,
  `workspace-switcher`.

---

## 10. Roadmap

### Phase 1: Foundation (MVP) ✅

* Create crate `mcp-server`.
* Add `rust-mcp-sdk`, `rust-mcp-axum`, and `tokio` dependencies.
* Implement MCP transport via `rust-mcp-axum`'s `create_axum_server` with Streamable HTTP + SSE.
* Start the MCP server as an integrated task in the core.
* Tool registry with `open_area`, `close_area`, `list_areas`, `focus_area`, `send_message`.
* Resource registry with `area://list`.
* `send_message` processes only `serde_json` payloads.

### Phase 2: Tool Extensions ✅

* Implement `toggle_area`, `get_area_config`, `list_all_areas`, `open_transient_area`.
* `reload_config`, `send_action`, `trigger_widget`, `set_global_theme`, `play_sound` remain proposed (not yet implemented).

### Phase 3: Plugin-Resource-Registry, Service Resources & Plugin Tools 🚧

* Define generic **Plugin-Resource-Registry** and **Plugin-Tool-Registry**. ✅
* The following service plugins have implemented and registered their resources:
    * `sysinfo`: `sysinfo://cpu`, `sysinfo://memory`, `sysinfo://disk`, `sysinfo://temperature`, `sysinfo://network`, `sysinfo://uptime` ✅
    * `power`: `power://capabilities`, `power://inhibitors`, `power://scheduled_actions` ✅
    * `network`: `network://status`, `network://scan-results`, `network://vpn-profiles` ✅
    * `wallpaper`: `wallpaper://status`, `wallpaper://themes` ✅
    * `weather`: `weather://current` ✅
* The following service plugins have implemented and registered their tools:
    * `sysinfo`: `sysinfo_refresh` ✅
    * `power`: `system_power_action`, `system_schedule_power_action`, `system_cancel_power_action`, `system_reboot_to_uefi` ✅
    * `network`: `network_toggle_radio`, `network_connect_wifi`, `network_toggle_vpn`, `network_get_public_ip` ✅
    * `wallpaper`: `add_wallpaper_theme`, `remove_wallpaper_theme`, `select_wallpaper_theme`, `start_selected_wallpaper_process`,
      `stop_current_wallpaper_process` ✅
    * `weather`: `weather_refresh`, `weather_get_forecast` ✅
* The following widget plugins have implemented and registered their tools and resources:
    * `clock`: `get_current_time` tool, `clock://time` resource ✅
    * `weather`: `weather_widget_refresh` tool, `weather://widget` resource ✅
* The following service plugins must still implement and register their resources:
    * `audio`: Snapshot `plugin://audio/status` as well as fine-grained resources `plugin://audio/volume`, `plugin://audio/muted`, `plugin://audio/active_sink`,
      `plugin://audio/sinks` ⏳
    * `mpris`: `mpris://status`, `mpris://players`, `mpris://playback`, `mpris://metadata` ✅
        * `notifications`: `plugin://notifications/status` ⏳
        * `app_launcher`: `plugin://app_launcher/running_apps` ⏳
        * `hyprland`: `plugin://hyprland/active_workspace` (new status tracking needed) ⏳
        * `http`: `plugin://http/stats` (new status tracking needed) ⏳
* The following service plugins must still implement and register their tools:
    * `audio`: `plugin.audio.volume_up`, `plugin.audio.volume_down`, `plugin.audio.set_volume`, `plugin.audio.toggle_mute`, `plugin.audio.mute`,
      `plugin.audio.unmute`, `plugin.audio.next_device`, `plugin.audio.previous_device`, `plugin.audio.refresh_status` ⏳
    * `mpris`: `mpris_play`, `mpris_pause`, `mpris_toggle_play_pause`, `mpris_stop`, `mpris_next_track`, `mpris_previous_track`, `mpris_seek`,
      `mpris_set_position`, `mpris_cycle_loop`, `mpris_toggle_shuffle`, `mpris_next_player`, `mpris_previous_player`, `mpris_raise`, `mpris_quit`,
      `mpris_refresh_status` ✅
        * `hyprland`: `plugin.hyprland.switch_workspace` (new status tracking needed) ⏳
* Core resources still pending: `area://<area_id>/state`, `area://current/focus`, `area://current/visible` ⏳

### Phase 4: Protocol Stabilization ✅

* Per-session SSE response routing with `sessionId` query parameter.
* Broadcast notifications to all connected SSE clients.
* SSE keep-alive and anti-buffering headers (`Cache-Control: no-store`, `Connection: keep-alive`, `X-Accel-Buffering: no`).
* 10-second timeouts on tool/resource invocations.
* JSON-RPC error responses for parse errors, unknown methods, and timeouts without closing the SSE stream.
* Optional bearer token authentication via `McpServerConfig::auth_token`.
* CORS support for browser-based MCP clients. ⏳
* Add sampling/logging support for MCP clients. ⏳

### Phase 5: Integration & Tests

* Manual integration tests with Claude Desktop and other MCP clients.
* End-to-end tests for plugin tool/resource invocation.

### Phase 6: Advanced Notifications ✅

* Push resource content updates to subscribed SSE clients.
* Add `notifications/resources/updated` and `notifications/tools/updated`.

---

## 11. Dependencies

* `rust-mcp-sdk` for MCP protocol handling (`ServerHandler` trait, schema types).
* `rust-mcp-axum` for Streamable HTTP + SSE transport via `create_axum_server`.
* `tokio` for async runtime and timeouts.
* `async-channel` for `McpCommand` channel between server and launcher core.
* `async-trait` for the `ServerHandler` trait implementation.
* `tower-http` for CORS middleware (proposed).
* `uuid` for per-session SSE connection identifiers.
* Standalone JSON-RPC implementation in `mcp-server/src/jsonrpc.rs`.
* `stabby` for ABI-stable FFI messages shared with plugins.
* Access to the internal `AreaManager` and `MessageBroker` of the launcher.
* Plugin-Resource-Registry and Plugin-Tool-Registry in `model/mcp`.
* JSON converter registry for serializing stabby-FFI types to JSON.

---

*Concept for exposing the Smearor Swipe Launcher as an MCP server, focusing on area control, broker messages, central resource registry, and plugin-tool
registry, with Streamable HTTP + SSE transport powered by `rust-mcp-sdk` and `rust-mcp-axum`.*
