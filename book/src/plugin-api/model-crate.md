# Developing Model Crates

Model crates define the shared data types that are exchanged between widget plugins, service plugins, and the launcher core via the FFI boundary.

## Purpose

When a widget and a service need to communicate, they share a model crate that defines:

- **Message structs** — Typed payloads carried in `FfiEnvelope`
- **Action enums** — Enumerations of actions a service can perform
- **Topic constants** — Static topic strings for message routing
- **JSON converters** — Deserialization from generic JSON strings to typed messages

## 1. Create the Crate

```bash
cargo new --lib model/my-model
```

Edit `Cargo.toml`:

```toml
[package]
name = "smearor-my-model"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
stabby = { workspace = true, features = ["serde"] }
serde = { workspace = true }
serde_json = { workspace = true }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
```

Add to workspace `Cargo.toml`.

## 2. Define Types

Create `src/lib.rs`:

```rust
use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for my command messages.
pub const TOPIC_MY_COMMAND: &str = "service.my_service.command";

/// Topic for my status messages.
pub const TOPIC_MY_STATUS: &str = "service.my_service.status";

/// Command message sent from widgets to the service.
#[stabby::stabby]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MyCommand {
    pub action: MyAction,
}

/// Actions the service can perform.
#[stabby::stabby]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum MyAction {
    #[default]
    DoSomething,
    DoSomethingElse,
}

/// Status message broadcast from the service to all widgets.
#[stabby::stabby]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MyStatus {
    pub ok: bool,
}

impl TypedMessage for MyCommand {
    const TYPE_ID: u64 = generate_type_id("MyCommand");
}

impl TypedMessage for MyStatus {
    const TYPE_ID: u64 = generate_type_id("MyStatus");
}

impl MessageTopic for MyCommand {
    fn topic() -> &'static str {
        TOPIC_MY_COMMAND
    }
}

impl MessageTopic for MyStatus {
    fn topic() -> &'static str {
        TOPIC_MY_STATUS
    }
}
```

## 3. JSON Converters

Register JSON converters so generic widgets (like `button`) can send typed messages:

```rust
use smearor_swipe_launcher_plugin_api::JsonConvertible;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(
    MyCommandConverter,
    MyCommand,
    |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default()
);

impl_json_convertible!(
    MyStatusConverter,
    MyStatus,
    |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default()
);

/// Register all JSON converters for this model crate.
pub fn register_json_converters(context: &smearor_swipe_launcher_plugin_api::FfiCoreContext) {
    MyCommandConverter::register_in_host(context);
    MyStatusConverter::register_in_host(context);
}
```

## Rules

- All message types must derive `Serialize, Deserialize` from `serde`
- FFI-relevant types must carry `#[stabby::stabby]`
- The `stabby` dependency must include the `serde` feature
- JSON converters must use `impl_json_convertible!` with `serde_json::from_value(json).unwrap_or_default()`
- Structs used as deserialization fallbacks must also derive `Default`
- Manual `parse_*` functions in `json_converters.rs` are forbidden

See [model/audio](https://github.com/smearor/smearor-swipe-launcher/tree/main/model/audio)
and [model/weather](https://github.com/smearor/smearor-swipe-launcher/tree/main/model/weather) for complete examples.
