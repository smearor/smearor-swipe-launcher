# Multi-Instance Concept for Smearor Swipe Launcher (Single-Process Multi-Window)

## 1. Goal

Enable running multiple independent Smearor Swipe Launcher instances from a **single process** / binary. Each instance uses its own configuration file and
renders its own window/layer surface.

## 1.1 Use Case: Smearor Smart Desk

A Table-Top Smart Desk has launchers on all four sides, each rotated towards the person sitting or standing in front of it. The short sides have less space than
the long sides. The user wants to open a menu in one window (e.g., for the person across the table) from a Swiper menu, Tokio, and the Message Broker.

## 2. Current Architecture Constraints

### 2.1 Single GTK Application ID

The current `main.rs` initializes exactly one `gtk4::Application` with a hard-coded `application_id`:

```rust
let gtk_app = Application::builder()
.application_id("com.smearor.swipe-launcher")
.build();
```

This is correct — GTK4 requires exactly one `Application` per process. Multiple `ApplicationWindow`s can be created from it.

### 2.2 `connect_activate` Fires Once

`build_ui` uses `gtk_app.connect_activate(...)`, which fires **once** per application lifecycle. The callback creates exactly one `ApplicationWindow`:

```rust
self .gtk_app.connect_activate( move | app| {
// ... creates one window
});
```

### 2.3 `LauncherApplication` Owns Everything

`LauncherApplication` currently owns:

- One `SwipeLauncherConfig`
- One `PluginManager` + `ServiceManager`
- One `AreaManager`
- One `UnboundedSender<FfiEnvelope>` / `UnboundedReceiver<FfiEnvelope>` pair
- One `gtk4::Application`

### 2.4 Process-Wide Global State

`context.rs` contains a process-wide static:

```rust
static GLOBAL_JSON_CONVERTER_REGISTRY: OnceLock<Arc<JsonConverterRegistry>> = OnceLock::new();
```

This is currently initialized once. All plugins register their JSON converters into the same global registry.

## 3. Recommended Solution: Single-Process Multi-Window

### 3.1 Core Idea

One `gtk4::Application`, multiple `ApplicationWindow`s. Each window is a self-contained **Launcher Instance** with:

- Its own `SwipeLauncherConfig`
- Its own `PluginManager` + `ServiceManager`
- Its own `AreaManager`
- Its own `UnboundedSender<FfiEnvelope>` / `UnboundedReceiver<FfiEnvelope>` pair
- Its own message routing loop

### 3.2 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Single Process                            │
│                                                                  │
│  ┌──────────────────┐                                           │
│  │ gtk4::Application │  ← One per process                       │
│  └────────┬─────────┘                                           │
│           │                                                      │
│     ┌─────┴─────┬─────────────┬─────────────┐                   │
│     │           │             │             │                   │
│  ┌──▼──┐    ┌──▼──┐      ┌──▼──┐      ┌──▼──┐                │
│  │Win 1│    │Win 2│      │Win 3│      │Win 4│                │
│  └──┬──┘    └──┬──┘      └──┬──┘      └──┬──┘                │
│     │          │             │             │                   │
│  ┌──▼──────────▼─────────────▼─────────────▼──┐               │
│  │          Launcher Instance (per window)       │               │
│  │  ┌─────────────────────────────────────┐     │               │
│  │  │ SwipeLauncherConfig                 │     │               │
│  │  │ PluginManager + ServiceManager      │     │               │
│  │  │ AreaManager                         │     │               │
│  │  │ UnboundedChannel<FfiEnvelope>       │     │               │
│  │  │ JsonConverterRegistry               │     │               │
│  │  │ Message Routing Loop (tokio/gtk)    │     │               │
│  │  └─────────────────────────────────────┘     │               │
│  └──────────────────────────────────────────────┘               │
│                                                                  │
│  Global: JsonConverterRegistry (shared, keyed by instance or     │
│          registered by all plugins from all instances)          │
└─────────────────────────────────────────────────────────────────┘
```

## 4. Refactoring Plan

### 4.1 New Type: `LauncherInstance`

Extract all per-window state from `LauncherApplication` into a new `LauncherInstance` struct. Rename `LauncherApplication` to `LauncherHost` or keep it as the
top-level orchestrator.

**New `instance.rs`:**

```rust
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use gtk4::Application;
use gtk4::ApplicationWindow;

