# Concept Skill — How to Write a Complete Feature Concept

This guide describes how to create a comprehensive concept document for a new feature in the **Smearor Swipe Launcher**. A concept must cover all architectural
layers, patterns, and conventions used in the codebase so that implementation can proceed without ambiguity.

**Reference concept**: `concepts/planned/BLUETOOTH_CONCEPT.md` — study it as a concrete example of this guide.

---

## 1. Document Structure

A concept document must contain the following sections, in order:

1. **Title & Motivation** — What problem does this feature solve? Why does it need dedicated crates?
2. **Crate Structure** — Table listing Model, Service, and Widget crates with paths and responsibilities
3. **Model Crate** — Message topics, status/command structs, enums, FFI types, JSON converters
4. **Service Crate** — D-Bus/system integration, service struct, async loop, config, MCP tools
5. **Widget Crate** — Widget struct, config, view rendering, click/gesture handling, instance types
6. **Cross-Service Coordination** — Message-based coordination with other services (if applicable)
7. **Config Integration** — TOML examples for service config, widget config, and area definitions
8. **Implementation Phases** — Ordered phases with dependencies, tasks, and exit criteria
9. **Dependencies** — Crate-level dependency table
10. **Testing & Verification** — Test tasks covering all functional paths and edge cases
11. **Personalization Integration** — Locale-aware labels via `PersonalizationStatusMessage`
12. **Future Enhancements** — Features explicitly out of scope for the initial implementation

---

## 2. Crate Architecture

### 2.1 Three-Crate Pattern

Every feature that has both a service (business logic) and a widget (UI) must be split into three crates:

| Crate Type  | Path Pattern       | Responsibility                                          |
|-------------|--------------------|---------------------------------------------------------|
| **Model**   | `model/<name>/`    | Shared structs, enums, message formats, FFI types       |
| **Service** | `services/<name>/` | System integration, status broadcasts, command handling |
| **Widget**  | `plugins/<name>/`  | GTK4 tile widget with view-based rotation               |

If the feature only needs a widget (no background service), the model crate may be omitted and the widget can use existing model crates. If the feature only
needs a service (no UI), the widget crate may be omitted.

### 2.2 Model Crate (`model/<name>`)

#### 2.2.1 Message Topics

Define topic constants for all messages the service broadcasts or commands it receives:

```rust
pub const TOPIC_STATUS: &str = "service.<name>.status";
pub const TOPIC_COMMAND: &str = "service.<name>.command";
```

Topic naming convention: `service.<name>.<message_type>` for status/events, `service.<name>.command` for commands.

#### 2.2.2 Status Message Struct

The status message is broadcast by the service and consumed by the widget. It must:

- Derive `Clone, Debug, Default, Deserialize, Serialize`
- Carry `#[stabby::stabby]` for FFI compatibility
- Use `StabbyString` instead of `String`, `StabbyVec` instead of `Vec`, `StabbyOption` instead of `Option`
- Use `stabby::string::String::from()` for conversions

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct MyStatusMessage {
    pub powered: bool,
    pub name: StabbyString,
    pub last_updated: StabbyString,
}
```

#### 2.2.3 Command Message Struct

Commands sent to the service. Must carry `#[stabby::stabby]` and use stabby types:

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct MyCommandMessage {
    pub action: MyCommandAction,
    pub address: StabbyOption<StabbyString>,
    pub enabled: bool,
}
```

Document the semantics of each field per action variant in the struct's doc comments.

#### 2.2.4 Command Action Enum

```rust
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum MyCommandAction {
    #[default]
    TogglePower,
    StartScan,
    // ...
}
```

#### 2.2.5 View Enum (Widget)

Defines which data categories the widget can display. Each variant corresponds to a rendered tile:

```rust
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum MyView {
    #[default]
    PowerStatus,
    ConnectedDevices,
    // ...
}
```

#### 2.2.6 Device/Item Type Enum

If the feature deals with categorized external entities (e.g. device types), define an enum with a `from_<source>()` mapping method:

```rust
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum MyDeviceType {
    #[default]
    Unknown,
    AudioHeadphones,
    // ...
}

