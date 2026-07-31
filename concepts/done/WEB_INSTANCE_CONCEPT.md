# Concept: Web Instance — Launcher Instances via HTTP

Delivery of launcher instances via an **HTTP web server** integrated into the host. Each web instance is a `LauncherInstance` with
`instance_type = InstanceType::Web` — no GTK window, no pixel buffer, but **HTML fragments** rendered by widgets and composed into a full page by the host using
a template system.

The host runs an embedded HTTP server (`axum`). Multiple web instances are supported, each addressable via its instance ID in the URL path. Widget plugins
provide HTML fragments via a new `WebRenderer` trait. The host composes the final page from a template file.

---

## 1. Goal

Enable remote access to launcher instances via a web browser. Use cases:

- **Remote control**: Operate the launcher from a phone, tablet, or another computer.
- **Headless servers**: Run the launcher without a display, controlled entirely via web.
- **Dashboards**: Embed launcher buttons in a web dashboard.

The web instance reuses the existing plugin, area, and message broker infrastructure. No new message types are needed for instance lifecycle —
`load_instance()` / `stop_instance()` from the Dynamic Load concept (`DYNAMIC_LOAD_LAUNCHER_INSTANCE.md`) handle creation and destruction.

---

## 2. Architecture

### 2.1 Core Idea

Each web instance is registered as a **`LauncherInstance`** with `instance_type = InstanceType::Web`. The host runs an embedded `axum` HTTP server that serves
pages for all web instances. Widgets implement the `WebRenderer` trait to produce HTML fragments. The host composes the full page from a template.

```
┌──────────────────────────────────────────────────────────────────────┐
│                           Single Process                              │
│                                                                       │
│  ┌────────────────┐                                                  │
│  │ gtk4::Application│                                                 │
│  └───────┬────────┘                                                  │
│          │                                                            │
│    ┌─────┴─────┬──────────┬──────────────┬──────────────┐           │
│    │           │          │              │              │            │
│ ┌──▼──┐   ┌──▼──┐   ┌──▼─────────┐ ┌──▼─────────┐ ┌──▼─────────┐  │
│ │Win 1│   │Win 2│   │MacroPad 1  │ │Web 1       │ │Web 2       │  │
│ │Gtk  │   │Gtk  │   │Headless    │ │Web         │ │Web         │  │
│ └──┬──┘   └──┬──┘   └──┬─────────┘ └──┬─────────┘ └──┬─────────┘  │
│    │         │         │              │              │             │
│ ┌──▼─────────▼─────────▼──────────────▼──────────────▼────────┐    │
│ │               Central Message Broker                         │    │
│ └──────────────────────┬──────────────────────────────────────┘    │
│                        │                                             │
│ ┌──────────────────────▼──────────────────────────────────────┐    │
│ │  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │    │
│ │  │streamdeck│  │loupedeck │  │  axum HTTP Server (Host)  │  │    │
│ │  │service   │  │service   │  │  GET  /instances/{id}/   │  │    │
│ │  └──────────┘  └──────────┘  │  POST /instances/{id}/   │  │    │
│ │                              │       click/{plugin_id}  │  │    │
│ │                              │  WS   /instances/{id}/ws │  │    │
│ │                              └──────────────────────────┘  │    │
│ └────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 Instance Comparison

| Component        | GTK Instance        | MacroPad Instance            | Web Instance             |
|------------------|---------------------|------------------------------|--------------------------|
| Window           | `ApplicationWindow` | None (headless)              | None (served via HTTP)   |
| `PluginManager`  | Yes                 | Yes                          | Yes                      |
| `AreaManager`    | Yes (GTK widgets)   | Yes (logical areas)          | Yes (logical areas)      |
| Widget rendering | `WidgetBuilder`     | `GraphicRenderer` trait      | `WebRenderer` trait      |
| Output           | GTK widget tree     | RGBA pixel buffer            | HTML fragment            |
| Input            | GTK events          | `MacroPadInputMessage`       | HTTP POST / WebSocket    |
| State updates    | GTK signal system   | Re-render + `SetButtonImage` | WebSocket push (partial) |

### 2.3 Host Changes Summary

The host changes are **additive**, building on the Dynamic Load concept:

| Change                                                               | Location                                            | Description                                                                                    |
|----------------------------------------------------------------------|-----------------------------------------------------|------------------------------------------------------------------------------------------------|
| `InstanceType::Web` variant                                          | `model/instance-control`                            | Third instance type alongside `Gtk` and `Headless`.                                            |
| `WebServer` struct                                                   | new file `smearor-swipe-launcher/src/web/server.rs` | Embedded `axum` server, manages routes for all web instances.                                  |
| `LauncherHost::web_server` field                                     | `application.rs`                                    | Optional `Arc<WebServer>`, initialized if web server is enabled.                               |
| `route_message()` — web click handling                               | `application.rs`                                    | Converts HTTP click POSTs into broker messages.                                                |
| `LauncherInstance` gains `web_metadata: Option<WebInstanceMetadata>` | `instance.rs`                                       | Optional metadata for web instances (template path, auth token). `None` for non-web instances. |

No existing `LauncherInstance` code, broker logic, `ServiceManager`, or MCP infrastructure is modified. The host knows nothing about specific widgets — it only
calls `WebRenderer::render_html()` on plugins that implement it.

---

## 3. Plugin-API Extension: WebRenderer Trait

### 3.1 Motivation

Widgets currently implement `WidgetBuilder::build_widget() -> gtk4::Widget` (Gtk) or `GraphicRenderer::render_graphic() -> FfiGraphic` (Headless). Web instances
need **HTML fragments** instead. A new `WebRenderer` trait allows widgets to produce HTML without GTK or pixel rendering.

### 3.2 FfiString Struct

```rust
/// An FFI-safe string for passing HTML fragments across the plugin boundary.
#[repr(C)]
pub struct FfiString {
    /// UTF-8 string data.
    pub data: *mut u8,
    /// Length of the string in bytes.
    pub len: usize,
}

