# Concept: Dynamic Launcher Instance Load & Unload

Enables dynamic creation and termination of **Launcher Instances** at runtime — driven by **Message-Broker-Topics** and **MCP Server Tools**. This extends the
existing multi-instance architecture (`LauncherHost` / `LauncherInstance`) with the ability to load a new instance from a config file path and to stop a running
instance by its `instance_id`, without restarting the process.

Instances can be **GTK** (with a visible window), **Headless** (no window, for hardware devices like MacroPads), or **Web** (no window, served via HTTP). The
instance type is specified at load time and determines whether a window is built and how widgets are rendered. See `concepts/WEB_INSTANCE_CONCEPT.md` for the
Web instance concept.

---

## 1. Goal

Currently, launcher instances are created exclusively at startup in `main.rs` from CLI `--config` arguments. Once the GTK main loop is running, no new instances
can be added or removed. This concept adds four runtime operations:

| Operation                  | Trigger              | Action                                                                                                  |
|----------------------------|----------------------|---------------------------------------------------------------------------------------------------------|
| **Load GTK instance**      | Message-Broker-Topic | Parse config, create `LauncherInstance`, load plugins, build window                                     |
| **Load GTK instance**      | MCP Server Tool      | Same as above, invoked via MCP tool call                                                                |
| **Load headless instance** | Message-Broker-Topic | Parse config, create `LauncherInstance`, load plugins, no window                                        |
| **Load headless instance** | MCP Server Tool      | Same as above, invoked via MCP tool call                                                                |
| **Load web instance**      | Message-Broker-Topic | Parse config, create `LauncherInstance`, load plugins, no window, served via HTTP                       |
| **Load web instance**      | MCP Server Tool      | Same as above, invoked via MCP tool call                                                                |
| **Stop instance**          | Message-Broker-Topic | Close window (if GTK), close WebSockets (if Web), unload plugins, remove from `LauncherHost::instances` |
| **Stop instance**          | MCP Server Tool      | Same as above, invoked via MCP tool call                                                                |

---

## 2. Current Architecture

### 2.1 Instance Lifecycle (Startup Only)

```
main.rs
  ├── LauncherHost::new(gtk_app)
  ├── host.load_services(&services_config)
  ├── for each --config:
  │     host.create_instance(instance_id, config)
  ├── host.build_ui()
  │     └── gtk_app.connect_activate:
  │           for each instance: instance.build_window(app)
  └── host.run()  // GTK main loop
```

### 2.2 Key Structures

- **`LauncherHost`** (`host/mod.rs`): Owns `instances: Arc<Mutex<HashMap<String, LauncherInstance>>>`, `gtk_app`, `broker_sender`, `service_manager`.
- **`LauncherInstance`** (`instance.rs`): Owns `config`, `plugin_manager`, `area_manager`, `window: Mutex<Option<ApplicationWindow>>`, `instance_id`,
  `instance_type: InstanceType`.
- **`create_instance()`** (`host/mod.rs:104`): Creates instance, loads plugins, inserts into HashMap.
- **`build_ui()`** (`host/mod.rs:159`): `connect_activate` callback builds windows for all instances.
- **`route_message()`** (`host/mod.rs:363`): Central broker routes by `target_instance_id`.

### 2.3 Limitation

`create_instance` and `build_window` are only called during startup. The `connect_activate` callback fires once. There is no mechanism to:

1. Create and build a window for a new instance after activation.
2. Tear down an instance and remove it from the broker routing table.

---

## 3. Architecture Changes