impl MyDeviceType {
    pub fn from_bluez_icon(icon: &str) -> Self {
        match icon {
            "audio-headphones" | "audio-headset" => Self::AudioHeadphones,
            // ...
            _ => Self::Unknown,
        }
    }
}
```

Use this enum for type checking instead of raw string matching — it normalizes variant names from external sources.

#### 2.2.7 JSON Converters

All FFI-relevant message types must register JSON converters in `lib.rs` using the `impl_json_convertible!` macro. **Manual `parse_*` functions are forbidden.**

```rust
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(MyStatusMessageConverter, MyStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

pub fn register_json_converters(core_context: Option<FfiCoreContext>) {
    MyStatusMessageConverter::register_in_host(core_context);
    MyCommandMessageConverter::register_in_host(core_context);
}
```

All structs used as deserialization fallbacks must derive `Default`.

#### 2.2.8 Model Crate `Cargo.toml`

```toml
[package]
name = "smearor_<name>_model"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
stabby = { workspace = true, features = ["serde"] }
smearor_swipe_launcher_plugin_api = { path = "../../plugin-api" }
```

The `stabby` dependency must include the `serde` feature.

### 2.3 Service Crate (`services/<name>`)

#### 2.3.1 Service Struct

The service struct must implement these traits:

- `ServicePlugin` — provides `on_message` and `start`
- `MessageHandler<FfiEnvelopePayload<MyCommandMessage>>` — dispatches commands
- `MessageBroadcaster` — empty impl for broadcasting
- `MessageTopicBroadcaster<MyStatusMessage>` — empty impl for typed broadcasting
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`

```rust
pub struct MyService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: MyServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<MyCommand>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<MyCommand>>,
    pub shared_state: Arc<Mutex<MySharedState>>,
}
```

#### 2.3.2 `lib.rs`

```rust
pub(crate) mod config;
pub(crate) mod dbus; // or system integration module
pub(crate) mod service;

use crate::service::MyService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(MyService);
```

#### 2.3.3 `start()` Method

The `start` method spawns a thread with `tokio::runtime::Builder::new_current_thread().enable_all()` + `LocalSet`:

```rust
fn start(&mut self) {
    if let Some(ctx) = &self.core_context {
        let meta = self.meta.clone();
        let core_context = *ctx;
        let command_receiver = self.command_receiver.take();
        let config = self.config.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
            // ... error handling ...
            let local_set = tokio::task::LocalSet::new();
            local_set.block_on(&rt, async move {
                if let Some(receiver) = command_receiver {
                    run_my_async(meta, core_context, receiver, config).await;
                }
            });
        });
    }
}
```

#### 2.3.4 Async Loop

The async loop uses `tokio::select!` with:

- Command channel receiver
- System signal streams (e.g. D-Bus `PropertiesChanged`)
- Fallback interval (e.g. every 30s)

**Critical**: Subscribe to signal streams **before** the initial state fetch to avoid race conditions. Use `retry_proxy()` for D-Bus proxy creation with
explicit error logging.

**Mutex discipline**: Execute async system calls (D-Bus, file I/O) **outside** the `shared_state` mutex lock. Acquire the lock only briefly for writing and
building the status message. Broadcast outside the lock.

#### 2.3.5 Service Config

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MyServiceConfig {
    #[serde(default = "default_true")]
    pub enable_scanning: bool,
    #[serde(default)]
    pub automation: Vec<MyAutomationRule>,
}
```

Use `#[serde(default)]` and `#[serde(default = "fn_name")]` for all fields so partial TOML configs work.

#### 2.3.6 MCP Tools

If the service exposes MCP tools, the implementation follows a three-part pattern: a tools enum in the **model crate**, tool registration during `start()`, and
an `InvokeToolMessage` handler in the **service crate**.

##### Step 1: MCP Tools Enum (Model Crate)

Define a tools enum in `model/<name>/src/mcp_tools.rs` with `AsRef<str>`, `FromStr`, and `Display` implementations. This replaces raw string matching in the
handler and provides type-safe tool dispatch. The enum uses `UnknownToolError` from `smearor-model-mcp`.

```rust
use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the <name> service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MyFeatureMcpTools {
    /// Description of what the tool does.
    DoSomething,
    /// Another tool.
    DoSomethingElse,
}

impl AsRef<str> for MyFeatureMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::DoSomething => "my_feature_do_something",
            Self::DoSomethingElse => "my_feature_do_something_else",
        }
    }
}

impl FromStr for MyFeatureMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "my_feature_do_something" => Ok(Self::DoSomething),
            "my_feature_do_something_else" => Ok(Self::DoSomethingElse),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for MyFeatureMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
```

Add `smearor-model-mcp` as a dependency in the model crate's `Cargo.toml`:

```toml
smearor-model-mcp = { path = "../../model/mcp" }
```

Export the enum from `lib.rs`:

```rust
mod mcp_tools;
pub use mcp_tools::MyFeatureMcpTools;
```

##### Step 2: Tool Registration (Service Crate)

Implement `McpCapabilitiesRegistrator` and register tools during `start()` using `RegisterToolMessage`:

```rust
let tool = RegisterToolMessage::new(
    "my_feature_do_something",
    "Human-readable description of the tool.",
    r#"{ "type": "object", "properties": { "param": { "type": "string", "description": "Parameter description" } }, "required": ["param"] }"#,
);
broadcaster.broadcast_message_to_topic(tool);
```

##### Step 3: InvokeToolMessage Handler (Service Crate)

Implement `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` in the service crate. The handler parses the tool name via `from_str`, returns an
`InvokeToolError` response for unknown tools, then dispatches on the enum variants:

```rust
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for MyFeatureService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("MyFeature Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        let tool = match MyFeatureMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(
                    InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id))
                );
                return;
            }
        };

        match tool {
            MyFeatureMcpTools::DoSomething => {
                // Parse arguments with explicit error handling — do not use
                // unwrap_or(Null) which silently swallows malformed JSON.
                let args_result = serde_json::from_str::<serde_json::Value>(
                    &message.0.arguments.to_string()
                );
                match args_result {
                    Ok(args) => {
                        // Extract parameters, validate required fields, execute tool
                        let response = InvokeToolResponse::success(&correlation_id, "Done");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    Err(parse_error) => {
                        debug!("MyFeature Service: argument parse error: {parse_error}");
                        let response = InvokeToolResponse::error(
                            &correlation_id,
                            &format!("Invalid arguments: {parse_error}"),
                        );
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            MyFeatureMcpTools::DoSomethingElse => {
                let response = InvokeToolResponse::success(&correlation_id, "Done");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
```

**Key conventions:**

- **Never use string matching** (`match tool_name.as_str() { "my_tool" => ... }`) — always use the `FromStr` enum pattern
- **Extract `correlation_id` once** at the top of the handler and use it in all responses
- **Parse JSON arguments explicitly** — return `InvokeToolResponse::error` on parse failure instead of silently falling back to `Value::Null`
- **Validate required parameters** — return `InvokeToolResponse::error` with a descriptive message if a required field is missing
- **Use `debug!` for logging** tool invocations and parse errors
- **Reference implementations**: `services/weather/src/mcp/handler/tools.rs`, `services/app-launcher/src/mcp/handler/tools.rs`

### 2.4 Widget Crate (`plugins/<name>`)

#### 2.4.1 Widget Struct

The widget struct must implement:

- `WidgetPlugin` (extends `PluginMetaGetter` + `WidgetBuilder`) — provides `on_message` and `start`
- `MessageHandler<FfiEnvelopePayload<MyStatusMessage>>` — handles status updates
- `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` — handles locale
- `MessageBroadcaster` — for broadcasting commands
- `MessageTopicBroadcaster<MyCommandMessage>` — for typed command broadcasting
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`
- `AcceptTopic<FfiEnvelope>` — filters relevant topics in `on_message`
- `GestureHandler` — provides `attach_gesture_handlers` and `DefaultFallback`
- `GraphicRenderer` — for headless instance pixel rendering
- `WebRenderer` — for web instance HTML rendering

Use `Rc<RefCell<...>>` for interior mutability and `glib::clone!` for closure ownership.

#### 2.4.2 `lib.rs`

Use `widget_plugin_graphic!` if the widget supports all three instance types (GTK, Headless, Web):

```rust
pub mod config;
pub mod graphic;
pub mod widget;

use crate::widget::MyWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin_graphic;

widget_plugin_graphic!(MyWidget);
```

Use `widget_plugin!` if only GTK is needed (no `render_graphic` / `render_html`).

#### 2.4.3 Widget Config

Use shared config structs via `#[serde(flatten)]`:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MyWidgetConfig {
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    #[serde(flatten)]
    pub layout: WidgetLayout,
    #[serde(flatten)]
    pub icon: WidgetIcon,
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    #[serde(flatten)]
    pub mode: WidgetMode,
    #[serde(flatten)]
    pub actions: ActionBindings,
    #[serde(flatten)]
    pub icons: MyIcons, // feature-specific icon fields
    pub views: Vec<MyView>,
}
```

Shared config structs from `plugin-api`:

- `WidgetDimensions` — `width`, `height`, `max_width`
- `WidgetLayout` — layout-related fields
- `WidgetIcon` — `icon_size`, `icon_only`
- `WidgetTextColors` — `main_text_color`, `info_text_color`
- `WidgetMode` — `compact` or `wide` layout
- `ActionBindings` — `click`, `longpress`, `drag_up`, `drag_down`, `drag_left`, `drag_right`, `scroll` with `BindingMode` (`replace` or `supplement`)

#### 2.4.4 View Rendering

`render_view` returns a `ViewData` struct:

```rust
fn render_view(
    view: MyView,
    status: &MyStatusMessage,
    config: &MyWidgetConfig,
    labels: &MyLabel,
) -> ViewData {
    match view {
        MyView::PowerStatus => {
            let icon = if status.powered { &config.icons.icon_on } else { &config.icons.icon_off };
            let info = if status.powered { &labels.on } else { &labels.off };
            ViewData::new(icon, &status.name.to_string(), info)
        }
        // ...
    }
}
```

`ViewData` fields:

- `icon_name: String` — Nerd Font icon name (e.g. `nf-md-bluetooth`)
- `main_text: String` — primary text line
- `info_text: String` — secondary text line
- `icon_color: Option<Color>` — optional semantic icon color
- `is_error: bool` — error/loading state

#### 2.4.5 Icon Handling

- Define a feature-specific icons struct (e.g. `MyIcons`) with `Default` impl
- Use `#[serde(flatten)]` to embed it in the widget config
- Icon names are Nerd Font names (e.g. `nf-md-bluetooth`, `nf-fa-music`)
- GTK resolves icons via `resolve_gtk_nerd_icon()` → GResource SVG paths
- Pixel/atomic rendering resolves via `resolve_icon_codepoint()` → Unicode codepoints
- For state-dependent icons, select the icon in `render_view` based on status data
- For `StabbyOption` values, use explicit `match` statements, not `.map().unwrap_or()`

