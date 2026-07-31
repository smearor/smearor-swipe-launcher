# Concept: MCP Server Self-Inspection

This document describes the concept for **MCP Server Self-Inspection** — a set of resources and tools that allow an MCP client (e.g., an AI assistant) to
dynamically discover the launcher's own configuration, including areas, plugins, buttons, and their associated message topics and payloads. This enables the
client to reason about available capabilities without reading configuration files directly.

---

## 1. Goal & Motivation

The *Smearor Swipe Launcher* already exposes tools and resources through its MCP server. However, the current resources are service-specific (e.g.,
`audio://status`, `wallpaper://themes`) and do not provide a general view of the launcher's configuration structure. An MCP client cannot currently answer
questions like:

- Which buttons are configured and in which area?
- What topics and payloads does a specific button publish on click, long-press, or swipe?
- Which plugins are loaded and what are their IDs?
- What is the full area layout (areas, their plugins, their configuration)?

This information is essential for an AI agent that needs to dynamically discover how to control smart-home devices (e.g., Shelly lights configured as button
widgets) or trigger actions in other plugins without hard-coding knowledge of the configuration.

**Benefits:**

- **Dynamic capability discovery:** An AI client can inspect all configured buttons and their topics to understand what actions are available.
- **Smart-home control:** Buttons that send HTTP requests to IoT devices (e.g., Shelly lights) become discoverable and controllable via MCP.
- **Self-documenting launcher:** The launcher exposes its own configuration through a standardized MCP interface, eliminating the need for clients to parse
  `config.toml` directly.
- **Agent autonomy:** A voice assistant or AI agent can reason about available actions and compose multi-step workflows without prior knowledge of the
  configuration.

---

## 2. Architecture

The self-inspection capabilities are implemented as **core MCP resources** in the `mcp-server` crate. They read from the launcher's in-memory configuration
structures (not from files) and return JSON representations.

```
┌─────────────────────────────────────────────────────┐
│                  MCP CLIENT                         │
│  (e.g., Claude, Cursor, Voice Assistant Agent)     │
└──────────────────────┬──────────────────────────────┘
                       │ JSON-RPC / MCP
                       ▼
┌─────────────────────────────────────────────────────┐
│              MCP Server (rust-mcp-sdk)              │
│                                                     │
│  Core Resources:                                    │
│  ├── area://list          (existing)                │
│  ├── area://plugins       (NEW)                     │
│  ├── area://buttons       (NEW)                     │
│  ├── plugin://list        (NEW)                     │
│  └── launcher://config    (planned, see concept)    │
│                                                     │
│  Core Tools:                                        │
│  ├── get_area_config      (existing)                │
│  ├── trigger_button       (NEW)                     │
│  └── send_message         (existing)                │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│            Smearor Swipe Launcher Core               │
│  ┌──────────────────┐  ┌────────────────────────┐  │
│  │  Area Manager     │  │  Central Message Broker │  │
│  │  (areas, plugins) │  │  (publish/subscribe)    │  │
│  └──────────────────┘  └────────────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │  SwipeLauncherConfig (in-memory)              │  │
│  │  (areas, entries, plugins, button configs)    │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 3. Proposed Resources

All resources return JSON and are registered as core resources in the MCP server (not plugin-registered).

### 3.1 `area://plugins`

Lists all plugins across all configured areas, including their plugin ID, area assignment, and widget library path.

**Response format:**

```json
{
  "areas": [
    {
      "area_id": "livingroom_area",
      "plugins": [
        {
          "id": "livingroom_area_close_button",
          "path": "target/release/libsmearor_button_widget.so",
          "disabled": false
        },
        {
          "id": "shelly_livingroom_couch_floor_light_button",
          "path": "target/release/libsmearor_button_widget.so",
          "disabled": false
        }
      ]
    },
    {
      "area_id": "scroll_band",
      "plugins": [
        {
          "id": "clock",
          "path": "target/release/libsmearor_clock_widget.so",
          "disabled": false
        }
      ]
    }
  ]
}
```

**Implementation:** Reads from `SwipeLauncherConfig::areas` and `SwipeLauncherConfig::entries`, iterating over each area's plugin list and resolving the plugin
entry from the config entries map.

---

### 3.2 `area://buttons`

Lists all configured button widgets across all areas with their full action configuration (topics, payloads, state topics, icons, etc.). This is the primary
resource for smart-home discovery.

**Response format:**

```json
{
  "buttons": [
    {
      "button_id": "shelly_livingroom_couch_floor_light_button",
      "area_id": "livingroom_area",
      "text": "Living Room Couch Floor",
      "icon": "nf-md-led_strip",
      "icon_only": false,
      "click_topic": "service.http.request",
      "click_payload": {
        "method": "Get",
        "url": "http://192.168.178.25/color/0?turn=on&red=255&green=0&blue=0&gain=100",
        "response_topic": "service.http.response.shelly.livingroom.couch.floor.light"
      },
      "longpress_topic": "service.http.request",
      "longpress_payload": {
        "method": "Get",
        "url": "http://192.168.178.25/color/0?turn=off",
        "response_topic": "service.http.response.shelly.livingroom.couch.floor.light"
      },
      "state_topic": "service.http.response.shelly.livingroom.couch.floor.light",
      "state_icon": "{ison?nf-md-led_strip:nf-md-led_strip_variant_off}",
      "state_css_class": "light-active",
      "swipe_up_topic": "service.http.request",
      "swipe_up_payload": {
        "method": "Get",
        "url": "http://192.168.178.25/color/0?turn=on&red=255&green=0&blue=0&gain={gain+10}",
        "response_topic": "service.http.response.shelly.livingroom.couch.floor.light"
      },
      "swipe_down_topic": "service.http.request",
      "swipe_down_payload": {
        "method": "Get",
        "url": "http://192.168.178.25/color/0?turn=on&red=255&green=0&blue=0&gain={gain-10}",
        "response_topic": "service.http.response.shelly.livingroom.couch.floor.light"
      }
    }
  ]
}
```