impl FfiString {
    pub fn from_string(s: String) -> Self {
        let len = s.len();
        let mut boxed = s.into_bytes().into_boxed_slice();
        let data = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        Self { data, len }
    }

    pub fn as_str(&self) -> &str {
        if self.data.is_null() || self.len == 0 {
            ""
        } else {
            unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.data, self.len)) }
        }
    }

    pub fn null() -> Self {
        Self { data: std::ptr::null_mut(), len: 0 }
    }
}
```

### 3.3 WebRenderer Trait

```rust
/// Trait for widgets that can render to HTML for web instances.
pub trait WebRenderer {
    /// Render the widget as an HTML fragment.
    /// `instance_id` and `plugin_id` are provided for data-attribute wiring
    /// (e.g. `data-plugin-id`, `data-click-topic`).
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String;
}
```

### 3.4 Plugin VTable Extension

`PluginVTable` gains an optional `render_html` function pointer. `PLUGIN_VTABLE_VERSION` is incremented to `3`. Existing GTK-only and Headless-only widgets set
this to `None`.

```rust
#[repr(C)]
pub struct PluginVTable {
    pub destroy: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
    pub build_widget: unsafe extern "C" fn(instance: *mut core::ffi::c_void) -> FfiWidget,
    pub on_message: unsafe extern "C" fn(instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void),
    pub start: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
    // New in v2:
    pub render_graphic: Option<unsafe extern "C" fn(
        instance: *mut core::ffi::c_void,
        width: u32,
        height: u32,
    ) -> FfiGraphic>,
    // New in v3:
    pub render_html: Option<unsafe extern "C" fn(
        instance: *mut core::ffi::c_void,
        instance_id: *const u8, instance_id_len: usize,
        plugin_id: *const u8, plugin_id_len: usize,
    ) -> FfiString>,
}
```

---

## 4. Button Widget HTML Rendering

### 4.1 HTML Fragment

The Button Widget implements `WebRenderer::render_html()`:

```html
<button class="smearor-button"
        data-plugin-id="button_apps"
        data-click-topic="area.app_launcher.open"
        data-click-instance="web_1"
        data-click-payload="">
  <span class="smearor-button-icon nf-md-apps"></span>
  <span class="smearor-button-label">Apps</span>
