# MCP Server Concept for Smearor Swipe Launcher

This document describes the concept for an **MCP Server (Model Context Protocol)** that exposes the *Smearor Swipe Launcher* through a standardized interface
for external AI clients. The MCP server runs as an integrated part of the launcher application and communicates with MCP clients exclusively via **SSE (
Server-Sent Events)** over an **axum** web server.

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
dedicated tokio task inside the launcher process. The transport is exclusively **SSE over an axum HTTP server**.

**Process model:** The MCP server runs as a dedicated task inside the launcher process. No separate process is required.

**Permissions:** An optional bearer token can be configured via `McpServerConfig::auth_token`. When set, all endpoints require an
`Authorization: Bearer <token>`
header. Without a token, the server only binds to the configured address (default `127.0.0.1:8765`).

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP CLIENT                              │
│  (e.g., Claude Desktop, Cursor, VS Code Extension)          │
└─────────────────────┬───────────────────────┬───────────────┘
                      │ JSON-RPC / MCP over SSE │
                      ▼                         ▼
┌──────────────────────────────────────────┐  ┌────────────────────────┐
│          MCP Server (axum + SSE)         │  │  Resource/Tool Registry │
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

| Tool              | Description                                                                              | Parameters                                                      |
|-------------------|------------------------------------------------------------------------------------------|-----------------------------------------------------------------|
| `open_area`       | Opens a defined area by its ID.                                                          | `area_id: string`                                               |
| `close_area`      | Closes a currently visible area.                                                         | `area_id: string`                                               |
| `list_areas`      | Lists all configured areas with ID, position, activation status, and current visibility. | –                                                               |
| `focus_area`      | Sets focus on an area (e.g., for keyboard navigation).                                   | `area_id: string`                                               |
| `send_message`    | Publishes a message to a topic on the central broker.                                    | `topic: string`, `payload: json`, `target_instance_id?: string` |
| `toggle_area`     | Toggles the visibility of an area.                                                       | `area_id: string`                                               |
| `get_area_config` | Returns the configuration of an area as JSON.                                            | `area_id: string`                                               |

### 3.2 Additional Useful Tools

| Tool               | Description                                                                  | Parameters                                            |
|--------------------|------------------------------------------------------------------------------|-------------------------------------------------------|
| `reload_config`    | Reloads the configuration files (`config.toml`, `services.toml`).            | –                                                     |
| `send_action`      | Sends a typed action to a widget or service (e.g., `AppLaunch`, `VolumeUp`). | `plugin_id: string`, `action: string`, `params: json` |
| `trigger_widget`   | Triggers the primary interaction event of a widget (e.g., tap).              | `widget_id: string`, `event: string`                  |
| `set_global_theme` | Switches the global CSS theme (e.g., light/dark).                            | `theme: string`                                       |
| `play_sound`       | Plays a configured system sound.                                             | `sound_id: string`                                    |

### 3.3 Plugin-Provided Tools

In addition to generic core tools, service plugins can register their own, semantically typed tools via the **Plugin-Tool-Registry**. The MCP server queries
this registry and exposes the tools dynamically.

**Example: Audio Service (based on `PulseCommand`):**

| Tool                           | Description                            | Parameters                |
|--------------------------------|----------------------------------------|---------------------------|
| `plugin.audio.volume_up`       | Increases volume by a configured step. | –                         |
| `plugin.audio.volume_down`     | Decreases volume by a configured step. | –                         |
| `plugin.audio.set_volume`      | Sets the volume to an absolute value.  | `volume: f32` (0.0 – 1.0) |
| `plugin.audio.toggle_mute`     | Toggles the mute state.                | –                         |
| `plugin.audio.mute`            | Enables mute.                          | –                         |
| `plugin.audio.unmute`          | Disables mute.                         | –                         |
| `plugin.audio.next_device`     | Selects the next audio device.         | –                         |
| `plugin.audio.previous_device` | Selects the previous audio device.     | –                         |
| `plugin.audio.refresh_status`  | Manually re-reads the status.          | –                         |

Other plugins (e.g., `sysinfo`, `mpris`, `hyprland`) register their own tools analogously, e.g., `sysinfo.refresh`, `plugin.mpris.play_pause`,
`plugin.hyprland.switch_workspace`.

---

## 4. Proposed Resources

Resources are values queryable by the MCP client that reflect the current state of the launcher or the system.

### 4.1 Mandatory Resources (from the MVP)