/// Per-window launcher instance with fully isolated state
pub struct LauncherInstance {
    pub config: SwipeLauncherConfig,
    pub plugin_manager: Arc<PluginManager>,
    pub area_manager: Arc<Mutex<AreaManager>>,
    pub message_sender: UnboundedSender<FfiEnvelope>,
    pub topic_rate_limiter: Arc<Mutex<HashMap<String, Instant>>>,
    pub json_converter_registry: Arc<JsonConverterRegistry>,
    pub window: Option<ApplicationWindow>,
    pub instance_id: String,
}

impl LauncherInstance {
    pub fn new(
        config: SwipeLauncherConfig,
        instance_id: String,
        broker_sender: UnboundedSender<FfiEnvelope>,
    ) -> Self {
        let json_converter_registry = Arc::new(JsonConverterRegistry::new());
        let plugin_manager = Arc::new(PluginManager::new(
            instance_id.clone(),
            broker_sender.clone(),
        ));
        let config_arc = Arc::new(config.clone());
        let area_manager = Arc::new(Mutex::new(
            AreaManager::new(plugin_manager.clone(), config_arc, json_converter_registry.clone())
        ));

        LauncherInstance {
            config,
            plugin_manager,
            area_manager,
            message_sender: broker_sender,
            topic_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            json_converter_registry,
            window: None,
            instance_id,
        }
    }

    pub fn load_plugins(&self) { /* same as before, namespaced IDs */ }
    pub fn handle_message(&self, envelope: FfiEnvelope) { /* same routing logic */ }
    pub fn build_window(&mut self, app: &Application) -> ApplicationWindow { /* window creation */ }
}
```

### 4.2 Refactored `LauncherApplication` → `LauncherHost`

`LauncherHost` owns the single `gtk4::Application`, the shared `ServiceManager`, the central message broker, and a collection of `LauncherInstance`s:

```rust
use gtk4::Application;
use std::collections::HashMap;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};

/// Host that manages all launcher instances in a single process
pub struct LauncherHost {
    pub gtk_app: Application,
    pub service_manager: Arc<ServiceManager>,
    pub broker_sender: UnboundedSender<FfiEnvelope>,
    pub broker_receiver: Mutex<Option<UnboundedReceiver<FfiEnvelope>>>,
    pub instances: HashMap<String, LauncherInstance>,
}

impl LauncherHost {
    pub fn new(gtk_app: Application) -> Self {
        let (broker_sender, broker_receiver) = unbounded_channel::<FfiEnvelope>();
        let service_manager = Arc::new(ServiceManager::new(broker_sender.clone()));

        LauncherHost {
            gtk_app,
            service_manager,
            broker_sender,
            broker_receiver: Mutex::new(Some(broker_receiver)),
            instances: HashMap::new(),
        }
    }

    pub fn create_instance(&mut self, instance_id: String, config: SwipeLauncherConfig) -> &mut LauncherInstance {
        let instance = LauncherInstance::new(
            config,
            instance_id.clone(),
            self.broker_sender.clone(),
        );
        instance.load_plugins();
        self.instances.insert(instance_id.clone(), instance);
        self.instances.get_mut(&instance_id).unwrap()
    }

    pub fn load_services(&self, services_config: &ServicesConfig) {
        for service_entry in &services_config.services {
            let service_config = services_config.plugin_config(&service_entry.id);
            if let Err(e) = self.service_manager.load_service(service_entry, service_config) {
                error!("Failed to load service {}: {}", service_entry.id, e);
            }
        }
    }

    pub fn open_window(&mut self, instance_id: &str) {
        if let Some(instance) = self.instances.get_mut(instance_id) {
            let window = instance.build_window(&self.gtk_app);
            instance.window = Some(window);
        }
    }