### 3.1 Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                         Single Process                              │
│                                                                     │
│  ┌──────────────────┐                                              │
│  │ gtk4::Application │  ← connect_activate fires once              │
│  └────────┬─────────┘                                              │
│           │                                                         │
│  ┌────────▼──────────────────────────────────────────┐            │
│  │              LauncherHost                          │            │
│  │  instances: HashMap<String, LauncherInstance>     │            │
│  │  broker_sender / broker_receiver                  │            │
│  │  service_manager (shared)                         │            │
│  │  mcp_registry / mcp_command_sender                │            │
│  └────────┬──────────────────────────────────────────┘            │
│           │                                                         │
│  ┌────────▼──────────────────────────────────────────┐            │
│  │  Central Message Broker (route_message)            │            │
│  │                                                    │            │
│  │  New topic handlers:                               │            │
│  │  ├── core.instance.load   → load_instance()       │            │
│  │  └── core.instance.stop    → stop_instance()      │            │
│  └───────────────────────────────────────────────────┘            │
│                                                                     │
│  MCP Server:                                                        │
│  ├── launcher_load_instance (core tool)                            │
│  └── launcher_stop_instance (core tool)                            │
└────────────────────────────────────────────────────────────────────┘
```

### 3.2 New `LauncherHost` Methods

```rust
impl LauncherHost {
    /// Dynamically load a new launcher instance from a config file path.
    /// Called at runtime (after GTK activation).
    /// `instance_type` determines whether a GTK window is built (Gtk) or not (Headless).
    pub fn load_instance(&self, instance_id: String, config_path: &str, instance_type: InstanceType) -> Result<String, String> {
        // 1. Validate config path against allowlist
        validate_config_path(config_path)?;

        // 2. Validate instance_id (no colons, no path separators)
        validate_instance_id(&instance_id)?;

        // 3. Parse config from file
        let config = SwipeLauncherConfig::from_file(config_path)
            .map_err(|e| format!("Failed to parse config {}: {}", config_path, e))?;

        // 4. Check for duplicate instance_id
        if let Ok(instances) = self.instances.lock() {
            if instances.contains_key(&instance_id) {
                return Err(format!("Instance '{}' already exists", instance_id));
            }
        }

        // 5. Create instance (loads plugins)
        self.create_instance(instance_id.clone(), config, instance_type);

        // 6. Build window only for GTK instances (on the GTK main thread)
        if instance_type == InstanceType::Gtk {
            let self_clone = self.clone();
            let instance_id_clone = instance_id.clone();
            gtk4::glib::idle_add_local_once(move || {
                if let Ok(instances) = self_clone.instances.lock() {
                    if let Some(instance) = instances.get(&instance_id_clone) {
                        match instance.build_window(&self_clone.gtk_app) {
                            Ok(window) => {
                                if let Ok(mut window_guard) = instance.window.lock() {
                                    *window_guard = Some(window);
                                }
                                debug!("Dynamically loaded GTK instance '{}'", instance_id_clone);
                            }
                            Err(e) => {
                                error!("Failed to build window for dynamic instance '{}': {}", instance_id_clone, e);
                            }
                        }
                    }
                }
            });
        } else {
            debug!("Dynamically loaded {:?} instance '{}'", instance_type, instance_id);
        }

        // 7. Recalculate coordinated sizes (new instance may share a monitor)
        self.calculate_coordinated_sizes();

        // 8. Persist instance to state file
        self.persist_instance(&instance_id, config_path, instance_type);

        // 9. Broadcast status to all instances and services
        self.broadcast_instance_status(&instance_id, InstanceLifecycleEvent::Loaded);

        Ok(format!("Instance '{}' loaded from {}", instance_id, config_path))
    }

    /// Stop and remove a running launcher instance.
    /// Closes the window (if GTK), fully unloads plugins, unregisters MCP tools,
    /// and removes from the instance map.
    pub fn stop_instance(&self, instance_id: &str) -> Result<String, String> {
        // 1. Remove from instances map
        let instance = {
            let mut instances = self.instances.lock()
                .map_err(|e| format!("Failed to lock instances: {}", e))?;
            instances.remove(instance_id)
                .ok_or_else(|| format!("Instance '{}' not found", instance_id))?
        };

        // 2. Close window and unload areas on GTK main thread (only for GTK instances)
        let instance_id_owned = instance_id.to_string();
        if instance.instance_type == InstanceType::Gtk {
            gtk4::glib::idle_add_local_once(move || {
                if let Ok(mut window_guard) = instance.window.lock() {
                    if let Some(window) = window_guard.take() {
                        window.close();
                    }
                }
                // 3. Remove all areas (fully unloads plugins from area manager)
                if let Ok(area_manager) = instance.area_manager.lock() {
                    area_manager.remove_all_areas_immediate();
                }
                debug!("Stopped and removed GTK instance '{}'", instance_id_owned);
            });
        } else {
            // Headless or Web: just unload areas (no window to close)
            if instance.instance_type == InstanceType::Web {
                // Close all WebSockets for this instance
                if let Some(web_server) = &self.web_server {
                    web_server.close_websockets(&instance_id_owned);
                }
            }
            if let Ok(area_manager) = instance.area_manager.lock() {
                area_manager.remove_all_areas_immediate();
            }
            debug!("Stopped and removed {:?} instance '{}'", instance.instance_type, instance_id_owned);
        }

        // 4. Unregister MCP tools registered by this instance's plugins
        self.mcp_registry.remove_tools_by_instance(instance_id);

        // 5. Recalculate coordinated sizes (remaining instances may need resize)
        self.calculate_coordinated_sizes();

        // 6. Remove from persistence state file
        self.unpersist_instance(instance_id);

        // 7. Broadcast status to all instances and services
        self.broadcast_instance_status(instance_id, InstanceLifecycleEvent::Stopped);

        Ok(format!("Instance '{}' stopped", instance_id))
    }