The `AreaManager` and **every service plugin** must register at least their central state resources via the Plugin-Resource-Registry. The MCP server then offers
these resources dynamically.

#### Core Resources (AreaManager)

| URI                      | Description                                            | Format |
|--------------------------|--------------------------------------------------------|--------|
| `area://list`            | List of all configured areas with status and position. | JSON   |
| `area://<area_id>/state` | Current state of an area (open, focused, visible).     | JSON   |
| `area://current/focus`   | Currently focused area.                                | JSON   |
| `area://current/visible` | Currently visible area(s).                             | JSON   |

#### Service Plugin Resources

Each service plugin provides a **snapshot resource** for the complete status and, where useful, **fine-grained individual resources** for frequently queried
values.

| Service         | URI                                  | Description                                                   | Source type                      |
|-----------------|--------------------------------------|---------------------------------------------------------------|----------------------------------|
| `app_launcher`  | `plugin://app_launcher/running_apps` | Status of all monitored `.desktop` files (running / stopped). | `DesktopFileStatusMessageStabby` |
| `audio`         | `plugin://audio/status`              | Complete audio status (volume, mute, devices, active device). | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/volume`              | Current volume (0.0 – 1.0).                                   | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/muted`               | Current mute status.                                          | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/active_sink`         | Active output device with name, index, and channels.          | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/sinks`               | List of all available output devices.                         | `AudioStatusMessage`             |
| `mpris`         | `plugin://mpris/status`              | Active players, playback status, metadata, position, volume.  | `MprisStatusMessage`             |
| `notifications` | `plugin://notifications/status`      | Do-not-disturb, active notifications, unread count.           | `NotificationStatusMessage`      |
| `sysinfo`       | `sysinfo://cpu`                      | CPU usage and temperature.                                    | `CpuStatusMessage`               |
| `sysinfo`       | `sysinfo://memory`                   | RAM usage, total, used, available.                            | `MemoryStatusMessage`            |
| `sysinfo`       | `sysinfo://battery`                  | Battery level and charging state.                             | `BatteryStatusMessage`           |
| `sysinfo`       | `sysinfo://disks`                    | Mount point usage, read/write throughput.                     | `DisksStatusMessage`             |
| `sysinfo`       | `sysinfo://network`                  | Incoming/outgoing network throughput.                         | `NetworkStatusMessage`           |
| `sysinfo`       | `sysinfo://uptime`                   | Uptime in seconds and load average.                           | `UptimeStatusMessage`            |
| `hyprland`      | `plugin://hyprland/active_workspace` | Current workspace and window list (to be implemented).        | Custom status message            |
| `http`          | `plugin://http/stats`                | Last request statistics or last response (to be implemented). | Custom status message            |

### 4.2 Additional Useful Resources

| URI                   | Description                           | Format    |
|-----------------------|---------------------------------------|-----------|
| `launcher://config`   | Entire active launcher configuration. | JSON/TOML |
| `launcher://version`  | Version of the launcher application.  | JSON      |
| `network://status`    | Network status (connected, SSID, IP). | JSON      |
| `bluetooth://devices` | Paired Bluetooth devices.             | JSON      |

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

Every tool and resource call that waits for the launcher core uses a 5-second timeout. If the launcher core does not respond in time, the server returns a
JSON-RPC error with `JSONRPC_INTERNAL_ERROR` and a clear timeout message, instead of leaving the client waiting indefinitely.

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

---

## 8. Resource Implementation Details (MVP)

### 8.1 Area Status and Current Area

The resources `area://list`, `area://<area_id>/state`, `area://current/focus`, and `area://current/visible` are read directly from the `AreaManager`. The
`AreaManager` registers them at startup via the Plugin-Resource-Registry. An area status resource contains at least:

```json
{
  "area_id": "audio",
  "visible": true,
  "focused": false,
  "position": "bottom",
  "active": true
}
```

### 8.2 Service Plugin Resources

Each service plugin keeps its current state and registers the corresponding resources. Example `plugin://sysinfo/cpu`:

```
URI: plugin://sysinfo/cpu
MIME type: application/json
Body: { "cpu_usage": 12.5, "cpu_temperature": 45.2 }
```

Example `plugin://audio/status`:

```
URI: plugin://audio/status
MIME type: application/json
Body: {
  "volume": 0.8,
  "is_muted": false,
  "active_device": { "id": 1, "name": "Built-in Audio", "is_default": true }
}
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
│  plugin://sysinfo/cpu  -> PluginHandler │
│  plugin://audio/volume -> PluginHandler │
│  plugin://clock/time   -> PluginHandler │
└─────────────────────────────────────────┘
```

**Mechanism:**

1. Every plugin (widget or service) can register resources during initialization via a new callback in `FfiCoreContext`:
    - `resource_uri: stabby::string::String` (e.g., `plugin://sysinfo/cpu`)
    - `metadata: ResourceMetadata` (name, description, MIME type)
    - `read_fn: extern "C" fn(...) -> DynFuture<'static, stabby::string::String>`
2. The MCP server queries the registry at startup and registers all URIs dynamically with the MCP client.
3. For `read_resource(plugin://<plugin>/<name>)`, the MCP server calls the corresponding `read_fn` of the plugin.
4. The plugin returns JSON as a `stabby::string::String`; the MCP server forwards it unchanged to the client.

Thus, even plugins developed later can provide resources without requiring changes to the MCP server or the core model. For stabby-FFI types, the plugin
converts internally via the JSON converter registry.

**No last-value cache for MCP:** Because the `AreaManager` and all service plugins expose their state explicitly as resources, the MCP interface **does not need
a `topic://<topic>/last` resource**. The MCP server reads exclusively through registered resource handlers.

The message broker may still maintain an internal last-value cache for widgets/services (late-subscriber initialization), but this cache is not visible to the
MCP server and is not exposed as a resource.

### 8.4 Plugin-Tool-Registry

Analogous to the Plugin-Resource-Registry, there is a **Plugin-Tool-Registry** through which service plugins register their own MCP tools. The MCP server
dynamically extends its tool list without having to implement tool handlers manually for each plugin.

**Registration per plugin:**

- `tool_id: stabby::string::String` (e.g., `plugin.audio.volume_up`)
- `description: stabby::string::String`
- `input_schema: stabby::string::String` (JSON schema for the tool parameters)
- `handler: extern "C" fn(...) -> DynFuture<'static, ToolResult>`

**Flow:**

1. The MCP server reads all registered tools from the registry at startup.
2. For each tool, it reports `name`, `description`, and `inputSchema` dynamically to the MCP client.
3. When a tool is called, the MCP server serializes the JSON arguments and passes them to the plugin handler.
4. The plugin executes the action (e.g., `PulseCommand::VolumeUp`) and returns a result or an error message.

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

- **Generic core tools** (`open_area`, `close_area`, `list_areas`, `focus_area`, `send_message`) for launcher and broker control.
- **Plugin tools** (`plugin.audio.*`, `plugin.mpris.*`, `plugin.hyprland.*`) for semantically specific actions.

---

## 9. Current Implementation Status

Implemented and building:

* Dedicated `mcp-server` crate with `axum` + SSE transport.
* Server starts automatically in the launcher (no CLI flag required).
* Core tools: `open_area`, `close_area`, `list_areas`, `focus_area`, `send_message`, `toggle_area`, `get_area_config`.
* Core resources: `area://list`, `area://<id>/state`.
* Plugin-Tool-Registry and Plugin-Resource-Registry in `model/mcp`.
* Dynamic tool/resource registration by plugins (e.g., `plugins/clock` registers `get_current_time` and `clock://time`; `services/sysinfo` registers
  `sysinfo.refresh` and `sysinfo://cpu`, `sysinfo://memory`, `sysinfo://battery`, `sysinfo://disks`, `sysinfo://network`, `sysinfo://uptime`).
* Plugin tool and resource invocation via `mcp.invoke.tool` / `mcp.invoke.resource` with correlation IDs and `McpResponseTracker`.
* JSON-RPC error code constants, MCP-compliant response formats, `initialize`, `initialized`, and `ping`.
* Optional bearer token authentication via `McpServerConfig::auth_token`.
* Session notifications: `notifications/tools/list_changed`, `notifications/resources/list_changed`, and `notifications/resources/updated` pushed over SSE.
* Server advertises `resources.subscribe: true`.
* Per-session SSE response routing via `sessionId` query parameter.
* SSE response headers: `Cache-Control: no-store`, `Connection: keep-alive`, `X-Accel-Buffering: no`.
* 5-second timeouts on all tool/resource invocations to prevent hanging clients.
* Graceful error handling: parse errors, unknown methods, and timeouts are returned as JSON-RPC errors without closing the SSE stream.
* Unit tests for JSON-RPC types in `mcp-server/src/jsonrpc.rs`.
* Manual smoke test script `test_mcp_server.sh`.