    pub fn run(&self) {
        self.gtk_app.run();
    }
}
```

### 4.3 Central Message Broker with `instance_id`

Instead of one message broker per instance, use a **single central broker** in `LauncherHost`. Every `FfiEnvelope` carries a `target_instance_id` field.

#### 4.3.1 Extended `FfiEnvelope`

```rust
#[stabby::stabby(no_opt)]
#[derive(Clone)]
pub struct FfiEnvelope {
    pub sender_id: stabby::string::String,
    pub target_instance_id: stabby::string::String, // "*" = broadcast, "" = sender's own instance
    pub topic: stabby::string::String,
    pub type_id: u64,
    pub payload: *mut core::ffi::c_void,
    pub destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
}
```

Plugins send messages via `MessageBroadcasterInner::broadcast_message`. The broadcaster can set `target_instance_id`:

```rust
impl MessageBroadcasterInner {
    pub fn broadcast_message<T: Clone + TypedMessage>(&self, target_instance_id: &str, topic: &str, payload: &T) {
        if let Some(ctx) = &self.core_context {
            let payload_ptr = Box::into_raw(Box::new(payload.clone())) as *mut core::ffi::c_void;
            let envelope = FfiEnvelope {
                sender_id: stabby::string::String::from(self.meta.id.clone()),
                target_instance_id: stabby::string::String::from(target_instance_id),
                topic: stabby::string::String::from(topic),
                type_id: T::TYPE_ID,
                payload: payload_ptr,
                destroy_payload: Some(default_destroy_payload),
            };
            ctx.send_message(envelope);
        }
    }
}
```

#### 4.3.2 Broker Routing in `LauncherHost`

The central broker receives all messages and routes them:

```rust
impl LauncherHost {
    pub fn start_broker_loop(&self) {
        let Ok(mut receiver_guard) = self.broker_receiver.lock() else {
            error!("Failed to lock broker receiver");
            return;
        };
        let Some(receiver) = receiver_guard.take() else {
            error!("Broker receiver already taken");
            return;
        };

        MainContext::default().spawn_local(async move {
            while let Some(envelope) = receiver.recv().await {
                self.route_message(envelope);
            }
            error!("Central broker loop exited");
        });
    }