</button>
```

The fragment uses:

- **CSS classes**: `smearor-button`, `smearor-button-icon`, `smearor-button-label` for styling.
- **NerdFont icons**: `nf-md-*` classes (same icon identifiers as GTK/Headless, rendered via NerdFont CSS on the web).
- **Data attributes**: `data-click-topic`, `data-click-instance`, `data-click-payload` for click wiring.
- **State classes**: `smearor-button--active` / `smearor-button--inactive` for state visualization (same `state_topic` / `state_css_class` logic as GTK).

### 4.2 State Updates

When a plugin receives a state update via `state_topic`, the web instance triggers a re-render of the affected widget's HTML fragment and pushes it via
WebSocket to all connected browsers (partial update, no full page reload).

### 4.3 Dependencies

| Crate        | Purpose                                   |
|--------------|-------------------------------------------|
| `serde_json` | JSON serialization for WebSocket messages |
| No new crate | HTML generation is pure string formatting |

---

## 5. Template System

### 5.1 Overview

The host composes the full HTML page from a **template file** and widget fragments. Templates use simple `{{placeholder}}` syntax — no external template engine
dependency.

### 5.2 Template File

Templates are HTML files with placeholders. The host replaces placeholders at serve time:

| Placeholder          | Replaced with                                                |
|----------------------|--------------------------------------------------------------|
| `{{instance_id}}`    | The instance ID (e.g. `web_1`)                               |
| `{{instance_title}}` | Display title from instance config                           |
| `{{widgets}}`        | Concatenated HTML fragments from all plugins                 |
| `{{css_path}}`       | Path to the CSS file (e.g. `/instances/web_1/style.css`)     |
| `{{js_path}}`        | Path to the JavaScript file (e.g. `/instances/web_1/app.js`) |

### 5.3 Default Template

File: `resources/web/template-default.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{instance_title}}</title>
    <link rel="stylesheet" href="{{css_path}}">
    <link rel="stylesheet" href="/static/nerdfont.css">
</head>
<body>
    <div class="smearor-instance" data-instance-id="{{instance_id}}">
        <div class="smearor-areas">
            {{widgets}}
        </div>
    </div>
    <script src="{{js_path}}"></script>