    /// Hot-reload an instance: stop and re-load with the same ID atomically.
    /// Low priority — implemented as a convenience wrapper.
    /// Preserves the original instance type.
    pub fn reload_instance(&self, instance_id: &str, config_path: &str) -> Result<String, String> {
        // Determine original instance type before stopping
        let instance_type = {
            if let Ok(instances) = self.instances.lock() {
                instances.get(instance_id).map(|i| i.instance_type).unwrap_or(InstanceType::Gtk)
            } else {
                InstanceType::Gtk
            }
        };
        // Stop if running (ignore "not found" error — instance may not exist yet)
        let _ = self.stop_instance(instance_id);
        // Load with the same ID and original type
        self.load_instance(instance_id.to_string(), config_path, instance_type)
    }
}
```

### 3.2.1 Config Path Allowlist

The `validate_config_path` function checks that the resolved absolute path is within one of the allowed directories:

```rust
fn validate_config_path(config_path: &str) -> Result<(), String> {
    let path = std::path::Path::new(config_path);
    let canonical = path.canonicalize()
        .map_err(|e| format!("Config path '{}' cannot be resolved: {}", config_path, e))?;

    let allowed_dirs = [
        std::env::current_dir().unwrap_or_default(),
        dirs::config_dir().unwrap_or_default().join("smearor"),
    ];

    for allowed in &allowed_dirs {
        if canonical.starts_with(allowed) {
            return Ok(());
        }
    }

    Err(format!(
        "Config path '{}' is outside allowed directories (current dir and ~/.config/smearor/)",
        config_path
    ))
}
```

### 3.3 GTK Main Thread Safety

GTK4 requires all widget operations to happen on the main thread. Both `load_instance` and `stop_instance` use `gtk4::glib::idle_add_local_once` to schedule
window creation/destruction on the GLib main context. The config parsing and HashMap insertion happen on the calling thread (broker or tokio), but the GTK
operations are deferred. Windows appear instantly — no fade-in animation (GTK4 windows do not support transition animations).

### 3.4 Broker Topic Routing

The central broker in `route_message()` gains two new topic handlers, placed before the general instance routing:

```rust
// In LauncherHost::route_message()

// Dynamic instance loading
if topic == "core.instance.load" {
if ! envelope.payload.is_null() {
let msg = unsafe { & * (envelope.payload as * const InstanceLoadMessage) };
let instance_id = msg.instance_id.to_string();
let config_path = msg.config_path.to_string();
let instance_type = msg.instance_type;
let result = self.load_instance(instance_id, &config_path, instance_type);
// Send response on broker if sender expects one
if ! msg.response_topic.is_empty() {
self.send_broker_response( & msg.response_topic, &result);
}
}
return;
}

// Dynamic instance stopping
if topic == "core.instance.stop" {
if ! envelope.payload.is_null() {
let msg = unsafe { & * (envelope.payload as * const InstanceStopMessage) };
let instance_id = msg.instance_id.to_string();
let result = self.stop_instance( &instance_id);
if ! msg.response_topic.is_empty() {
self.send_broker_response( & msg.response_topic, &result);
}
}
return;
}
```

---

## 4. Model Crate (`model/instance-control`)

### 4.1 Overview

A new model crate defines the message types for dynamic instance load/stop operations. All FFI-relevant types carry `#[stabby::stabby]`.

### 4.2 Topics

```rust
/// Topic to dynamically load a new launcher instance.
pub const TOPIC_CORE_INSTANCE_LOAD: &str = "core.instance.load";
/// Topic to dynamically stop a running launcher instance.
pub const TOPIC_CORE_INSTANCE_STOP: &str = "core.instance.stop";
/// Topic to hot-reload a running instance (stop + load with same ID).
pub const TOPIC_CORE_INSTANCE_RELOAD: &str = "core.instance.reload";
/// Topic for instance status broadcasts (Loaded / Stopped).
pub const TOPIC_CORE_INSTANCE_STATUS: &str = "core.instance.status";
```

### 4.3 Instance Type Enum

```rust
/// The type of launcher instance, determining whether a GTK window is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[stabby::stabby]
pub enum InstanceType {
    /// A standard GTK launcher instance with a visible window.
    Gtk,
    /// A headless instance without a window (e.g. for MacroPad hardware devices).
    Headless,
    /// A web instance without a window, served via the embedded HTTP server.
    /// See `concepts/WEB_INSTANCE_CONCEPT.md`.
    Web,
}
```