    pub fn route_message(&self, envelope: FfiEnvelope) {
        let target = envelope.target_instance_id.to_string();

        // "*" or empty target = route to sender's own instance
        let target_instance = if target.is_empty() || target == "*" {
            // Extract instance from sender_id (format: "instance_id:plugin_id")
            envelope.sender_id.to_string()
                .split(':')
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            target
        };

        // Route to specific instance
        if let Some(instance) = self.instances.get(&target_instance) {
            instance.handle_message(envelope);
            return;
        }

        // Broadcast to all instances (used by shared services)
        if target == "*" {
            for instance in self.instances.values() {
                instance.handle_message(envelope.clone());
            }
            return;
        }

        warn!("Unknown target instance '{}', dropping message", target_instance);
    }
}
```

#### 4.3.3 Instance-Scoped `handle_message`

`handle_message` moves from `LauncherApplication` to `LauncherInstance`. It no longer needs to know about other instances — the broker already routed correctly.

```rust
impl LauncherInstance {
    pub fn handle_message(&self, envelope: FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        let topic = envelope.topic.to_string();

        // Rate-limit command topics
        if topic.ends_with(".command") || topic.ends_with(".status") {
            // ... rate limit logic using self.topic_rate_limiter
        }

        // Try JSON conversion using self.json_converter_registry
        let mut envelope = envelope;
        if let Some(converted) = try_convert_string_to_typed_envelope(
            &self.json_converter_registry,
            &envelope,
        ) {
            // ... replace envelope
            envelope = converted;
        }

        // Route to area manager (area.* topics)
        if topic.starts_with("area.") {
            if let Ok(area_manager) = self.area_manager.lock() {
                area_manager.route(&envelope);
            }
            return;
        }

        // Route to specific plugin (plugin.<id> topics)
        if topic.starts_with("plugin.") {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 {
                let target_plugin_id = parts[1];
                if let Some(plugin) = self.plugin_manager.plugins.get(target_plugin_id) {
                    unsafe { plugin.on_message(envelope); }
                }
            }
            return;
        }

        // Broadcast to all plugins in this instance
        if topic.starts_with("plugins.broadcast.") {
            for plugin in self.plugin_manager.plugins.iter() {
                unsafe { plugin.value().on_message(envelope.clone()); }
            }
            return;
        }

        // Core close — only close this instance's window
        if topic == "core.close" {
            if let Some(ref window) = self.window {
                window.close();
            }
            return;
        }
    }
}
```

### 4.4 Namespacing: `instance_id:area_id`

All resource IDs inside an instance are prefixed with the instance ID. This applies to plugins, areas, and any message targets.

#### 4.4.1 Plugin ID Namespacing

`PluginManager` stores plugins with prefixed IDs:

```rust
// In PluginManager::load_plugin
let namespaced_id = format!("{}:{}", instance_id, plugin_entry.id);
self .plugins.insert(namespaced_id.clone(), plugin);
```

The `sender_id` in `FfiEnvelope` is automatically set to `"instance_id:plugin_id"` by `SimpleCoreContext`:

```rust
// In SimpleCoreContext::new
let sender_id = format!("{}:{}", instance_id, plugin_id);
```

#### 4.4.2 Area ID Namespacing

Areas are also prefixed. When an area message targets `"left:clock_area"`, the broker routes to instance `"left"`, and the `AreaManager` resolves `"clock_area"`
internally.

```rust
// In area message routing
if topic.starts_with("area.") {
let area_id = topic.strip_prefix("area.").unwrap_or("");
// area_id might be "left:clock_area" or just "clock_area"
let (target_instance, local_area_id) = if area_id.contains(':') {
let mut parts = area_id.splitn(2, ':');
(parts.next().unwrap_or(""), parts.next().unwrap_or(""))
} else {
("", area_id)
};

if target_instance.is_empty() | | target_instance == self.instance_id {
// Handle locally
if let Ok(area_manager) = self.area_manager.lock() {
area_manager.route_to_area(local_area_id, & envelope);
}
} else {
// Re-route to other instance via central broker
let _ = self.message_sender.send(FfiEnvelope {
target_instance_id: stabby::string::String::from(target_instance),
..envelope
});
}
}
```

### 4.5 Shared ServiceManager on Host Level

D-Bus services (Notification, MPRIS) are registered **once** at the `LauncherHost` level. All instances share the same `ServiceManager`.

#### 4.5.1 Service Loading in `LauncherHost`

```rust
impl LauncherHost {
    pub fn load_services(&self, all_configs: &[SwipeLauncherConfig]) {
        // Collect unique service entries across all configs
        let mut seen_services = HashSet::new();
        for config in all_configs {
            for service_entry in &config.services {
                if seen_services.insert(service_entry.id.clone()) {
                    let service_config = config.plugin_config(&service_entry.id);
                    if let Err(e) = self.service_manager.load_service(service_entry, service_config) {
                        error!("Failed to load service {}: {}", service_entry.id, e);
                    }
                }
            }
        }
    }
}
```

#### 4.5.2 Service Message Routing

When a service sends a message, it goes through the central broker. Services broadcast to all instances by default:

```rust
// Service messages (topic starts with "service.") are broadcast
if topic.starts_with("service.") {
// Send to all instances
for instance in self.instances.values() {
instance.handle_message(envelope.clone());
}
return;
}
```

Services can also target a specific instance by setting `target_instance_id`.

#### 4.5.3 Service → Instance Notification Example

```
┌──────────────────┐
│ Notification     │  service.notifications.status
│ Service          │  ──────────────► central broker
└──────────────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
        ┌─────────┐   ┌─────────┐   ┌─────────┐
        │ left    │   │ right   │   │ top     │
        │Instance │   │Instance │   │Instance │
        └─────────┘   └─────────┘   └─────────┘
```

All three Notification widgets on the smart desk receive the same notification payload.

### 4.6 Global JSON Converter Registry

The `GLOBAL_JSON_CONVERTER_REGISTRY` is process-wide. Since all instances share it, converters registered by plugins from any instance are available to all.
This is correct behavior for shared message types.

**No changes required.**

However, if per-instance converters are needed in the future, replace with:

```rust
static GLOBAL_JSON_CONVERTER_REGISTRY: OnceLock<DashMap<String, Arc<JsonConverterRegistry>>> = OnceLock::new();
```

## 5. Configuration Changes

### 5.1 CLI Argument Structure

```rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct SwipeLauncherArguments {
    /// Configuration file for shared services (D-Bus background services).
    /// Loaded once and shared across all instances.
    #[arg(short = 's', long, default_value = Some("services.toml"))]
    pub(crate) services_config: Option<PathBuf>,

    /// Configuration files for each launcher instance window.
    /// Specify multiple times for multiple windows.
    #[arg(short, long, default_value = Some("config.toml"))]
    pub(crate) config: Vec<PathBuf>,

    /// Optional instance IDs corresponding to each --config.
    /// If omitted, the config file stem is used as the instance ID.
    #[arg(short, long)]
    pub(crate) instance_id: Vec<String>,
}
```

### 5.2 Services Config File

Services are defined in a **dedicated** config file (e.g. `services.toml`). This file is loaded **once** by `LauncherHost` and shared across all instances.

**Example `services.toml`:**

```toml
[[services]]
id = "notifications"
path = "target/debug/libnotifications_service.so"