#### 2.4.6 Unified 4-Line Layout

All GTK widgets use the same vertical structure:

| Line | Height      | Content            |
|------|-------------|--------------------|
| 0    | `icon_size` | Icon               |
| 1    | 20px        | `widget-main-text` |
| 2    | 16px        | `widget-info-text` |
| 3    | 16px        | spacer/bar         |

In Compact mode with `icon_only = true`, lines 1–3 are empty but retain height for alignment.

#### 2.4.7 Gesture Handling

Use the centralized `attach_gesture_handlers` trait method:

```rust
widget_self.attach_gesture_handlers(
    &button_widget,
    &config.actions,
    &broadcaster,
    &GestureHandlersConfiguration::default(),
);
```

Implement `DefaultFallback` for view-dependent click/longpress behavior when no `ActionBindings` are configured. Document the fallback table in the concept.

Swipe up/down cycles through `config.views` via `next_view()`/`prev_view()`.

#### 2.4.8 Instance Type Support

All three instance types (GTK, Headless, Web) share the same `render_view` function:

- **GTK** (`InstanceType::Gtk`): `WidgetBuilder::build_widget()` → `gtk4::Box` with icon, labels, gesture handlers. Icons via `resolve_gtk_nerd_icon()`.
- **Headless** (`InstanceType::Headless`): `GraphicRenderer::render_graphic(w, h)` → RGBA pixel buffer via `image` + `ab_glyph`. Icons via
  `resolve_icon_codepoint()`.