### 4.4 Instance Load Message

```rust
/// Message to dynamically load a new launcher instance from a config file.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct InstanceLoadMessage {
    /// Unique identifier for the new instance.
    pub instance_id: stabby::string::String,
    /// File system path to the TOML config file for this instance.
    pub config_path: stabby::string::String,
    /// Whether to create a GTK window (Gtk) or run headless (Headless).
    pub instance_type: InstanceType,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}
```

### 4.5 Instance Stop Message

```rust
/// Message to dynamically stop and remove a running launcher instance.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct InstanceStopMessage {
    /// Unique identifier of the instance to stop.
    pub instance_id: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}
```

### 4.5.1 Instance Reload Message

```rust
/// Message to hot-reload a running instance (stop + load with same ID).
/// Low priority — convenience wrapper around stop + load.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct InstanceReloadMessage {
    /// Unique identifier of the instance to reload.
    pub instance_id: stabby::string::String,
    /// File system path to the TOML config file for the reloaded instance.
    pub config_path: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}
```

### 4.6 Instance Status Message

```rust
/// Status message broadcast when an instance is loaded or stopped.
/// Allows other instances and services to react to instance lifecycle changes.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct InstanceStatusMessage {
    /// The instance ID that changed.
    pub instance_id: stabby::string::String,
    /// The lifecycle event type.
    pub event: InstanceLifecycleEvent,
}

/// Lifecycle events for launcher instances.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub enum InstanceLifecycleEvent {
    /// A new instance was loaded and its window was created.
    Loaded,
    /// An instance was stopped and its window was closed.
    Stopped,
    /// An instance was hot-reloaded (stopped and reloaded with the same ID).
    Reloaded,
}
```

### 4.7 JSON Converters

The crate registers JSON converters so that plugins and services can send `InstanceLoadMessage` and `InstanceStopMessage` as JSON string payloads (via
`send_message` MCP tool) and have them automatically converted to typed messages by the broker.

### 4.8 Cargo.toml

```toml
[package]
name = "smearor-instance-control-model"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
stabby = { workspace = true }
```

---

## 5. MCP Server Tools

Two new core tools are added to `mcp-server/src/tools.rs` in `core_tools()`.

### 5.1 `launcher_load_instance`

| Property        | Value                                                                                                                                 |
|-----------------|---------------------------------------------------------------------------------------------------------------------------------------|
| **Name**        | `launcher_load_instance`                                                                                                              |
| **Description** | Dynamically loads a new launcher instance from a config file at runtime. The instance gets its own window, plugins, and area manager. |
| **Arguments**   | `{ "instance_id": string, "config_path": string, "instance_type": "gtk" \| "headless" }`                                              |
| **Returns**     | `Ok("Instance '<id>' loaded from <path>")` or `Err("...")`                                                                            |

```rust
ToolDefinition {
name: "launcher_load_instance".to_string(),
description: "Dynamically loads a new launcher instance from a TOML config file path. The instance gets its own window, plugins, and areas. Use this to add a new launcher window at runtime.".to_string(),
input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "instance_id": {
                "type": "string",
                "description": "Unique identifier for the new instance (e.g. 'side3', 'macropad_5')"
            },
            "config_path": {
                "type": "string",
                "description": "File system path to the TOML config file (e.g. 'config-side3.toml')"
            },
            "instance_type": {
                "type": "string",
                "enum": ["gtk", "headless", "web"],
                "default": "gtk",
                "description": "Instance type: 'gtk' creates a visible window, 'headless' runs without a window (for hardware devices), 'web' serves the instance via HTTP (see WEB_INSTANCE_CONCEPT.md)"
            }
        },
        "required": ["instance_id", "config_path"]
    }),
handler: Box::new( | sender, params| {
let Some(instance_id) = get_string_param(params, "instance_id") else {
return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
};
let Some(config_path) = get_string_param(params, "config_path") else {
return Box::pin(async move { Err("Missing config_path".to_string()) }) as ToolFuture;
};
let instance_type = match get_string_param(params, "instance_type").as_deref() {
Some("headless") => InstanceType::Headless,
Some("web") => InstanceType::Web,
_ => InstanceType::Gtk,
};
Box::pin(async move {
send_command_and_wait(
sender,
McpCommand::LoadInstance {
instance_id,
config_path,
instance_type,
response: oneshot::channel().0,
},
)
.await
})
}),
}
```

