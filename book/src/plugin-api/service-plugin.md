# Developing Service Plugins

This guide walks through creating a new service plugin from scratch.

## 1. Create the Crate

```bash
cargo new --lib services/my-service
```

Edit `Cargo.toml`:

```toml
[package]
name = "smearor-my-service"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
stabby = { workspace = true }
paste = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
smearor-model-mcp = { path = "../../model/mcp" }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

Add the crate to the workspace `Cargo.toml`.

## 2. Implement the Service

Create `src/service.rs`:

```rust
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use tokio::sync::mpsc;

pub struct MyService {
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    command_sender: Option<mpsc::UnboundedSender<String>>,
}

impl MyService {
    pub fn new() -> Self {
        MyService {
            meta: PluginMeta {
                id: "my_service".to_string(),
                name: "My Service".to_string(),
                version: "0.1.0".to_string(),
            },
            core_context: None,
            command_sender: None,
        }
    }
}

impl PluginMetaGetter for MyService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for MyService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl MessageBroadcaster for MyService {}

impl ServicePlugin for MyService {}
```

## 3. Handle Messages

Implement `MessageHandler` for the message types your service should receive:

```rust
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<MyCommand>> for MyService {
    fn handle_message(&self, message: FfiEnvelopePayload<MyCommand>, _sender_id: &str) {
        // Process the command
        // Broadcast a status update
        self.broadcast_message("service.my_service.status", &MyStatus { ok: true });
    }
}
```

## 4. Async Task Spawning

Services use `tokio::sync::mpsc` for message reception and spawn async tasks via `PluginExecutor`:

```rust
impl MyService {
    pub fn start(&self) {
        if let Some(ctx) = &self.core_context {
            let (tx, mut rx) = mpsc::unbounded_channel::<String>();
            self.command_sender = Some(tx);

            ctx.executor.spawn(async move {
                while let Some(cmd) = rx.recv().await {
                    // Process command asynchronously
                }
            });
        }
    }
}
```

> **Important:** Use `tokio::sync::mpsc`, not `std::sync::mpsc`. Polling loops (`timeout_add_local`) are forbidden in services.

## 5. Export the Plugin

Create `src/lib.rs`:

```rust
mod service;

use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(MyService);
```

## 6. Configure

Add the service to `configs/services/services.toml`:

```toml
[[services]]
id = "my_service"
path = "target/release/libsmearor_my_service.so"
```

## 7. Build and Test

```bash
cargo build --release
# Restart the launcher
```

## Service Lifecycle

1. **Load** — `ServiceManager::load_service()` opens the `.so`, calls the constructor
2. **Start** — `service.start()` is called immediately after loading
3. **Message Handling** — `service.on_message(envelope)` routes incoming messages
4. **Unload** — `service.destroy()` is called (library is leaked to avoid unloading active async tasks)

Services are **shared across all instances** — loaded once by the `ServiceManager`, not per-instance.

See the [weather service](../services/weather.md) and [audio service](../services/audio.md) for complete examples.