- **Web** (`InstanceType::Web`): `WebRenderer::render_html(instance_id, plugin_id)` → HTML fragment with inline styles.

After every UI update, broadcast `WidgetUpdateMessage` so headless/Web instances can re-render.

#### 2.4.9 GTK Updates

Use `glib::MainContext::default().spawn_local` for all GTK updates from async message handlers. **Polling loops (`timeout_add_local`) are forbidden** — use
event-driven `recv().await` via `tokio::sync::mpsc`.

#### 2.4.10 Personalization

Subscribe to `TOPIC_PERSONALIZATION_STATUS` for locale-aware labels. Implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` and
`AcceptTopic<FfiEnvelope>` filtering.

Define a `MyLabel` struct with a `from_personalization(p: Option<&PersonalizationStatusMessage>)` constructor, analogous to `NetworkLabel`.

---

## 3. Cross-Service Coordination

When two services need to coordinate (e.g. Airplane Mode toggling both Network and Bluetooth), describe:

1. **Which service broadcasts** and on which topic
2. **Which service subscribes** and what action it takes
3. **Message format** — which command message struct is used
4. **Direction** — one-directional (broadcast → react) or bidirectional

Use the message system exclusively — no direct function calls between services.

Example pattern:

1. Widget detects user action, broadcasts `CommandMessage` on `service.<other>.command`
2. Other service handles command via `MessageHandler`, performs system action
3. Other service broadcasts `StatusMessage` on `service.<other>.status`
4. Widget subscribes to status, updates UI

---

## 4. Config Integration

Provide concrete TOML examples for all three config levels:

### 4.1 Service Config (`configs/services/<name>.toml`)

```toml
[services.<name>]
enable_scanning = true
max_devices = 15
```

### 4.2 Widget Config (in `config.toml` or area config)

```toml
[[plugins]]
plugin_id = "my_widget"
display_name = "My Feature"
icon_name = "nf-md-my-icon"
width = 100
height = 100
icon_size = 36
views = ["PowerStatus", "ConnectedDevices"]