### 5.2 `launcher_stop_instance`

| Property        | Value                                                                                 |
|-----------------|---------------------------------------------------------------------------------------|
| **Name**        | `launcher_stop_instance`                                                              |
| **Description** | Stops a running launcher instance, closes its window, and removes it from the broker. |
| **Arguments**   | `{ "instance_id": string }`                                                           |
| **Returns**     | `Ok("Instance '<id>' stopped")` or `Err("...")`                                       |

```rust
ToolDefinition {
name: "launcher_stop_instance".to_string(),
description: "Stops a running launcher instance by its instance_id. Closes the window, unloads plugins, and removes the instance from the message broker. Other instances are not affected.".to_string(),
input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "instance_id": {
                "type": "string",
                "description": "Unique identifier of the instance to stop"
            }
        },
        "required": ["instance_id"]
    }),
handler: Box::new( | sender, params| {
let Some(instance_id) = get_string_param(params, "instance_id") else {
return Box::pin(async move { Err("Missing instance_id".to_string()) }) as ToolFuture;
};
Box::pin(async move {
send_command_and_wait(
sender,
McpCommand::StopInstance {
instance_id,
response: oneshot::channel().0,
},
)
.await
})
}),
}
```

### 5.3 `launcher_list_instances`

A supplementary read-only tool for introspection:

| Property        | Value                                                                            |
|-----------------|----------------------------------------------------------------------------------|
| **Name**        | `launcher_list_instances`                                                        |
| **Description** | Lists all currently running launcher instances with their IDs and window states. |
| **Arguments**   | `{}`                                                                             |
| **Returns**     | JSON array of `[{ "instance_id": string, "has_window": bool }]`                  |

---

## 6. McpCommand Extensions

Two new variants are added to `McpCommand` in `mcp-server/src/lib.rs`:

```rust
pub enum McpCommand {
    // ... existing variants ...

    /// Dynamically load a new launcher instance.
    LoadInstance {
        instance_id: String,
        config_path: String,
        instance_type: InstanceType,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// Stop a running launcher instance.
    StopInstance {
        instance_id: String,
        response: oneshot::Sender<Result<String, String>>,
    },
    /// List all running launcher instances.
    ListInstances {
        response: oneshot::Sender<Result<String, String>>,
    },
}
```

The `send_command_and_wait` function in `tools.rs` is extended with match arms for the new variants, replacing the dummy `oneshot::channel().0` with the real
`response_tx`.

---

## 7. Main Entry Point Integration

`process_mcp_command` in `main.rs` gains handlers for the new `McpCommand` variants:

```rust
McpCommand::LoadInstance { instance_id, config_path, instance_type, response } => {
let result = host.load_instance(instance_id, &config_path, instance_type);
let _ = response.send(result);
}
McpCommand::StopInstance { instance_id, response } => {
let result = host.stop_instance( & instance_id);
let _ = response.send(result);
}
McpCommand::ListInstances { response } => {
let result = {
if let Ok(instances) = host.instances.lock() {
let list: Vec < serde_json::Value > = instances.values().map( | inst| {
let has_window = inst.window.lock().ok().map( | g | g.is_some()).unwrap_or(false);
serde_json::json ! ({
"instance_id": inst.instance_id,
"instance_type": match inst.instance_type { InstanceType::Gtk => "gtk", InstanceType::Headless => "headless", InstanceType::Web => "web" },
"has_window": has_window,
})
}).collect();
serde_json::to_string(& list).map_err( | e | e.to_string())
} else {
Err("Failed to lock instances".to_string())
}
};
let _ = response.send(result);
}
```

---

## 8. Message-Broker Topic Interface

### 8.1 Load Instance via Broker

Any plugin or service can request a new instance by sending an `InstanceLoadMessage` on topic `core.instance.load`:

```
Topic:   core.instance.load
Payload: InstanceLoadMessage {
    instance_id: "side3",
    config_path: "config-side3.toml",
    response_topic: "core.instance.load.response",
}
```

The broker routes this to `LauncherHost::route_message()`, which detects the `core.instance.load` topic, extracts the payload, and calls `self.load_instance()`.

### 8.2 Stop Instance via Broker

```
Topic:   core.instance.stop
Payload: InstanceStopMessage {
    instance_id: "side3",
    response_topic: "core.instance.stop.response",
}
```

### 8.3 Using `send_message` MCP Tool

Since JSON converters are registered, the existing `send_message` MCP tool can also trigger instance load/stop by sending a JSON string payload:

```json
{
    "topic": "core.instance.load",
    "payload": {
        "instance_id": "side3",
        "config_path": "config-side3.toml",
        "response_topic": ""
    }
}
```

The broker's JSON converter registry converts the string payload to a typed `InstanceLoadMessage` before routing.

### 8.4 Instance Status Broadcast

After a successful load or stop, the host broadcasts an `InstanceStatusMessage` on topic `core.instance.status` to all instances and services:

```
Topic:   core.instance.status
Target:  * (broadcast)
Payload: InstanceStatusMessage {
    instance_id: "side3",
    event: InstanceLifecycleEvent::Loaded,
}
```

This allows:

- **Widgets** to react to new instances (e.g. update a workspace switcher).
- **Services** to register tools for new instance plugins.
- **Other instances** to send cross-instance messages to the new instance.

---

## 9. Config Loading

### 9.1 Config Path Allowlist

The `config_path` in `InstanceLoadMessage` is validated against an allowlist of directories. Only config files within the following directories are accepted:

1. **Current working directory** of the launcher process (the directory from which the binary was launched).
2. **`~/.config/smearor/`** (the user's Smearor configuration directory).

The `validate_config_path` function canonicalizes the path and checks `starts_with` against both allowed directories. Paths outside this allowlist are rejected
with an error. This prevents directory traversal attacks and unauthorized config file loading.

### 9.2 Config File Resolution

Within the allowed directories, both relative and absolute paths are accepted. Relative paths are resolved against the current working directory. This matches
the behavior of `--config` CLI arguments.

### 9.3 Config Validation

Before creating the instance, the host validates:

1. **Path allowlist**: Return error if the config path is outside allowed directories.
2. **Instance ID sanitization**: Return error if `instance_id` contains colons or path separators.
3. **File exists**: Return error if the file cannot be read.
4. **Parseable TOML**: Return error if the config is invalid.
5. **Unique instance_id**: Return error if an instance with the same ID already exists.
6. **Monitor index valid**: Warn (not error) if the configured monitor index exceeds the connected monitor count.

### 9.4 Config Example

```toml
# config-side3.toml
[launcher]
instance_id = "side3"

[launcher.layer]
namespace = "smearor-side3"
layer = "Top"
exclusive_zone = 50
monitor = 2

[[areas]]
id = "main"
area_type = "Scroll"
plugins = [
    { id = "clock", path = "target/release/libclock_widget.so" },
    { id = "app-launcher", path = "target/release/libapp_launcher_widget.so" },
]
```

---

## 10. Implementation Phases

### Phase 1: Model Crate — `model/instance-control`

**Order**: First. All other phases depend on the message types.

**Changes**:

- Create `model/instance-control/` with `InstanceLoadMessage`, `InstanceStopMessage`, `InstanceReloadMessage`, `InstanceStatusMessage`,
  `InstanceLifecycleEvent`.
- Define topics `TOPIC_CORE_INSTANCE_LOAD`, `TOPIC_CORE_INSTANCE_STOP`, `TOPIC_CORE_INSTANCE_RELOAD`, `TOPIC_CORE_INSTANCE_STATUS`.
- Implement `MessageTopic`, `SharedMessage`, `TypedMessage` traits.
- Implement `register_json_converters()` for JSON-to-typed conversion.
- All FFI types `#[stabby::stabby]`.
- Add to workspace `Cargo.toml`.

**Exit Criteria**: Crate compiles, exports all types, `register_json_converters()` callable.

### Phase 2: Host Methods — `load_instance` / `stop_instance` / `reload_instance`

**Order**: After Phase 1.

**Changes**:

- Add `LauncherHost::load_instance()` method to `host/mod.rs`.
- Add `LauncherHost::stop_instance()` method to `host/mod.rs`.
- Add `LauncherHost::reload_instance()` method (low priority, convenience wrapper).
- Add `LauncherHost::list_instances()` helper method.
- Add `validate_config_path()` and `validate_instance_id()` functions.
- Use `gtk4::glib::idle_add_local_once` for GTK operations (windows appear instantly, no animation).
- Call `calculate_coordinated_sizes()` after load/stop.
- Broadcast `InstanceStatusMessage` after successful load/stop/reload.
- Fully unload plugins on stop (shared libraries are fully unloaded, not cached).
- Unregister MCP tools registered by the stopped instance's plugins from `McpRegistry`.
- Register JSON converters from `model/instance-control` in `main.rs` startup.

**Exit Criteria**: `load_instance` creates a visible window from a config path; `stop_instance` closes the window, fully unloads plugins, and unregisters MCP
tools; coordinated sizes are recalculated.

### Phase 3: Broker Topic Routing

**Order**: After Phase 2.

**Changes**:

- Add `core.instance.load`, `core.instance.stop`, and `core.instance.reload` topic handlers in `route_message()`.
- Extract typed payload from envelope, call `load_instance` / `stop_instance` / `reload_instance`.
- Send optional response on `response_topic`.
- Broadcast `InstanceStatusMessage` on `core.instance.status` to all instances.

**Exit Criteria**: A plugin sending `InstanceLoadMessage` on `core.instance.load` causes a new instance to appear; `InstanceStopMessage` on `core.instance.stop`
causes it to disappear.

### Phase 4: MCP Server Tools

**Order**: After Phase 2.

**Changes**:

- Add `McpCommand::LoadInstance`, `McpCommand::StopInstance`, `McpCommand::ListInstances` variants to `mcp-server/src/lib.rs`.
- Add match arms in `send_command_and_wait()` in `tools.rs`.
- Add `launcher_load_instance`, `launcher_stop_instance`, `launcher_list_instances` tool definitions in `core_tools()`.
- Add `process_mcp_command` handlers in `main.rs`.
- When an instance is loaded, its plugins' MCP tool registrations are automatically forwarded to the MCP server via the existing `RegisterToolMessage` broker
  broadcast.
- When an instance is stopped, its plugins' MCP tools are unregistered from `McpRegistry` (see Phase 2).

**Exit Criteria**: MCP tools can load and stop instances; `launcher_list_instances` returns the current instance list; MCP tools from dynamically loaded
instances are available; MCP tools from stopped instances are removed.

### Phase 5: Instance Persistence

**Order**: After Phase 4.

**Changes**:

- Implement `LauncherHost::persist_instance()` and `LauncherHost::unpersist_instance()` methods.
- State file: `~/.config/smearor/instances.toml`.
- On startup in `main.rs`, after CLI instances are loaded, read `instances.toml` and load any persisted instances that are not already running.
- On `load_instance`: append entry to `instances.toml`.
- On `stop_instance`: remove entry from `instances.toml`.
- Format:

```toml
# ~/.config/smearor/instances.toml
# Persisted dynamic launcher instances.
# Automatically managed by the launcher — do not edit manually.

[[instances]]
instance_id = "side3"
config_path = "config-side3.toml"
instance_type = "gtk"

[[instances]]
instance_id = "macropad_5"
config_path = "/home/user/.config/smearor/config-macropad-5.toml"
instance_type = "headless"
```

**Exit Criteria**: Dynamically loaded instances survive process restart; stopped instances are not reloaded; state file is updated atomically (write to temp
file, then rename).

### Phase 6: Integration Tests

**Order**: After Phase 5.

**Changes**:

- Test: Load headless instance via MCP tool, verify no window but plugins are active.
- Test: Stop headless instance via MCP tool, verify plugins fully unloaded, MCP tools removed.
- Test: Load web instance via MCP tool, verify no window but HTTP endpoint responds.
- Test: Stop web instance via MCP tool, verify HTTP endpoint returns 404, WebSockets closed, plugins unloaded.
- Test: Load instance via broker topic (from a plugin), verify window appears.
- Test: Stop instance via broker topic, verify window disappears.
- Test: Hot-reload instance via `reload_instance`, verify window is replaced.
- Test: Hot-reload headless instance, verify instance type is preserved.
- Test: Hot-reload web instance, verify instance type is preserved and WebSockets reconnect.
- Test: Duplicate instance_id rejection.
- Test: Invalid config path error handling (path outside allowlist).
- Test: Config path allowlist enforcement (current dir + `~/.config/smearor/`).
- Test: Coordinated size recalculation after load/stop.
- Test: Cross-instance messaging after dynamic load.
- Test: MCP tools from loaded instance are registered; MCP tools from stopped instance are unregistered.
- Test: Instance persistence — restart process, verify persisted instances are reloaded.

**Exit Criteria**: All tests pass, no GTK thread-safety violations, no resource leaks, no stale MCP tools.

---

## 11. File Changes Summary

| File                                     | Change                                                                                                                                                                                                                                          |
|------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `model/instance-control/Cargo.toml`      | **New** — model crate manifest                                                                                                                                                                                                                  |
| `model/instance-control/src/lib.rs`      | **New** — message types, topics, JSON converters                                                                                                                                                                                                |
| `smearor-swipe-launcher/src/instance.rs` | Add `instance_type: InstanceType` field to `LauncherInstance` struct                                                                                                                                                                            |
| `smearor-swipe-launcher/src/host/mod.rs` | Add `load_instance()`, `stop_instance()`, `reload_instance()`, `list_instances()`, `persist_instance()`, `unpersist_instance()` methods; add `validate_config_path()`, `validate_instance_id()`; add broker topic handlers in `route_message()` |
| `smearor-swipe-launcher/src/main.rs`     | Register `model/instance-control` JSON converters; add `process_mcp_command` handlers for new `McpCommand` variants; load persisted instances from `~/.config/smearor/instances.toml` on startup                                                |
| `mcp-server/src/lib.rs`                  | Add `McpCommand::LoadInstance`, `McpCommand::StopInstance`, `McpCommand::ListInstances` variants                                                                                                                                                |
| `mcp-server/src/tools.rs`                | Add `launcher_load_instance`, `launcher_stop_instance`, `launcher_list_instances` tool definitions; extend `send_command_and_wait` match arms                                                                                                   |
| `model/mcp/src/registry.rs`              | Add `remove_tools_by_instance()` method to `McpRegistry`                                                                                                                                                                                        |
| `Cargo.toml` (workspace)                 | Add `model/instance-control` to workspace members                                                                                                                                                                                               |

---

## 12. Dependencies

### New Workspace Dependencies

```toml
dirs = "6"
```

`dirs` is needed for resolving `~/.config/smearor/` in the config path allowlist and persistence state file.

### Per-Crate

| Crate                    | Additional Dependencies                                              |
|--------------------------|----------------------------------------------------------------------|
| `model/instance-control` | `serde`, `serde_json`, `stabby`, `smearor-swipe-launcher-plugin-api` |
| `smearor-swipe-launcher` | `smearor-instance-control-model` (path dependency), `dirs`           |
| `mcp-server`             | No new dependencies (uses existing `McpCommand` enum)                |

---

## 13. Security Considerations

1. **Config path allowlist**: The `config_path` argument is validated against an allowlist of two directories: the current working directory and
   `~/.config/smearor/`. Paths outside this allowlist are rejected. This prevents directory traversal attacks and unauthorized config file loading. Implemented
   in `validate_config_path()`.

2. **MCP auth token**: The MCP server already supports an optional `auth_token`. If instance load/stop is exposed via MCP, the auth token must be set to prevent
   unauthorized instance manipulation.

3. **Instance ID sanitization**: The `instance_id` must be validated to contain only alphanumeric characters, hyphens, and underscores. No colons (which would
   conflict with the `instance_id:plugin_id` namespacing) or path separators. Implemented in `validate_instance_id()`.

4. **Resource limits**: A maximum instance count should be configurable to prevent resource exhaustion from excessive instance creation.

5. **MCP tool cleanup**: When an instance is stopped, all MCP tools registered by its plugins must be unregistered from the `McpRegistry` to prevent stale tool
   entries. Implemented via `McpRegistry::remove_tools_by_instance()`.

6. **Persistence file atomicity**: The `instances.toml` state file is written atomically (write to temp file, then rename) to prevent corruption on crash.

---

## 14. Resolved Decisions

| Question                          | Decision                                                                                                                                                                                                                              |
|-----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Hot-reload**                    | Implemented as `reload_instance()` / `core.instance.reload` topic (low priority). Convenience wrapper: stop + load with same ID.                                                                                                      |
| **Window animation**              | Windows appear instantly. No fade-in animation (GTK4 windows do not support transition animations).                                                                                                                                   |
| **Plugin shared library caching** | Fully unloaded. When an instance is stopped, its plugins are fully unloaded — shared libraries are not cached in memory.                                                                                                              |
| **MCP tool auto-registration**    | Yes. When a new instance loads, its plugins' MCP tools are automatically registered via the existing `RegisterToolMessage` broker broadcast. When an instance is stopped, its plugins' MCP tools are unregistered from `McpRegistry`. |
| **Instance persistence**          | Yes. Dynamically loaded instances are persisted to `~/.config/smearor/instances.toml` and automatically reloaded on next startup.                                                                                                     |
| **Headless instances**            | Supported via `InstanceType::Headless`. No window is built, no GTK operations are scheduled. Plugin and area lifecycle is identical to GTK instances. Used by MacroPad integration (see `STREAMDECK_CONCEPT.md`).                     |
| **Web instances**                 | Supported via `InstanceType::Web`. No window is built. Served via embedded HTTP server. Plugin and area lifecycle is identical. WebSockets are closed on stop. See `WEB_INSTANCE_CONCEPT.md`.                                         |