[services.config]
max_history = 50

[[services]]
id = "mpris"
path = "target/debug/libmpris_service.so"

[[services]]
id = "audio"
path = "target/debug/libaudio_service.so"
```

**Instance configs (e.g. `config-left.toml`) no longer contain `services`:**

```toml
areas = ["main", "favorites"]

[launcher]
rotation = 0

[launcher.layer]
namespace = "smearor-left"
layer = "Top"
exclusive_zone = 50

[main]
area_type = "Scroll"
plugins = [
    { id = "clock", path = "target/debug/libclock_widget.so" },
    { id = "app-launcher", path = "target/debug/libapp_launcher_widget.so" },
]

[favorites]
area_type = "Fixed"
width = 200
plugins = [
    { id = "button-home", path = "target/debug/libbutton_widget.so" },
]
```

### 5.3 Instance ID from Config

Derive `instance_id` from the config file or an explicit `--instance-id` per config:

```bash
./smearor-swipe-launcher \
    --services-config services.toml \
    --config config-left.toml --instance-id left \
    --config config-right.toml --instance-id right
```

Or use the config file stem as ID:

```rust
let instance_id = config_path.file_stem()
.and_then( | s| s.to_str())
.unwrap_or("default")
.to_string();
```

### 5.4 Config Namespace Isolation

Each config declares distinct `layer.namespace`:

```toml
# config-left.toml
[launcher.layer]
namespace = "smearor-left"
layer = "Top"
exclusive_zone = 50

# config-right.toml
[launcher.layer]
namespace = "smearor-right"
layer = "Top"
exclusive_zone = 50
```

## 6. File Changes Summary

| File                                            | Change                                                                                                   |
|-------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| `smearor-swipe-launcher/src/instance.rs`        | **New** — per-window state container (`LauncherInstance`)                                                |
| `smearor-swipe-launcher/src/application.rs`     | Refactor `LauncherApplication` → `LauncherHost` with central broker and shared `ServiceManager`          |
| `smearor-swipe-launcher/src/config/services.rs` | **New** — `ServicesConfig` struct for the dedicated services config file                                 |
| `smearor-swipe-launcher/src/args/launcher.rs`   | Add `services_config: Option<PathBuf>`, change `config` → `Vec<PathBuf>`, add `instance_id: Vec<String>` |
| `smearor-swipe-launcher/src/main.rs`            | Load `services.toml` once into `LauncherHost`, then iterate instance configs                             |
| `smearor-swipe-launcher/src/window.rs`          | No changes (already parameterized by `SwipeLauncherSettings`)                                            |
| `smearor-swipe-launcher/src/plugin_manager.rs`  | Add `instance_id` parameter; store plugins with `instance_id:plugin_id` keys                             |
| `smearor-swipe-launcher/src/context.rs`         | Pass `instance_id` to `SimpleCoreContext` for namespaced `sender_id`                                     |
| `smearor-swipe-launcher/src/messages/mod.rs`    | Move `handle_message` impl to `LauncherInstance`; remove global `core.close` quit logic                  |
| `plugin-api/src/messages.rs`                    | Add `target_instance_id` field to `FfiEnvelope`                                                          |
| `plugin-api/src/context.rs`                     | Update `MessageBrokerHandle`/`FfiCoreContext` to carry `instance_id`                                     |
| `plugin-api/src/json_converter.rs`              | No changes (stays global)                                                                                |

## 7. Example Usage

```bash
# Default: single window (backward compatible)
./smearor-swipe-launcher --config config.toml

# Four sides of a smart desk
./smearor-swipe-launcher \
    --config config-north.toml \
    --config config-east.toml \
    --config config-south.toml \
    --config config-west.toml

# Two instances with explicit IDs
./smearor-swipe-launcher \
    --config config-left.toml --instance-id left \
    --config config-right.toml --instance-id right