[plugins.actions]
click_mode = "supplement"
# click = { topic = "service.my.command", payload = { ... } }
```

### 4.3 Area Config (if the feature has a scroll menu area)

```toml
[[areas]]
name = "my_area"
type = "scroll_menu"
# ...
```

---

## 5. Implementation Phases

Structure phases with explicit ordering, dependencies, tasks, and exit criteria:

### Phase 1: Model Crate

- Define message topics, status/command structs, enums
- Implement `register_json_converters`
- Add `#[stabby::stabby]` on all FFI-relevant types
- Add `stabby` with `serde` feature in `Cargo.toml`
- **Exit Criteria**: `cargo build -p smearor_<name>_model` succeeds

### Phase 2: Service Crate

- Implement service struct with all required traits
- Implement D-Bus/system integration
- Implement async loop with `tokio::select!`
- Implement config struct with `#[serde(default)]`
- **Exit Criteria**: `cargo build -p smearor_<name>_service` succeeds. Service loads and broadcasts status.

### Phase 3: Widget Crate

- Implement widget struct with all required traits
- Implement `config.rs` with shared config structs via `#[serde(flatten)]`
- Implement `render_view` for all view variants
- Implement `GraphicRenderer` and `WebRenderer`
- Implement gesture handling via `attach_gesture_handlers`
- **Exit Criteria**: `cargo build -p smearor_<name>_widget` succeeds. Widget displays status and responds to clicks.

### Phase 4: Cross-Service Coordination (if applicable)

- Update coordinating widgets/services to broadcast/listen
- **Exit Criteria**: Coordination works end-to-end.

### Phase 5: Workspace Wiring

- Add crates to workspace `Cargo.toml`
- Add service/plugin loading to launcher
- Add default config entries
- **Register in metapackage**: Add `smearor-plugin-<name>` and `smearor-service-<name>` to the `depends` list in `packages/full/Cargo.toml` so the full
  metapackage installs the new plugin/service automatically
- **Exit Criteria**: Launcher starts with the new feature loaded. Metapackage includes the new crates.

### Phase 6: Integration and Tests

- List all test tasks covering functional paths and edge cases
- Include graceful degradation tests (no hardware, adapter off, etc.)
- **Exit Criteria**: All tests pass.

### Phase 7: Documentation

- Update `book/src/SUMMARY.md` with new sections
- Create `book/src/features/<name>.md`
- Create `book/src/architecture/<name>.md`
- Add config examples to book
- Update `README.md` feature list
- **Exit Criteria**: `mdbook build` succeeds. README lists the feature.

---

## 6. Dependencies Table

List crate-level dependencies:

| Crate             | Dependencies                                                          |
|-------------------|-----------------------------------------------------------------------|
| `model/<name>`    | `stabby` (with `serde` feature), `serde`, `serde_json`, `plugin-api`  |
| `services/<name>` | `zbus` (if D-Bus), `tokio`, `tracing`, `plugin-api`, `model/<name>`   |
| `plugins/<name>`  | `gtk4`, `glib`, `plugin-api`, `model/<name>`, `model/personalization` |

---

## 7. Testing Checklist

Cover at least:

- Core functionality (toggle, scan, connect/disconnect)
- Signal-driven updates (properties changed, interfaces added/removed)
- Graceful degradation (no hardware, adapter off, service unavailable)
- Fallback interval triggers refresh when signals missed
- Irrelevant signals filtered (no unnecessary refreshes)
- View rotation (swipe up/down)
- Click and long-press fallback behavior
- Cross-service coordination (if applicable)
- Config parsing with partial TOML (defaults applied)
- No `unwrap()` or `expect()` in production code paths