**Implementation:** Iterates over all area configs, filters for plugins whose library path contains `libsmearor_button_widget`, and resolves the button's
configuration entry from `SwipeLauncherConfig::entries`. Only fields that are present in the config are included in the response (optional fields are omitted if
not configured).

**Filtering:** The resource supports optional query parameters (via tool arguments, not URI) to filter by `area_id` or by `topic` prefix (e.g., only buttons
that publish to `service.http.request`).

---

### 3.3 `plugin://list`

Lists all loaded plugins (services and widgets) with their IDs, library paths, and type (service or widget).

**Response format:**

```json
{
  "plugins": [
    {
      "id": "app_launcher",
      "path": "target/release/libsmearor_app_launcher_service.so",
      "type": "service"
    },
    {
      "id": "clock",
      "path": "target/release/libsmearor_clock_widget.so",
      "type": "widget"
    }
  ]
}
```

**Implementation:** Reads from the `ServiceManager` (for services) and `PluginManager` (for widgets), combining both into a single list.

---

## 4. No New Tools Required

The existing `send_message` tool already allows an MCP client to publish any topic/payload combination to the broker. Once the client has discovered button
configurations via the `area://buttons` resource, it can use `send_message` directly to trigger any button action — no dedicated `trigger_button` wrapper is
needed.

This keeps the tool surface minimal and avoids redundancy.

---

## 5. Implementation Plan

### 5.1 Crate Structure

All changes are in the `mcp-server` crate and the launcher core. No new crates are needed.

```
mcp-server/
├── src/
│   ├── lib.rs          (McpServer, ServerHandler)
│   ├── resources.rs    (ResourceDefinition, core resources)
│   ├── tools.rs        (ToolDefinition, core tools)
│   └── self_inspection.rs  (NEW: self-inspection resource handlers)
└── Cargo.toml
```

### 5.2 Data Flow

The MCP server needs access to the launcher's in-memory configuration. This is achieved through the existing `McpCommand` channel:

1. The MCP server receives a resource read request (e.g., `area://buttons`).
2. It sends an `McpCommand::ReadResource` to the launcher core.
3. The launcher core reads from `SwipeLauncherConfig` (which is already loaded in memory).
4. The response is sent back via a `oneshot` channel.

Alternatively, if the config is static after startup, a shared `Arc<SwipeLauncherConfig>` could be passed directly to the `McpServer` at initialization time,
avoiding the round-trip through the command channel. This is the preferred approach for read-only data.

### 5.3 Phases

| Phase | Description                         | Dependencies                                   | Exit Criteria                                            |
|-------|-------------------------------------|------------------------------------------------|----------------------------------------------------------|
| **1** | Implement `area://plugins` resource | Access to `SwipeLauncherConfig` in `McpServer` | MCP client can list all plugins per area                 |
| **2** | Implement `area://buttons` resource | Phase 1                                        | MCP client can list all buttons with topics and payloads |
| **3** | Implement `plugin://list` resource  | Access to `ServiceManager` and `PluginManager` | MCP client can list all loaded plugins                   |
| **4** | Integration tests                   | Phases 1–3                                     | Verify all resources work end-to-end                     |

---

## 6. Security Considerations

- **Sensitive data:** Button payloads may contain URLs with IP addresses of local IoT devices (e.g., `http://192.168.178.25`). The MCP server already binds to
  `127.0.0.1` by default, so this data is only exposed locally. When `bind_address = "0.0.0.0"` is configured, an `auth_token` should be required.
- **No new write actions:** The self-inspection resources are read-only. Triggering button actions is done via the existing `send_message` tool, which carries
  the same risk level as before.

---

## 7. Use Cases

### 7.1 Smart-Home Light Control

An AI agent discovers all configured Shelly light buttons and turns them off:

1. Client reads `area://buttons` resource.
2. Client filters buttons whose `longpress_payload.url` contains `turn=off`.
3. Client calls `send_message` for each matching button, using the `longpress_topic` and `longpress_payload` from the resource response.

### 7.2 Dynamic Capability Discovery

A voice assistant needs to know what actions are available:

1. Client reads `area://plugins` to understand the launcher layout.
2. Client reads `area://buttons` to discover all configurable actions.
3. Client reads `plugin://list` to see which services are loaded.
4. Client combines this with the existing `list_tools` and `list_resources` MCP methods to build a complete picture of available capabilities.

### 7.3 Area Inspection

An AI client wants to understand a specific area:

1. Client calls `get_area_config` with `area_id: "livingroom_area"`.
2. Client reads `area://buttons` and filters by `area_id: "livingroom_area"`.
3. Client now knows all buttons in that area and can trigger them via `send_message`.

---

## 8. Relationship to Existing Concepts

- **MCP_SERVER_CONCEPT.md:** This concept extends the existing MCP server with self-inspection resources. It does not replace any existing tools or resources.
- **BUTTON_WIDGET.md:** The button widget configuration format is the data source for `area://buttons`. No changes to the button widget itself are needed.
- **SPEAK_SWIPER_CONCEPT.md:** The voice assistant agent concept relies on dynamic tool discovery. The self-inspection resources provide the foundation for the
  agent to reason about available buttons and actions without hard-coding configuration knowledge.
- **HTTP_SERVICE_CONCEPT.md:** Buttons that control IoT devices (e.g., Shelly lights) use the HTTP service as their `click_topic`. The self-inspection resources
  expose these button configurations, making IoT devices discoverable via MCP.