</body>
</html>
```

### 5.4 Custom Templates

Each web instance can specify a custom template path in its config:

```toml
[web]
template_path = "resources/web/template-dashboard.html"
```

If no `template_path` is configured, the default template (`resources/web/template-default.html`) is used. Template paths are validated against the same config
path allowlist as instance configs (current working directory + `~/.config/smearor/`).

### 5.5 Template Rendering

```rust
/// Render a template by replacing placeholders with instance-specific values.
fn render_template(
    template: &str,
    instance_id: &str,
    instance_title: &str,
    widgets_html: &str,
    css_path: &str,
    js_path: &str,
) -> String {
    template
        .replace("{{instance_id}}", instance_id)
        .replace("{{instance_title}}", instance_title)
        .replace("{{widgets}}", widgets_html)
        .replace("{{css_path}}", css_path)
        .replace("{{js_path}}", js_path)
}
```

---

## 6. Static Assets

### 6.1 CSS

File: `resources/web/style.css`

Base styles for `smearor-button`, `smearor-button-icon`, `smearor-button-label`, `smearor-areas`, layout grid, responsive behavior. Served at
`/instances/{id}/style.css`.

### 6.2 JavaScript

File: `resources/web/app.js`

Client-side JavaScript for:

- **Click handling**: Intercept button clicks, read `data-click-*` attributes, send POST to `/instances/{id}/click/{plugin_id}`.
- **WebSocket connection**: Connect to `/instances/{id}/ws`, receive partial HTML updates, replace DOM elements by `data-plugin-id`.
- **State visualization**: Toggle CSS classes based on WebSocket state updates.

### 6.3 NerdFont CSS

File: `resources/web/nerdfont.css`

CSS mapping `nf-md-*` classes to NerdFont Unicode codepoints. Generated from the same NerdFont files used for Headless rendering
(`resources/NerdFontsSymbolsOnly/`).

### 6.4 Static File Routes

| Route                       | Serves                                                  |
|-----------------------------|---------------------------------------------------------|
| `/static/nerdfont.css`      | NerdFont CSS (shared across instances)                  |
| `/static/nerdfont.woff2`    | NerdFont web font file                                  |
| `/instances/{id}/style.css` | Instance-specific CSS (from template config or default) |
| `/instances/{id}/app.js`    | Instance-specific JavaScript (default `app.js`)         |

---

## 7. HTTP Server

### 7.1 Overview

The HTTP server is embedded in the host process, implemented with `axum`. It is initialized once when the host starts (if `web_server.enabled = true`) and
shared across all web instances.

### 7.2 WebServer Struct

```rust
/// Embedded HTTP server for web instances.
pub struct WebServer {
    /// Shared reference to the host for message routing.
    host: Arc<LauncherHost>,
    /// Port the server listens on.
    port: u16,
    /// WebSocket connections per instance ID.
    websockets: Arc<Mutex<HashMap<String, Vec<WebSocketSender>>>>,
}
```

### 7.3 Routes

| Method | Route                               | Handler               | Description                           |
|--------|-------------------------------------|-----------------------|---------------------------------------|
| GET    | `/instances/{id}/`                  | `serve_instance_page` | Compose and serve the full HTML page  |
| GET    | `/instances/{id}/style.css`         | `serve_instance_css`  | Serve instance CSS                    |
| GET    | `/instances/{id}/app.js`            | `serve_instance_js`   | Serve instance JavaScript             |
| GET    | `/instances/{id}/ws`                | `handle_websocket`    | WebSocket for real-time state updates |
| POST   | `/instances/{id}/click/{plugin_id}` | `handle_click`        | Convert click to broker message       |
| GET    | `/static/nerdfont.css`              | `serve_nerdfont_css`  | Shared NerdFont CSS                   |
| GET    | `/static/nerdfont.woff2`            | `serve_nerdfont_font` | Shared NerdFont web font              |

### 7.4 Page Composition

```rust
/// Serve the full HTML page for a web instance.
async fn serve_instance_page(
    State(host): State<Arc<LauncherHost>>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let instances = host.instances.lock();
    let Some(instance) = instances.get(&instance_id) else {
        return (StatusCode::NOT_FOUND, "Instance not found").into_response();
    };
    if instance.instance_type != InstanceType::Web {
        return (StatusCode::BAD_REQUEST, "Instance is not a web instance").into_response();
    }

    // 1. Collect HTML fragments from all plugins
    let mut widgets_html = String::new();
    if let Ok(area_manager) = instance.area_manager.lock() {
        for area in area_manager.areas() {
            for plugin in area.plugins() {
                if let Some(render_html) = plugin.vtable.render_html {
                    let fragment = unsafe {
                        let ffi = render_html(
                            plugin.instance_ptr,
                            instance_id.as_ptr(), instance_id.len(),
                            plugin.id.as_ptr(), plugin.id.len(),
                        );
                        ffi.as_str().to_string()
                    };
                    widgets_html.push_str(&fragment);
                }
            }
        }
    }

    // 2. Load template (custom or default)
    let template_path = instance.web_metadata
        .as_ref()
        .and_then(|m| m.template_path.as_deref())
        .unwrap_or("resources/web/template-default.html");
    let template = std::fs::read_to_string(template_path)
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    // 3. Render template
    let html = render_template(
        &template,
        &instance_id,
        &instance.config.title,
        &widgets_html,
        &format!("/instances/{}/style.css", instance_id),
        &format!("/instances/{}/app.js", instance_id),
    );

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()
}
```

### 7.5 Click Handling

```rust
/// Handle a click on a widget in a web instance.
async fn handle_click(
    State(host): State<Arc<LauncherHost>>,
    Path((instance_id, plugin_id)): Path<(String, String)>,
    body: axum::Json<ClickPayload>,
) -> impl IntoResponse {
    host.route_web_click(&instance_id, &plugin_id, body.0);
    StatusCode::OK
}