```

## 8. Resolved Questions

### Q2: Plugin Shared Library Loading (libloading)

**Status: Resolved**

`libloading` calls `dlopen` internally. Under Linux, the dynamic linker (`ld.so`) ensures that a shared library (`.so`) opened multiple times with the same path
is mapped **only once** into the process address space. Global symbols do not collide — they share the same code segment.

**Caveat:** If plugins use internal static/global state (static variables in C/Rust), all four instances share that state. Since plugins are instantiated via
`PluginManager::load_plugin` and should hold their state in a struct object, this is not critical. The plugin ID namespacing (Section 4.4.2) is the perfect and
sufficient safeguard.

### Q3: Memory Footprint

**Status: Resolved**

`.so` files are cached and shared by the OS (see Q2), so the RAM overhead for code is minimal. What consumes RAM are the GTK widgets per window. On a smart desk
PC in 2026 (typically 8–16 GB RAM), a few megabytes for four UI instances are absolutely negligible.

**Decision:** Widget deduplication would violate GTK4 ownership rules (a widget may only have one parent). Keep instances fully isolated.

### Q4: Focus Management

**Status: Resolved**

Since these are layer-shell windows (acting as panels/overlays), keyboard focus is typically acquired only via `gtk_layer_shell::set_keyboard_interactivity`
when the menu is actively swiped open. Wayland strictly regulates input focus based on which window receives the touch/click event. GTK4 automatically routes
input into the correct `ApplicationWindow`.

**Decision:** No manual focus routing implementation required.

## 9. Summary

| Aspect                 | Before         | After                                     |
|------------------------|----------------|-------------------------------------------|
| Process count          | 1 per instance | 1 total                                   |
| GTK Application        | 1 per process  | 1 total                                   |
| Windows                | 1 per process  | N per process                             |
| PluginManagers         | 1 per process  | 1 per window                              |
| ServiceManager         | 1 per process  | 1 shared at `LauncherHost`                |
| Message broker         | 1 per process  | 1 central broker in `LauncherHost`        |
| Config files           | 1 per process  | N per process                             |
| Area IDs               | local only     | `instance_id:area_id` namespaced          |
| Plugin IDs             | local only     | `instance_id:plugin_id` namespaced        |
| Cross-window messaging | not possible   | via `target_instance_id` in `FfiEnvelope` |
| Service → all widgets  | not possible   | broadcast via central broker              |

## 10. Implementation Roadmap

### Phase 1: Foundation (API & Config) ✅

| #   | Task                                                                    | Files                                                                | Effort  |
|-----|-------------------------------------------------------------------------|----------------------------------------------------------------------|---------|
| 1.1 | Add `target_instance_id` to `FfiEnvelope` in plugin-api                 | `plugin-api/src/messages.rs`                                         | Small ✅ |
| 1.2 | Add `instance_id` to `MessageBrokerHandle` / `FfiCoreContext`           | `plugin-api/src/context.rs`                                          | Small ✅ |
| 1.3 | Create `ServicesConfig` struct + parser for dedicated services config   | `smearor-swipe-launcher/src/config/services.rs`                      | Small ✅ |
| 1.4 | Update CLI args: `--services-config`, multi `--config`, `--instance-id` | `smearor-swipe-launcher/src/args/launcher.rs`                        | Small ✅ |
| 1.5 | Adapt Host `broker_send_wrapper` + service `FfiEnvelope` constructors   | `smearor-swipe-launcher/src/context.rs`, `services/*/src/service.rs` | Small ✅ |

### Phase 2: LauncherHost & LauncherInstance ✅

| #   | Task                                                                                          | Files                                          | Effort   |
|-----|-----------------------------------------------------------------------------------------------|------------------------------------------------|----------|
| 2.1 | Create `LauncherInstance` struct with isolated plugin manager + area manager                  | `smearor-swipe-launcher/src/instance.rs` (new) | Medium ✅ |
| 2.2 | Refactor `LauncherApplication` → `LauncherHost` with central broker + shared `ServiceManager` | `smearor-swipe-launcher/src/application.rs`    | Large ✅  |
| 2.3 | Move `handle_message` from `LauncherApplication` to `LauncherInstance`                        | `smearor-swipe-launcher/src/messages/mod.rs`   | Medium ✅ |
| 2.4 | Remove global `core.close` → `gtk_app.quit()`; replace with per-instance `window.close()`     | `smearor-swipe-launcher/src/messages/mod.rs`   | Small ✅  |

### Phase 3: Namespacing & Routing ✅

| #   | Task                                                                  | Files                                             | Effort                      |
|-----|-----------------------------------------------------------------------|---------------------------------------------------|-----------------------------|
| 3.1 | Prefix plugin IDs with `instance_id:plugin_id` in `PluginManager`     | `smearor-swipe-launcher/src/plugin_manager.rs`    | Small ✅                     |
| 3.2 | Pass `instance_id` to `SimpleCoreContext` for namespaced `sender_id`  | `smearor-swipe-launcher/src/context.rs`           | Small ✅                     |
| 3.3 | Implement central broker routing loop in `LauncherHost`               | `smearor-swipe-launcher/src/application.rs`       | Small ✅ *(done in Phase 2)* |
| 3.4 | Parse `instance_id:area_id` in area messages; re-route cross-instance | `smearor-swipe-launcher/src/area/area_manager.rs` | Medium ✅                    |

### Phase 4: Service Integration ✅

| #   | Task                                                                          | Files                                            | Effort                      |
|-----|-------------------------------------------------------------------------------|--------------------------------------------------|-----------------------------|
| 4.1 | Load services once from `services.toml` in `LauncherHost`                     | `smearor-swipe-launcher/src/main.rs`             | Small ✅ *(done in Phase 2)* |
| 4.2 | Broadcast service messages (`service.*`) to all instances from central broker | `smearor-swipe-launcher/src/application.rs`      | Small ✅ *(done in Phase 2)* |
| 4.3 | Remove `services` section from instance configs; validate no duplication      | `config.toml`, `config-90.toml`, `services.toml` | Small ✅                     |

### Phase 5: Main Entry Point & Testing ✅

| #   | Task                                                                                    | Files                                | Effort                       |
|-----|-----------------------------------------------------------------------------------------|--------------------------------------|------------------------------|
| 5.1 | Update `main.rs` to create `LauncherHost`, load services, then iterate instance configs | `smearor-swipe-launcher/src/main.rs` | Medium ✅ *(done in Phase 2)* |
| 5.2 | Test single-instance mode (backward compatibility)                                      | All                                  | ✅ Verified at runtime        |
| 5.3 | Test namespacing (`config:plugin_id`) and area message routing                          | All                                  | ✅ Verified at runtime        |
| 5.4 | Test shared service broadcast and per-instance `core.close`                             | All                                  | ✅ Verified at runtime        |

---

## CLI & Config Usage

### Single-Instance Mode (Backward Compatible)

```bash
smearor-swipe-launcher \
  --services-config services.toml \
  --config config.toml
```

- One launcher window with all configured areas.
- `instance_id` defaults to the config file stem (`config`).
- Services are loaded once and broadcast to the single instance.

### Multi-Instance Mode (Smart Desk)

```bash
smearor-swipe-launcher \
  --services-config services.toml \
  --config config.toml     --instance-id side1 \
  --config config-90.toml  --instance-id side2
```

- `side1` and `side2` are two independent windows sharing services.
- Each instance has its own `PluginManager`, `AreaManager`, and window.
- Cross-instance area addressing: a plugin on `side1` sends `area.side2:submenu.open` to open an area on `side2`.

### Config Files

**`services.toml`** (shared across all instances):

```toml
[[services]]
id = "app_launcher"
path = "target/release/libsmearor_app_launcher_service.so"

[[services]]
id = "notifications"
path = "target/release/libsmearor_notifications_service.so"
```

**`config.toml`** (per-instance):

```toml
areas = ["left_area", "scroll_band", "right_area"]

[launcher]
rotation = 0
layer = "top"

[left_area]
area_type = "fixed"
width = 200
plugins = [
    { id = "clock_widget", path = ".../libsmearor_clock_widget.so" }
]

[clock_widget]
format = "[hour]:[minute]"
```

> **Note:** The `services` section has been removed from instance configs. Services are now loaded exclusively from the dedicated `services.toml` file passed
> via `--services-config`.

---

This approach enables the full Smearor Smart Desk experience — four independent launchers, cross-window menu navigation, shared D-Bus background services, and
namespaced area addressing — from a single binary.
