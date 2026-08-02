# Developing Widget Plugins

This guide walks through creating a new widget plugin from scratch.

## 1. Create the Crate

```bash
cargo new --lib plugins/my-widget
```

Edit `Cargo.toml`:

```toml
[package]
name = "smearor-my-widget"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
stabby = { workspace = true }
gtk4 = { workspace = true, features = ["unsafe-assume-initialized"] }
glib = { workspace = true }
paste = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
smearor-model-widget = { path = "../../model/widget" }
smearor-model-mcp = { path = "../../model/mcp" }
thiserror = { workspace = true }
tracing = { workspace = true }
typed-builder = { workspace = true }
```

Add the crate to the workspace `Cargo.toml`:

```toml
members = [
    # ...
    "plugins/my-widget",
]
```

## 2. Implement the Config

Create `src/config.rs`:

```rust
use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MyWidgetConfig {
    pub main_text: String,
    pub icon: Option<String>,
    pub icon_size: i32,
    #[serde(flatten)]
    pub action_bindings: ActionBindings,
}
```

## 3. Implement the Widget

Create `src/widget.rs`:

```rust
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::Rotation;

pub struct MyWidget {
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    config: crate::config::MyWidgetConfig,
}

impl MyWidget {
    pub fn new(config: crate::config::MyWidgetConfig) -> Self {
        MyWidget {
            meta: PluginMeta {
                id: "my_widget".to_string(),
                name: "My Widget".to_string(),
                version: "0.1.0".to_string(),
            },
            core_context: None,
            config,
        }
    }
}

impl PluginMetaGetter for MyWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for MyWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl MessageBroadcaster for MyWidget {}

impl WidgetBuilder for MyWidget {
    fn build_widget(&self, _rotation: Rotation) -> gtk4::Widget {
        let button = gtk4::Button::with_label(&self.config.main_text);
        button.into()
    }
}

impl WidgetPlugin for MyWidget {}
```

## 4. Export the Plugin

Create `src/lib.rs`:

```rust
mod config;
mod widget;

use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(MyWidget);
```

## 5. Configure in config.toml

```toml
[scroll_band]
plugins = [
    { id = "my_widget", path = "target/release/libsmearor_my_widget.so" }
]

[my_widget]
main_text = "Hello World"
icon = "nf-md-star"
icon_size = 32
click_topic = "area.open"
click_payload = { area_id = "my_area" }
```

## 6. Build and Test

```bash
cargo build --release
# Restart the launcher
```

## Optional: Headless Rendering

To support MacroPad instances, implement `GraphicRenderer`:

```rust
impl GraphicRenderer for MyWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        // Render widget to RGBA pixel buffer using image + ab_glyph
        // ...
    }
}
```

## Optional: Web Rendering

To support web instances, implement `WebRenderer`:

```rust
impl WebRenderer for MyWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> FfiHtmlString {
        FfiHtmlString::from(format!(
            "<div class='widget'><span>{}</span></div>",
            self.config.main_text
        ))
    }
}
```

## Optional: MCP Tool Registration

To expose an MCP tool, send a `RegisterToolMessage` during initialization:

```rust
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

// In your widget's start() or init:
self.broadcast_message(
    "mcp.register.tool",
    &RegisterToolMessage::new("my_widget_action", "Does something useful", "{}"),
);
```

Then handle `InvokeToolMessage` in your `MessageHandler` implementation.

See the [clock plugin](../plugins/clock.md) and [button plugin](../plugins/button.md) for complete examples.