#[derive(Deserialize)]
struct ClickPayload {
    click_topic: String,
    click_instance: String,
    click_payload: Option<String>,
}
```

The host converts the click into a standard broker message — the same `click_topic` / `click_instance` / `click_payload` mechanism used by GTK and Headless
instances. The plugin receives it via `on_message()` and executes the action.

### 7.6 WebSocket for State Updates

When a plugin in a web instance receives a `state_topic` update, the host:

1. Calls `render_html()` on the affected plugin to get a new HTML fragment.
2. Sends a JSON message via WebSocket to all connected browsers:

```json
{
    "type": "update",
    "plugin_id": "button_weather",
    "html": "<button class=\"smearor-button smearor-button--active\" ...>...</button>"
}
```

3. Client-side JavaScript replaces the DOM element with matching `data-plugin-id`.

---

## 8. Web Instance Metadata

```rust
/// Metadata for web instances. None for non-web instances.
#[derive(Clone, Debug)]
pub struct WebInstanceMetadata {
    /// Path to a custom HTML template file. If None, the default template is used.
    pub template_path: Option<String>,
    /// Optional authentication token. If set, requests must include
    /// `Authorization: Bearer <token>` header.
    pub auth_token: Option<String>,
}
```

This field is `Option<WebInstanceMetadata>` on `LauncherInstance` — `None` for Gtk and Headless instances, `Some(...)` for Web instances. It is set during
`load_instance()` from the `[web]` section of the instance config.

---

## 9. Web Instance Config

### 9.1 Config Example

```toml
# config-web-1.toml
[launcher]
instance_id = "web_1"

[web]
template_path = "resources/web/template-dashboard.html"
auth_token = "secret-token-123"

[[areas]]
id = "main"
area_type = "fixed"
width = 480

[[areas.plugins]]
id = "button_apps"
path = "target/release/libsmearor_button_widget.so"
text = "Apps"
icon = "nf-md-apps"
click_topic = "area.app_launcher.open"
click_instance = "web_1"