---

## 10. Roadmap

### Phase 1: Foundation (MVP) ✅

* Create crate `mcp-server`.
* Add `axum` and SSE middleware dependencies.
* Implement MCP transport as an axum-based SSE endpoint.
* Start the MCP server as an integrated task in the core.
* Tool registry with `open_area`, `close_area`, `list_areas`, `focus_area`, `send_message`.
* Resource registry with `area://list`, `area://<id>/state`.
* `AreaManager` implements and registers its core resources.
* `send_message` processes only `serde_json` payloads.

### Phase 2: Tool Extensions ✅

* Implement `toggle_area`, `reload_config`, `get_area_config`.
* Implement `send_action` and `trigger_widget` for direct widget control.

### Phase 3: Plugin-Resource-Registry, Service Resources & Plugin Tools 🚧

* Define generic **Plugin-Resource-Registry** and **Plugin-Tool-Registry**. ✅
* The following service plugins must implement and register their resources:
    * `sysinfo`: `sysinfo://cpu`, `sysinfo://memory`, `sysinfo://battery`, `sysinfo://disks`, `sysinfo://network`, `sysinfo://uptime` ✅
    * `audio`: Snapshot `plugin://audio/status` as well as fine-grained resources `plugin://audio/volume`, `plugin://audio/muted`, `plugin://audio/active_sink`,
      `plugin://audio/sinks` ⏳
    * `mpris`: `plugin://mpris/status` ⏳
    * `notifications`: `plugin://notifications/status` ⏳
    * `app_launcher`: `plugin://app_launcher/running_apps` ⏳
    * `hyprland`: `plugin://hyprland/active_workspace` (new status tracking needed) ⏳
    * `http`: `plugin://http/stats` (new status tracking needed) ⏳
* The following service plugins must implement and register their tools:
    * `sysinfo`: `sysinfo.refresh` ✅
    * `audio`: `plugin.audio.volume_up`, `plugin.audio.volume_down`, `plugin.audio.set_volume`, `plugin.audio.toggle_mute`, `plugin.audio.mute`,
      `plugin.audio.unmute`, `plugin.audio.next_device`, `plugin.audio.previous_device`, `plugin.audio.refresh_status` ⏳
    * `mpris`: `plugin.mpris.play_pause`, `plugin.mpris.next`, `plugin.mpris.previous`, `plugin.mpris.stop` ⏳
    * `hyprland`: `plugin.hyprland.switch_workspace` (new status tracking needed) ⏳
* Bind existing services: `network`, `bluetooth` (if available). ⏳

### Phase 4: Protocol Stabilization ✅

* Per-session SSE response routing with `sessionId` query parameter.
* Broadcast notifications to all connected SSE clients.
* SSE keep-alive and anti-buffering headers (`Cache-Control: no-store`, `Connection: keep-alive`, `X-Accel-Buffering: no`).
* 5-second timeouts on tool/resource invocations.
* JSON-RPC error responses for parse errors, unknown methods, and timeouts without closing the SSE stream.
* Optional bearer token authentication via `McpServerConfig::auth_token`.
* CORS support for browser-based MCP clients.
* Add sampling/logging support for MCP clients.

### Phase 5: Integration & Tests

* Manual integration tests with Claude Desktop and other MCP clients.
* End-to-end tests for plugin tool/resource invocation.

### Phase 6: Advanced Notifications ✅

* Push resource content updates to subscribed SSE clients.
* Add `notifications/resources/updated` and `notifications/tools/updated`.

---

## 11. Dependencies

* `axum` and `tokio` for the SSE HTTP transport.
* `tower-http` for CORS middleware.
* `uuid` for per-session SSE connection identifiers.
* Standalone JSON-RPC implementation in `mcp-server/src/jsonrpc.rs`.
* `stabby` for ABI-stable FFI messages shared with plugins.
* Access to the internal `AreaManager` and `MessageBroker` of the launcher.
* Plugin-Resource-Registry and Plugin-Tool-Registry in `model/mcp`.
* JSON converter registry for serializing stabby-FFI types to JSON.

---

*Concept for exposing the Smearor Swipe Launcher as an MCP server, focusing on area control, broker messages, central resource registry, and plugin-tool
registry, with SSE transport powered by axum.*