---

## 8. Rust Code Standards

- **Edition 2024**
- `snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants
- No `unwrap()`, `expect()`, or `panic!()` in production code
- Use `Result<T, E>` for recoverable errors, `Option<T>` for absent values
- Use `thiserror` for internal errors, `miette` for user-facing errors
- Use `tokio::sync::mpsc` (not `std::sync::mpsc`) for message channels
- Use `glib::MainContext::default().spawn_local` for GTK updates
- No polling loops (`timeout_add_local`); use event-driven `recv().await`
- All `use` statements: one per line, alphabetical, no star imports (except preludes)
- `crate::` imports first, then external crates, then `std::`
- Use `debug!` instead of `tracing::debug!` with proper imports
- Run `cargo fmt` before committing
- One struct/enum per file in model crates
- All public enums and structs must have rustdoc comments
- All enum variants and struct fields must be documented
- All comments in English

---

## 9. FFI Standards

- All FFI-relevant types must carry `#[stabby::stabby]`
- Use `StabbyString` instead of `String` in FFI structs
- Use `StabbyVec` instead of `Vec` in FFI structs
- Use `StabbyOption` instead of `Option` in FFI structs
- For `StabbyOption` access, use explicit `match` statements:
  ```rust
  let level = match device.battery_level {
      stabby::option::StabbyOption::Some(level) => level,
      stabby::option::StabbyOption::None => 0,
  };
  ```
- All message types must derive `Serialize, Deserialize` from `serde`
- The `stabby` dependency must include the `serde` feature
- JSON converters must use `impl_json_convertible!` with `serde_json::from_value(json).unwrap_or_default()`
- Manual `parse_*` functions in `json_converters.rs` are forbidden
- Structs used as deserialization fallbacks must derive `Default`
- `register_json_converters(context)` function in `lib.rs` calls `Converter::register_in_host(context)` for each converter

---

## 10. Common Pitfalls to Document

When writing a concept, explicitly address these potential issues:

- **Race conditions**: Subscribe to event streams before initial state fetch
- **Silent failures**: Log errors explicitly when proxy/connection creation fails; use retry mechanisms
- **Mutex contention**: Execute async I/O outside mutex locks; acquire briefly only for writes
- **Signal filtering vs. active polling**: If a property is filtered in event handlers but needed for periodic checks, document the separation clearly
- **Security implications**: Auto-accept/auto-approve behaviors must be disabled by default and user-toggleable
- **Idle guards**: Skip periodic polling when the subsystem is inactive (powered off, no connections)
- **Type normalization**: Use enum mappings instead of raw string matching for external system values
- **StabbyOption vs Option**: Do not use `.map()/.unwrap_or()` on `StabbyOption`; use explicit `match`

---

## 11. Checklist Before Publishing a Concept

- [ ] All three crates documented (Model, Service, Widget) with struct definitions
- [ ] Message topics defined with naming convention
- [ ] All FFI types carry `#[stabby::stabby]` and use stabby types
- [ ] JSON converters use `impl_json_convertible!` macro
- [ ] Service implements all required traits (ServicePlugin, MessageHandler, MessageBroadcaster, PluginMetaGetter, AsRef)
- [ ] Widget implements all required traits (WidgetPlugin, MessageHandler, MessageBroadcaster, PluginMetaGetter, AsRef, GestureHandler, GraphicRenderer,
  WebRenderer, AcceptTopic)
- [ ] Widget config uses shared structs via `#[serde(flatten)]`
- [ ] `render_view` documented for all view variants
- [ ] Click and long-press fallback tables documented
- [ ] Instance type support (GTK, Headless, Web) documented
- [ ] Cross-service coordination described (if applicable)
- [ ] TOML config examples provided (service, widget, area)
- [ ] Implementation phases with ordering, dependencies, tasks, exit criteria
- [ ] New plugin/service registered in `packages/full/Cargo.toml` metapackage `depends` list
- [ ] Dependencies table
- [ ] Testing checklist with edge cases and graceful degradation
- [ ] Personalization integration documented
- [ ] Future enhancements listed
- [ ] No `unwrap()`, `expect()`, or `panic!()` in code examples
- [ ] All code examples follow import organization rules
- [ ] Common pitfalls addressed (race conditions, mutex discipline, security)