[[areas.plugins]]
id = "button_weather"
path = "target/release/libsmearor_button_widget.so"
text = "Weather"
icon = "nf-md-weather-partly-cloudy"
click_topic = "area.weather.open"
click_instance = "main"
```

### 9.2 Host Config

The HTTP server port is configured in the main launcher config or via CLI argument:

```toml
# config.toml (host-level)
[web_server]
port = 8080
enabled = true
```

Or via CLI: `--web-port 8080`

### 9.3 Config Validation

Web instance configs are validated using the same `validate_config_path()` / `validate_instance_id()` from DYNAMIC_LOAD. Additionally:

1. **Template path**: If `template_path` is set, it must exist and be within the allowlist.
2. **Port availability**: The HTTP server port must be available.
3. **Auth token**: If `auth_token` is set, all requests to instance routes must include the `Authorization: Bearer <token>` header.

---

## 10. Instance Lifecycle

### 10.1 Creation Flow

1. `load_instance("web_1", "config-web-1.toml", InstanceType::Web)` is called (via MCP tool, broker topic, or startup config).
2. `LauncherInstance` is created with `instance_type = InstanceType::Web`.
3. `WebInstanceMetadata` is parsed from the `[web]` section of the config and attached to the instance.
4. Plugins are loaded, areas are created.
5. No GTK window is built (same as Headless).
6. The HTTP server (already running) automatically serves the instance at `/instances/web_1/`.

### 10.2 Destruction Flow

1. `stop_instance("web_1")` is called.
2. All connected WebSockets for `web_1` are closed.
3. Plugins are unloaded, MCP tools are unregistered.
4. The instance is removed from the `instances` HashMap.
5. Requests to `/instances/web_1/` return `404 Not Found`.

### 10.3 Hot-Reload

`reload_instance("web_1", "config-web-1.toml")` preserves `InstanceType::Web`. Connected WebSockets are closed during stop and reconnected by clients after
reload.

---

## 11. Security Considerations

1. **Auth token**: Each web instance can set an `auth_token`. If set, all HTTP requests to instance routes must include `Authorization: Bearer <token>`. Static
   asset routes (`/static/*`) do not require auth.

2. **HTTPS**: For production use, a reverse proxy (nginx, Caddy) with TLS termination is recommended. The embedded server serves plain HTTP.

3. **Config path allowlist**: Template paths are validated against the same allowlist as instance configs (current working directory + `~/.config/smearor/`).

4. **Instance ID sanitization**: Same `validate_instance_id()` as DYNAMIC_LOAD — no colons, no path separators.

5. **Click payload validation**: Click payloads are limited to reasonable size (e.g. 4 KB) to prevent abuse.

6. **WebSocket origin check**: WebSocket connections are checked against the `Origin` header to prevent cross-site WebSocket hijacking.

7. **CORS**: By default, same-origin only. If cross-origin access is needed, CORS headers can be configured per instance.

---

## 12. MCP Server Tools

### 12.1 `launcher_load_instance` (Existing, Extended)

The existing `launcher_load_instance` MCP tool from DYNAMIC_LOAD already accepts `instance_type`. The `"web"` value is now supported:

```json
{
    "instance_id": "web_1",
    "config_path": "config-web-1.toml",
    "instance_type": "web"
}
```

### 12.2 `launcher_list_instances` (Existing, Extended)

The `instance_type` field in the response now includes `"web"`:

```json
[
    { "instance_id": "main", "instance_type": "gtk", "has_window": true },
    { "instance_id": "macropad_1", "instance_type": "headless", "has_window": false },
    { "instance_id": "web_1", "instance_type": "web", "has_window": false }
]
```

### 12.3 `web_server_status` (New)

| Property        | Value                                                                                           |
|-----------------|-------------------------------------------------------------------------------------------------|
| **Name**        | `web_server_status`                                                                             |
| **Description** | Returns the status of the embedded web server, including port and list of active web instances. |
| **Arguments**   | `{}`                                                                                            |
| **Returns**     | JSON object with `port`, `enabled`, and `instances` array                                       |

---

## 13. Implementation Phases

### Phase 1: Plugin-API Extension — `WebRenderer` Trait

**Order**: First. All other phases depend on the trait and FFI types.

**Changes**:

- Add `FfiString` struct to `plugin-api/src/widget.rs`.
- Add `WebRenderer` trait to `plugin-api/src/widget.rs`.
- Add `render_html: Option<...>` function pointer to `PluginVTable` in `plugin-api/src/plugin.rs`.
- Increment `PLUGIN_VTABLE_VERSION` to `3`.
- Export `WebRenderer`, `FfiString` from `plugin-api/src/lib.rs`.

**Exit Criteria**: Crate compiles, `WebRenderer` trait is exported, VTable has `render_html` field.

### Phase 2: Button Widget — `WebRenderer` Implementation

**Order**: After Phase 1.

**Changes**:

- Implement `WebRenderer` for `ButtonWidget` in `plugins/button/src/widget.rs`.
- Generate HTML fragment with `data-plugin-id`, `data-click-*` attributes, NerdFont icon class, label, state class.
- Export `render_html` function pointer in the plugin's VTable.

**Exit Criteria**: `render_html()` returns valid HTML fragment for a configured button, with correct data attributes and state classes.

### Phase 3: HTTP Server — Host Integration

**Order**: After Phase 2.

**Changes**:

- Add `axum`, `tokio` (with `net` feature), `tower` dependencies to `smearor-swipe-launcher/Cargo.toml`.
- Create `smearor-swipe-launcher/src/web/mod.rs` and `smearor-swipe-launcher/src/web/server.rs`.
- Implement `WebServer` struct with `axum` router.
- Add `web_server: Option<Arc<WebServer>>` field to `LauncherHost`.
- Initialize web server in `main.rs` if `web_server.enabled = true`.
- Implement routes: `serve_instance_page`, `serve_instance_css`, `serve_instance_js`, `serve_nerdfont_css`, `serve_nerdfont_font`, `handle_click`.
- Implement page composition: collect `render_html()` fragments, load template, replace placeholders.
- Add `WebInstanceMetadata` struct and `web_metadata: Option<WebInstanceMetadata>` field to `LauncherInstance`.
- Parse `[web]` section from instance config during `load_instance()`.
- Add `InstanceType::Web` to `model/instance-control`.
- Add `"web"` to `launcher_load_instance` MCP tool `instance_type` enum.

**Exit Criteria**: `GET /instances/web_1/` returns a full HTML page with button fragments; `POST /instances/web_1/click/button_apps` triggers the configured
action.

### Phase 4: WebSocket — Real-Time State Updates

**Order**: After Phase 3.

**Changes**:

- Implement WebSocket handler at `/instances/{id}/ws`.
- Maintain `websockets: HashMap<String, Vec<WebSocketSender>>` in `WebServer`.
- On plugin state update: call `render_html()`, send JSON update message via WebSocket.
- Client-side JavaScript in `app.js`: connect WebSocket, replace DOM elements on update messages.
- Close WebSockets on `stop_instance()`.

**Exit Criteria**: State change in a plugin is reflected in the browser within 1 second without page reload.

### Phase 5: Static Assets and Templates

**Order**: After Phase 3.

**Changes**:

- Create `resources/web/template-default.html`.
- Create `resources/web/style.css` with base styles.
- Create `resources/web/app.js` with click handling and WebSocket logic.
- Create `resources/web/nerdfont.css` mapping `nf-md-*` classes to Unicode codepoints.
- Generate `resources/web/nerdfont.woff2` from `resources/NerdFontsSymbolsOnly/`.
- Implement custom template loading and validation.
- Serve static assets via axum routes.

**Exit Criteria**: Default template renders correctly in a browser, buttons are styled, NerdFont icons display, clicks work, WebSocket updates work.

### Phase 6: Security and Polish

**Order**: After Phase 4 and Phase 5.

**Changes**:

- Implement auth token middleware: check `Authorization: Bearer <token>` header if `auth_token` is set.
- Implement WebSocket origin check.
- Add click payload size limit (4 KB).
- Add `web_server_status` MCP tool.
- Config validation: template path exists and is within allowlist.
- Integration tests: load web instance, verify page, click, state update, stop instance.
- Documentation: README section for web instance setup.

**Exit Criteria**: Auth token enforced, WebSocket origin checked, all integration tests pass.

---

## 14. File Changes Summary

| File                                        | Change                                                                                        |
|---------------------------------------------|-----------------------------------------------------------------------------------------------|
| `plugin-api/src/widget.rs`                  | Add `FfiString` struct, `WebRenderer` trait                                                   |
| `plugin-api/src/plugin.rs`                  | Add `render_html` to `PluginVTable`, increment `PLUGIN_VTABLE_VERSION` to `3`                 |
| `plugin-api/src/lib.rs`                     | Export `WebRenderer`, `FfiString`                                                             |
| `plugins/button/src/widget.rs`              | Implement `WebRenderer` for `ButtonWidget`, export `render_html` in VTable                    |
| `smearor-swipe-launcher/src/web/mod.rs`     | **New** — web module                                                                          |
| `smearor-swipe-launcher/src/web/server.rs`  | **New** — `WebServer` struct, axum router, route handlers                                     |
| `smearor-swipe-launcher/src/instance.rs`    | Add `web_metadata: Option<WebInstanceMetadata>` field                                         |
| `smearor-swipe-launcher/src/application.rs` | Add `web_server: Option<Arc<WebServer>>` field, `route_web_click()` method                    |
| `smearor-swipe-launcher/src/main.rs`        | Initialize web server if enabled, parse `[web_server]` config                                 |
| `smearor-swipe-launcher/Cargo.toml`         | Add `axum`, `tower` dependencies                                                              |
| `model/instance-control/src/lib.rs`         | Add `InstanceType::Web` variant                                                               |
| `mcp-server/src/tools.rs`                   | Add `"web"` to `instance_type` enum in `launcher_load_instance`, add `web_server_status` tool |
| `resources/web/template-default.html`       | **New** — default HTML template                                                               |
| `resources/web/style.css`                   | **New** — base CSS styles                                                                     |
| `resources/web/app.js`                      | **New** — client-side JavaScript                                                              |
| `resources/web/nerdfont.css`                | **New** — NerdFont CSS mapping                                                                |
| `resources/web/nerdfont.woff2`              | **New** — NerdFont web font                                                                   |
| `Cargo.toml` (workspace)                    | Add `axum`, `tower` to workspace dependencies                                                 |

---

## 15. Dependencies

### New Workspace Dependencies

```toml
axum = "0.8"
tower = "0.5"
```

### Per-Crate

| Crate                    | Additional Dependencies                                         |
|--------------------------|-----------------------------------------------------------------|
| `plugin-api`             | No new dependencies (`FfiString` is pure Rust)                  |
| `plugins/button`         | No new dependencies (HTML generation is string formatting)      |
| `smearor-swipe-launcher` | `axum`, `tower`, `tokio` (with `net` feature)                   |
| `model/instance-control` | No new dependencies (`InstanceType::Web` is a new enum variant) |
| `mcp-server`             | No new dependencies                                             |

---

## 16. Open Questions

1. **Multiple web instances on different ports**: Should each web instance be able to specify its own port, or do all web instances share a single port
   (differentiated by URL path)?
2. **Area layout in web**: Should the web instance render areas as a vertical stack, horizontal row, or configurable grid? Should the area config `width` field
   be repurposed as a CSS hint?
3. **Touch/scroll support**: Should the web instance support touch events and scroll areas (like the GTK Scroll area type), or only fixed grids?
4. **Session management**: Should the web instance support user sessions (login/logout), or is the auth token sufficient?
5. **Partial vs. full updates**: Should WebSocket updates always be partial (per-plugin), or should there be a "full page reload" message type for cases where
   the layout changes?
6. **Plugin discovery**: Should the web instance page auto-discover all plugins in all areas, or should there be a configurable list of which plugins to expose
   via web?

---

## 17. References

- **Dynamic Load concept**: `concepts/DYNAMIC_LOAD_LAUNCHER_INSTANCE.md` — defines `InstanceType`, `load_instance()`, `stop_instance()`.
- **Instance Types comparison**: `concepts/INSTANCE_TYPES.md` — compares Gtk, Headless, and Web instance types.
- **MacroPad concept**: `concepts/STREAMDECK_CONCEPT.md` — defines `GraphicRenderer` trait and Headless instance pattern.
