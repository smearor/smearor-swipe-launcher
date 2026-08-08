# Concept: Launcher Instance Lifecycle

This document describes the concept for a **lifecycle state machine** for launcher instances, an **auto_start** config field, and the corresponding **MCP
tools** and **message broker routings** for controlling instance transitions at runtime.

---

## 1. Motivation

The Smearor Swipe Launcher currently supports multiple launcher instances, but their lifecycle is limited: an instance is either **created** (running with a
window or headless areas) or **removed** entirely. There is no way to load an instance without starting it, no way to stop an instance without unloading it, and
no way to open temporary launcher instances at runtime that disappear on the next restart.

The primary use case is **dynamic, temporary launcher instances** opened at runtime:

1. **Secondary menu**: A second launcher opens (e.g. a tools menu) while the first launcher remains visible — not as a transient area within the same instance,
   but as a separate launcher instance with its own window, plugins, and areas.
2. **On-demand sidebar**: A sidebar launcher can be opened on demand to show specific tools, then stopped when no longer needed.
3. **Overlay (Layer-Shell)**: A temporary overlay launcher opens on top of other windows for quick access.

These temporary instances should **not** be persisted — on the next launcher process restart, they should not reappear. They are loaded on demand, used while
needed, and then unloaded.

A secondary use case is **deferred-start config files**: A launcher config file placed in `~/.config/smearor/launcher/` with `auto_start = false` is discovered
and loaded at startup, but not started. It can be started later via MCP tool or broker message. Unlike temporary instances, these config-file instances **are**
persisted (in `Ready` state) so they survive restarts.

This creates three problems that the current architecture cannot solve:

1. **No transient instances**: `load_instance` always calls `persist_instance`, writing the instance to `instances.toml`. Temporary instances opened at runtime
   would reappear after restart — undesirable for dynamic, on-demand launchers.
2. **No deferred start**: A launcher config file placed in `~/.config/smearor/launcher/` is always auto-started. There is no way to keep a config file there
   that should only be started later (e.g. a sidebar that is loaded but not visible until needed).
3. **No stop-without-unload**: Stopping a running instance also unloads all plugins, removes config watchers, and deletes the instance from the `instances` map.
   If the user wants to restart it later, the entire config must be re-parsed and all plugins re-loaded. This is wasteful for instances that are stopped and
   started frequently.

The solution introduces:

- A **lifecycle state machine** with six states and well-defined transitions
- An **auto_start** config field to control whether a loaded instance starts automatically
- A **persist** flag on `load_instance` to distinguish transient (runtime) from persistent (config-file) instances
- An **auto_stop_ttl** config field to automatically stop an instance after a configurable time-to-live
- **auto_start_topic** and **auto_stop_topic** config fields for event-driven lifecycle control via the message broker
- Four symmetric MCP tools (`load`, `start`, `stop`, `unload`) plus a `list` tool that reports the lifecycle state of each instance

### 1.1 Auto-Stop TTL Motivation

The `auto_stop_ttl` feature addresses two concrete use cases:

1. **Sub-menu launcher instances**: When a user selects a category (e.g. "Games"), instead of switching areas within the same instance, a **separate transient
   launcher instance** opens showing the sub-menu. This instance should automatically close after a period of inactivity — the user does not need to manually
   dismiss it. Without `auto_stop_ttl`, these sub-menu instances would remain open indefinitely, cluttering the screen and consuming resources.

2. **OSD-style launcher instances**: Transient launcher instances with a TTL can serve as **on-screen displays** (OSDs). For example, a volume-control OSD
   appears when the volume changes, displays the current level, and automatically disappears after a few seconds. The TTL ensures the information is shown only
   temporarily without requiring the user to close it manually. This pattern is also useful for notification banners, media-player controls, or battery-status
   overlays.

### 1.2 Event-Driven Auto-Start / Auto-Stop Motivation

For OSD-style instances to work autonomously, they need to open in response to external events — not just manual MCP or broker commands. The `auto_start_topic`
and `auto_stop_topic` config fields enable **event-driven lifecycle control** via the message broker:

1. **Reactive OSDs**: The audio service broadcasts a volume-change status on `audio.status.volume`. An OSD launcher instance configured with
   `auto_start_topic = "audio.status.volume"` automatically opens when the volume changes, displays the new level, and auto-stops after its TTL expires. No
   manual intervention or external script is needed.

2. **Symmetric event-driven stop**: An `auto_stop_topic` allows external events to stop a running instance before its TTL expires. For example, a "display-off"
   event could stop all OSD instances immediately rather than waiting for their timers.

3. **Decoupled architecture**: Services do not need to know about launcher instances — they simply broadcast status on their topics. Launcher instances
   subscribe to the topics they care about via config, maintaining separation of concerns between services and the launcher.

---

## 2. Affected Crates

| Crate             | Path                      | Responsibility                                                                                                                                                 |
|-------------------|---------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Model**         | `model/instance-control/` | Lifecycle enum, message types, topics, JSON converters                                                                                                         |
| **Launcher Core** | `smearor-swipe-launcher/` | Lifecycle field on `LauncherInstance`, transition methods on `LauncherHost`, broker routing, auto_start config field, startup loop changes, REST API endpoints |
| **MCP Server**    | `mcp-server/`             | `McpCommand` variants, tool definitions, `send_command_and_wait` match arms                                                                                    |

This feature does not introduce new crates. It extends the existing `model/instance-control` crate, the launcher core (including the embedded web server in
`smearor-swipe-launcher/src/web/`), and the MCP server. There is no service or widget crate because instance lifecycle management is an internal responsibility
of the launcher host, not a plugin or service feature.

---

## 3. Model Crate (`model/instance-control`)

### 3.1 Lifecycle State Enum

The existing `InstanceLifecycleEvent` enum (Loaded/Stopped/Reloaded) is replaced by a comprehensive state enum. This enum represents the **current state** of an
instance, not just a transition event. It is used both as a field on `LauncherInstance` and as the `event` field in `InstanceStatusMessage` broadcasts.

```rust
/// Lifecycle states for launcher instances.
///
/// Each instance transitions through these states during its lifetime.
/// Invalid transitions are rejected by the launcher host.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherInstanceLifecycle {
    /// Instance is currently being loaded (config parsed, plugins loading).
    Loading,
    /// Instance is loaded but not running. Plugins are loaded, config is watched,
    /// but no window or headless areas have been built.
    #[default]
    Ready,
    /// Instance is currently being started (window or headless areas being built).
    Starting,
    /// Instance is running. Window is visible (GTK) or headless areas are active.
    Running,
    /// Instance is currently being stopped (window closing, areas being removed).
    Stopping,
    /// Instance is currently being unloaded (plugins being unloaded, watchers removed).
    Unloading,
}
```

**State transitions:**

```
load_instance:     Loading → Ready
start_instance:    Ready → Starting → Running
stop_instance:     Running → Stopping → Ready
unload_instance:   Ready → Unloading (instance removed)
```

Invalid transitions (e.g. `start_instance` on a `Running` instance, `unload_instance` on a `Running` instance) return a
`LauncherInstanceLifecycleTransitionError`.

**Intermediate states** (`Loading`, `Starting`, `Stopping`, `Unloading`) are transient — they exist only during the execution of a lifecycle method. If the
method fails (error or panic), the state must roll back to the previous stable state (`Ready` or `Running`). This is enforced via a RAII guard pattern
(see [§ 4.3 Lifecycle Transaction Guard](#43-lifecycle-transaction-guard)).

The `as_str()` and `from_str()` methods map each variant to its lowercase string representation (`"loading"`, `"ready"`, `"starting"`, `"running"`,
`"stopping"`, `"unloading"`).

### 3.2 Message Topics

Two new topics are added alongside the existing `core.instance.load`, `core.instance.stop`, and `core.instance.reload`:

```rust
/// Topic to dynamically start a loaded (Ready) launcher instance.
pub const TOPIC_CORE_INSTANCE_START: &str = "core.instance.start";
/// Topic to dynamically unload a stopped (Ready) launcher instance.
pub const TOPIC_CORE_INSTANCE_UNLOAD: &str = "core.instance.unload";
```

The existing `TOPIC_CORE_INSTANCE_STATUS` remains unchanged — it broadcasts `InstanceStatusMessage` with the new lifecycle state as the `event` field.

### 3.3 Start Message

```rust
/// Message to dynamically start a loaded (Ready) launcher instance.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceStartMessage {
    /// Unique identifier of the instance to start.
    pub instance_id: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}
```

### 3.3a Load Message (Updated)

The existing `InstanceLoadMessage` gains a `persist` field to distinguish transient instances (runtime, not persisted) from persistent instances (config-file,
persisted to `instances.toml`):

```rust
/// Message to dynamically load a new launcher instance from a config file.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceLoadMessage {
    /// Unique identifier for the new instance.
    pub instance_id: stabby::string::String,
    /// File system path to the TOML config file for this instance.
    pub config_path: stabby::string::String,
    /// Whether to create a GTK window (Gtk) or run headless (Headless) or web (Web).
    pub instance_type: InstanceType,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
    /// Whether to persist this instance to the state file.
    /// Set to `false` for transient instances (opened at runtime, not restored on restart).
    /// Set to `true` for persistent instances (config-file instances that should survive restarts).
    /// Defaults to `false` for backward compatibility with existing broker callers.
    #[serde(default)]
    pub persist: bool,
}
```

The `InstanceLoadMessage::new` constructor is extended with a `persist` parameter.

### 3.4 Unload Message

```rust
/// Message to dynamically unload a stopped (Ready) launcher instance.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceUnloadMessage {
    /// Unique identifier of the instance to unload.
    pub instance_id: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}
```

### 3.5 Status Message (Updated)

The existing `InstanceStatusMessage` is updated to use `LauncherInstanceLifecycle` instead of `InstanceLifecycleEvent`:

```rust
/// Status message broadcast when an instance changes its lifecycle state.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceStatusMessage {
    /// The instance ID that changed.
    pub instance_id: stabby::string::String,
    /// The new lifecycle state.
    pub event: LauncherInstanceLifecycle,
}
```

### 3.6 JSON Converters

New converters for `InstanceStartMessage` and `InstanceUnloadMessage` are added. The `InstanceStatusMessageConverter` is updated to parse
`LauncherInstanceLifecycle` instead of `InstanceLifecycleEvent`:

```rust
impl_json_convertible!(InstanceStartMessageConverter, InstanceStartMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(InstanceUnloadMessageConverter, InstanceUnloadMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
```

The `register_json_converters` function is extended to register the two new converters.

### 3.7 File Structure

The `model/instance-control/src/` directory gains two new files:

| File                 | Content                                                                   |
|----------------------|---------------------------------------------------------------------------|
| `lifecycle_event.rs` | **Replaced**: `InstanceLifecycleEvent` → `LauncherInstanceLifecycle` enum |
| `start_message.rs`   | **New**: `InstanceStartMessage`                                           |
| `unload_message.rs`  | **New**: `InstanceUnloadMessage`                                          |
| `topics.rs`          | **Extended**: `TOPIC_CORE_INSTANCE_START`, `TOPIC_CORE_INSTANCE_UNLOAD`   |
| `json_converters.rs` | **Extended**: Converters for Start + Unload; Status converter updated     |
| `lib.rs`             | **Extended**: Re-exports for new types                                    |

---

## 4. Launcher Core (`smearor-swipe-launcher`)

### 4.1 Auto-Start Config Field

`SwipeLauncherSettings` in `smearor-swipe-launcher/src/config/launcher.rs` gains a new field:

```rust
/// Whether this instance should automatically start (build its window or headless areas)
/// when the launcher process starts or when the instance is dynamically loaded.
/// Defaults to `true` for backward compatibility.
/// Set to `false` to load the instance into `Ready` state without starting it.
#[serde(default = "default_true")]
pub auto_start: bool,
```

A `default_true()` function returns `true` so existing configs without the field continue to auto-start.

#### Auto-Stop TTL

`SwipeLauncherSettings` gains an additional field for automatic stop after a time-to-live (TTL):

```rust
/// Optional time-to-live in seconds after which a running instance is automatically stopped.
/// When the TTL expires, the launcher calls `stop_instance` internally, returning the instance to `Ready` state.
/// The instance is not unloaded — it can be started again manually.
/// Defaults to `None` (no auto-stop).
/// Set to a duration in seconds to automatically stop the instance after that time.
/// Useful for transient overlays, temporary menus, or secondary launchers that should auto-close.
#[serde(default)]
pub auto_stop_ttl: Option<u64>,
```

The field is optional (`Option<u64>`). When `None` or absent, no TTL timer is started. When set to a number of seconds, `start_instance` spawns a `tokio::spawn`
task that sleeps for the specified duration and then calls `stop_instance` on that instance. The timer task handle is stored on `LauncherInstance` so it can be
cancelled if the instance is stopped or unloaded before the TTL expires.

#### Auto-Start / Auto-Stop Topics

`SwipeLauncherSettings` gains two additional fields for event-driven lifecycle control via the message broker:

```rust
/// Optional message broker topic that triggers `start_instance` when a message is received.
/// When configured, the launcher subscribes to this topic on the message broker.
/// Any message sent to this topic causes the instance to start (if it is in `Ready` state).
/// This enables event-driven instance activation — e.g. an audio service sends a volume-change
/// status message, and the OSD launcher instance opens automatically because its
/// `auto_start_topic` matches the audio status topic.
/// Defaults to `None` (no event-driven auto-start).
#[serde(default)]
pub auto_start_topic: Option<String>,
/// Optional message broker topic that triggers `stop_instance` when a message is received.
/// When configured, the launcher subscribes to this topic on the message broker.
/// Any message sent to this topic causes the instance to stop (if it is in `Running` state).
/// This enables event-driven instance deactivation — e.g. a "hide-osd" event stops the OSD
/// instance without waiting for the TTL to expire.
/// Defaults to `None` (no event-driven auto-stop).
#[serde(default)]
pub auto_stop_topic: Option<String>,
```

Both fields are optional (`Option<String>`). When `None` or absent, no broker subscription is created for that direction. When set, the launcher subscribes to
the configured topic during `load_instance` and unsubscribes during `unload_instance`.

**Example flow:**

1. The audio service sends a volume-change status on topic `audio.status.volume`
2. The OSD launcher instance has `auto_start_topic = "audio.status.volume"` in its config
3. The launcher's broker subscription receives the message and calls `start_instance` — the OSD window appears
4. After `auto_stop_ttl` seconds (or when a message arrives on `auto_stop_topic`), the instance is stopped and returns to `Ready`
5. The next volume-change message re-opens the OSD

**Idempotency:** If a message arrives on `auto_start_topic` while the instance is already `Running`, the start is a no-op (the transition `Running → Starting`
is invalid and silently ignored). Similarly, a message on `auto_stop_topic` while the instance is `Ready` is silently ignored. This prevents error spam from
frequent status messages.

### 4.2 Lifecycle Field on LauncherInstance

`LauncherInstance` in `smearor-swipe-launcher/src/instance/launcher_instance.rs` gains a new field:

```rust
/// Current lifecycle state of this instance.
pub(crate) lifecycle: Mutex<LauncherInstanceLifecycle>,
/// Optional auto-stop TTL timer task handle.
/// When `start_instance` is called and `auto_stop_ttl` is configured,
/// a `tokio::spawn` task is started that calls `stop_instance` after the TTL expires.
/// Storing the handle allows `stop_instance` and `unload_instance` to cancel the timer
/// if the instance is stopped or unloaded before the TTL fires.
pub(crate) auto_stop_handle: Mutex<Option<tokio::task::JoinHandle<() > > >,
```

The `lifecycle` field is initialized to `Ready` in `LauncherInstance::new`. The `auto_stop_handle` is initialized to `None`. The `LauncherInstanceLifecycle`
enum provides two member methods for transition validation:

```rust
/// Error returned when a lifecycle transition is not allowed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LauncherInstanceLifecycleTransitionError {
    /// The transition from `current` to `target` is not a valid state transition.
    #[error("Invalid lifecycle transition: {current:?} → {target:?}")]
    InvalidTransition {
        current: LauncherInstanceLifecycle,
        target: LauncherInstanceLifecycle,
    },
}

impl LauncherInstanceLifecycle {
    /// Validates that transitioning FROM `current` TO `self` is allowed.
    /// `self` is the target state.
    ///
    /// # Example
    /// ```
    /// let current = LauncherInstanceLifecycle::Ready;
    /// let target = LauncherInstanceLifecycle::Starting;
    /// target.validate_transition_from(current)?; // Ok
    /// ```
    pub fn validate_transition_from(
        &self,
        current: LauncherInstanceLifecycle,
    ) -> Result<(), LauncherInstanceLifecycleTransitionError> {
        match (current, *self) {
            (Loading, Ready) => Ok(()),
            (Ready, Starting) => Ok(()),
            (Starting, Running) => Ok(()),
            (Running, Stopping) => Ok(()),
            (Stopping, Ready) => Ok(()),
            (Ready, Unloading) => Ok(()),
            _ => Err(LauncherInstanceLifecycleTransitionError::InvalidTransition {
                current,
                target: *self,
            }),
        }
    }

    /// Validates that transitioning FROM `self` TO `target` is allowed.
    /// `self` is the current state.
    ///
    /// # Example
    /// ```
    /// let current = LauncherInstanceLifecycle::Ready;
    /// let target = LauncherInstanceLifecycle::Starting;
    /// current.validate_transition_to(target)?; // Ok
    /// ```
    pub fn validate_transition_to(
        &self,
        target: LauncherInstanceLifecycle,
    ) -> Result<(), LauncherInstanceLifecycleTransitionError> {
        match (*self, target) {
            (Loading, Ready) => Ok(()),
            (Ready, Starting) => Ok(()),
            (Starting, Running) => Ok(()),
            (Running, Stopping) => Ok(()),
            (Stopping, Ready) => Ok(()),
            (Ready, Unloading) => Ok(()),
            _ => Err(LauncherInstanceLifecycleTransitionError::InvalidTransition {
                current: *self,
                target,
            }),
        }
    }
}
```

The `LauncherInstanceLifecycleTransitionError` type is defined in `model/instance-control/src/lifecycle_event.rs` alongside the enum. The `thiserror` dependency
is already present in the workspace.

### 4.3 Lifecycle Transaction Guard

Intermediate lifecycle states (`Loading`, `Starting`, `Stopping`, `Unloading`) are transient. If a lifecycle method fails (returns `Err` or panics during
`build_window`, `build_headless`, or cleanup), the instance must not remain stuck in the intermediate state — otherwise all subsequent transitions are blocked
because the validation matrix only allows specific transitions from each state.

The solution is a **RAII guard** that automatically rolls back the lifecycle state to the previous stable state if the method does not explicitly complete the
transition. The guard is defined in `smearor-swipe-launcher/src/instance/lifecycle.rs`:

```rust
/// RAII guard for lifecycle transitions.
///
/// On construction, stores the rollback state (the previous stable state).
/// If the guard is dropped without calling `complete()`, the lifecycle
/// is rolled back to the rollback state.
///
/// This ensures that intermediate states (Starting, Stopping, Unloading)
/// are never left dangling if a transition fails or panics.
pub struct LifecycleGuard<'a> {
    lifecycle: &'a Mutex<LauncherInstanceLifecycle>,
    rollback_state: LauncherInstanceLifecycle,
    completed: bool,
}

impl<'a> LifecycleGuard<'a> {
    /// Create a new guard. The current state should already be set to the
    /// intermediate state (e.g. `Starting`). The `rollback_state` is the
    /// state to restore if the transition fails (e.g. `Ready`).
    pub fn new(
        lifecycle: &'a Mutex<LauncherInstanceLifecycle>,
        rollback_state: LauncherInstanceLifecycle,
    ) -> Self {
        Self {
            lifecycle,
            rollback_state,
            completed: false,
        }
    }

    /// Mark the transition as completed. The guard will not roll back on drop.
    pub fn complete(mut self) {
        self.completed = true;
    }
}

impl<'a> Drop for LifecycleGuard<'a> {
    fn drop(&mut self) {
        if !self.completed {
            if let Ok(mut state) = self.lifecycle.lock() {
                *state = self.rollback_state;
                debug!(
                    "Lifecycle transition failed, rolled back to {:?}",
                    self.rollback_state
                );
            }
        }
    }
}
```

**Usage pattern in `start_instance`:**

```rust
// State is now `Starting`
let guard = LifecycleGuard::new( & instance.lifecycle, LauncherInstanceLifecycle::Ready);

// Build window — may fail or panic
let window = instance.build_window(app) ?; // or build_headless()

// Success — commit the transition
* instance.lifecycle.lock() = LauncherInstanceLifecycle::Running;
guard.complete(); // Prevents rollback on drop
```

If `build_window` returns `Err` or panics, `guard` is dropped without `complete()`, and the `Drop` implementation rolls the state back to `Ready`.

**Guard placement per method:**

| Method            | Intermediate State | Rollback State                  | Completed State      |
|-------------------|--------------------|---------------------------------|----------------------|
| `load_instance`   | `Loading`          | *(instance removed on failure)* | `Ready`              |
| `start_instance`  | `Starting`         | `Ready`                         | `Running`            |
| `stop_instance`   | `Stopping`         | `Running`                       | `Ready`              |
| `unload_instance` | `Unloading`        | `Ready`                         | *(instance removed)* |

**Note on `load_instance`:** If `load_instance` fails, the partially-created instance is removed from the `instances` map entirely (no rollback to a previous
state, since there is no previous state). The guard is not needed here — instead, the method cleans up and removes the instance on error.

**Note on `unload_instance`:** If `unload_instance` fails, the guard rolls back to `Ready` and the instance remains in the map. On success, `complete()` is
called before the instance is removed from the map.

### 4.4 Persisted Instance State

`PersistedInstance` in `smearor-swipe-launcher/src/instance/persisted_instance.rs` gains a `lifecycle` field:

```rust
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PersistedInstance {
    pub instance_id: String,
    pub config_path: String,
    pub instance_type: String,
    /// Lifecycle state at time of persistence: "ready" or "running".
    #[serde(default = "default_lifecycle")]
    pub lifecycle: String,
}
```

The default is `"running"` for backward compatibility with existing state files that do not contain the field.

### 4.5 LauncherHost Methods

#### 4.5.1 `load_instance` (Refactored)

`load_instance` becomes a pure **load** operation. It no longer builds windows or headless areas. It gains a `persist: bool` parameter to control whether the
instance is written to the state file.

**Responsibilities:**

1. Validate config path and instance ID
2. Parse config, resolve defaults and includes
3. Check for duplicate instance ID
4. Set lifecycle to `Loading`
5. Call `create_instance` (creates `LauncherInstance`, loads plugins)
6. Register config watcher, CSS watcher (GTK only)
7. Set lifecycle to `Ready`
8. If `persist == true`: persist instance with `lifecycle = "ready"`
9. Broadcast `InstanceStatusMessage` with `Ready`
10. If `config.launcher.auto_start == true`: call `start_instance`

**No longer does:**

- `build_window` / `build_headless`
- `calculate_coordinated_sizes` (moved to `start_instance`)
- Unconditional `persist_instance` — now controlled by the `persist` parameter

**Persistence rules:**

- **Startup loop** (config files discovered at startup): `persist = true` — these are persistent instances that should survive restarts
- **Runtime load via MCP tool**: `persist` parameter on the tool, defaults to `false` — transient by default
- **Runtime load via broker message**: `persist` field on `InstanceLoadMessage`, defaults to `false` — transient by default

#### 4.5.2 `start_instance` (New)

Starts a `Ready` instance, building its window or headless areas.

**Responsibilities:**

1. Lock instances map, find instance by ID
2. Read current lifecycle; validate transition `Ready → Starting`
3. Set lifecycle to `Starting`
4. Cancel any previous TTL timer if still present: lock `auto_stop_handle`, take the `JoinHandle`, call `abort()` on it — this prevents a stale sleep task from
   a prior run from prematurely closing the newly started instance
5. Acquire `LifecycleGuard` with rollback state `Ready` (see [§ 4.3](#43-lifecycle-transaction-guard))
6. For GTK: `build_window` via `idle_add_local_once` (must run on main thread)
7. For Headless/Web: `build_headless`
8. Set lifecycle to `Running`; mark guard as completed
9. `calculate_coordinated_sizes`
10. Broadcast `InstanceStatusMessage` with `Running`
11. If instance is persisted: update persisted state to `lifecycle = "running"`
12. If `config.launcher.auto_stop_ttl` is set: spawn a `tokio::spawn` task that sleeps for the TTL duration and then calls `stop_instance` on this instance ID;
    store the `JoinHandle` in `instance.auto_stop_handle`
13. Return success message

**Note:** Transient instances (not persisted) skip step 10. Their lifecycle state is tracked in memory only.

**Auto-stop TTL:** The timer task holds a weak reference to the `LauncherHost` (or uses the broker sender) to avoid keeping the host alive. When the TTL fires,
it calls `stop_instance` internally — the instance returns to `Ready` and can be started again. If the instance is stopped or unloaded before the TTL expires,
the timer is cancelled (see `stop_instance` step 4 and `unload_instance` step 4).

**Stale timer prevention on re-start:** When an instance is stopped (either manually or by TTL expiry) and then started again before the original timer handle
has been cleared, step 4 ensures the old handle is aborted before spawning a new one. Without this, the old sleep task could fire after the new start and
prematurely close the instance. The pattern is:

```rust
if let Some(old_handle) = instance.auto_stop_handle.lock().unwrap().take() {
old_handle.abort();
}
```

This is safe because `take()` removes the handle from the slot, and `abort()` on an already-completed task is a no-op.

**Failure handling:** If `build_window` or `build_headless` fails (returns `Err` or panics), the `LifecycleGuard` rolls the lifecycle state back to `Ready`
automatically. The error is returned to the caller. The instance remains in `Ready` state and can be retried.

**Error cases:**

- Instance not found
- Instance is not in `Ready` state (already running, loading, etc.)
- `build_window` or `build_headless` fails → state rolled back to `Ready`, error returned

#### 4.5.3 `stop_instance` (Refactored)

`stop_instance` becomes a pure **stop** operation. It no longer removes the instance from the `instances` map or unloads plugins.

**Responsibilities:**

1. Lock instances map, find instance by ID (do not remove)
2. Read current lifecycle; validate transition `Running → Stopping`
3. Set lifecycle to `Stopping`
4. Cancel auto-stop TTL timer if active: lock `auto_stop_handle`, take the `JoinHandle`, call `abort()` on it
5. Acquire `LifecycleGuard` with rollback state `Running` (see [§ 4.3](#43-lifecycle-transaction-guard))
6. For GTK: disconnect close handler, close window, remove areas (via `idle_add_local_once`)
7. For Web: unregister from web server
8. For Headless: remove areas
9. Set lifecycle to `Ready`; mark guard as completed
10. `calculate_coordinated_sizes`
11. Broadcast `InstanceStatusMessage` with `Ready`
12. If instance is persisted: update persisted state to `lifecycle = "ready"`
13. Return success message

**Failure handling:** If closing the window or removing areas fails, the `LifecycleGuard` rolls the lifecycle state back to `Running` automatically. The
instance remains in `Running` state and the stop can be retried.

**No longer does:**

- `plugin_manager.unload_plugins()` (moved to `unload_instance`)
- `mcp_registry.remove_tools_by_instance()` (moved to `unload_instance`)
- `config_watcher.remove_instance()` (moved to `unload_instance`)
- `css_watcher.remove_instance_css()` (moved to `unload_instance`)
- Remove from `instances` map (moved to `unload_instance`)
- `unpersist_instance()` (moved to `unload_instance`)

#### 4.5.4 `unload_instance` (New)

Completely removes a `Ready` instance. This is the symmetric counterpart to `load_instance`.

**Responsibilities:**

1. Lock instances map, find instance by ID
2. Read current lifecycle; validate transition `Ready → Unloading`
3. Set lifecycle to `Unloading`
4. Cancel auto-stop TTL timer if active: lock `auto_stop_handle`, take the `JoinHandle`, call `abort()` on it
5. Acquire `LifecycleGuard` with rollback state `Ready` (see [§ 4.3](#43-lifecycle-transaction-guard))
6. `plugin_manager.unload_plugins()`
7. `mcp_registry.remove_tools_by_instance()`
8. `mcp_registry.remove_resources_by_instance()`
9. `mcp_registry.remove_prompts_by_instance()`
10. `config_watcher.remove_instance()`
11. For GTK: `css_watcher.remove_instance_css()` (if config path available)
12. Remove instance from `instances` map
13. If instance was persisted: `unpersist_instance()`
14. Mark guard as completed (no rollback — instance is removed)
15. Return success message

**Failure handling:** If any cleanup step fails, the `LifecycleGuard` rolls the lifecycle state back to `Ready` and the instance remains in the `instances` map.
The unload can be retried.

**Note:** Transient instances (not persisted) skip step 12. They were never written to the state file.

**Error cases:**

- Instance not found
- Instance is not in `Ready` state (must be stopped first)

#### 4.5.5 `reload_instance` (Refactored)

`reload_instance` is updated to use the new symmetric operations. It **preserves the previous lifecycle state** rather than blindly following the `auto_start`
flag in the (possibly changed) config file. This is critical because the config watcher fires `reload_instance` whenever the TOML file changes on disk — if the
instance was in `Ready` state (stopped, `auto_start = false`), it must remain in `Ready` after reload, even if the user changed `auto_start` to `true` in the
file.

**Responsibilities:**

1. Read current instance type and lifecycle state (`previous_state`)
2. If `Running`: call `stop_instance` first
3. Call `unload_instance`
4. Call `load_instance` with `persist = true` and `auto_start = false` (suppress auto-start)
5. If `previous_state == Running`: call `start_instance` to restore the running state
6. If `previous_state == Ready`: leave the instance in `Ready` state
7. Broadcast `InstanceStatusMessage` with the resulting state

**Why not rely on `auto_start`?** The config file may have been edited with a different `auto_start` value. The reload should preserve the user's runtime intent
(stopped or running), not the config file's preference. A stopped instance should not suddenly start just because the file was edited.

**Edge case — `auto_start` changed from `false` to `true` in config:** The instance remains in its previous state after reload. The new `auto_start` value only
takes effect on the next explicit `load_instance` or process restart. This is intentional — a file edit should not override the user's runtime lifecycle
decision.

#### 4.5.6 `list_instances` (Updated)

The `list_instances` method is updated to include the lifecycle state:

```json
[
  {
    "instance_id": "main",
    "instance_type": "gtk",
    "has_window": true,
    "lifecycle": "running"
  },
  {
    "instance_id": "side3",
    "instance_type": "gtk",
    "has_window": false,
    "lifecycle": "ready"
  }
]
```

The `lifecycle` field allows MCP tool consumers to distinguish between loaded-but-not-started instances and running instances.

#### 4.5.7 `load_persisted_instances` (Updated)

When loading persisted instances on startup:

1. Read `entry.lifecycle` from the state file
2. Call `load_instance` with `persist = true` (which loads into `Ready` state)
3. If `entry.lifecycle == "running"` and `auto_start == true` in the config: call `start_instance`
4. If `entry.lifecycle == "ready"`: leave in `Ready` state

This ensures that persistent instances that were running before a restart are restored to `Running`, and persistent instances that were stopped are restored to
`Ready`. Transient instances are not in the state file and therefore not restored.

### 4.5 Broker Routing

Two new routing blocks are added in `route_message` in `host/mod.rs`, alongside the existing `core.instance.load`, `core.instance.stop`, and
`core.instance.reload` blocks:

```rust
if topic == TOPIC_CORE_INSTANCE_START {
if ! envelope.payload.is_null() {
let msg = unsafe { & * (envelope.payload as * const InstanceStartMessage) };
let instance_id = msg.instance_id.to_string();
let response_topic = msg.response_topic.to_string();
let result = self.start_instance(& instance_id);
self.send_broker_response( & response_topic, & result);
}
return;
}
if topic == TOPIC_CORE_INSTANCE_UNLOAD {
if ! envelope.payload.is_null() {
let msg = unsafe { & * (envelope.payload as * const InstanceUnloadMessage) };
let instance_id = msg.instance_id.to_string();
let response_topic = msg.response_topic.to_string();
let result = self.unload_instance(& instance_id);
self.send_broker_response( & response_topic, & result);
}
return;
}
```

### 4.6 Startup Loop (`main.rs`)

The startup loop in `main.rs` is adjusted:

1. For each discovered config path: call `host.load_instance` with `persist = true`
2. `load_instance` internally checks `auto_start` — if `true`, it calls `start_instance` automatically
3. If `auto_start = false`: the instance remains in `Ready` state and can be started later via MCP tool or broker message

The existing code that calls `host.create_instance` followed by `build_headless` / `css_watcher.watch_instance_css` in the startup loop is replaced by a single
`host.load_instance` call, which encapsulates all of that logic.

Config-file instances are always persistent (`persist = true`) — they survive restarts. Transient instances loaded at runtime via MCP or broker are not
persistent by default.

---

## 5. MCP Server (`mcp-server`)

### 5.1 New McpCommand Variants

Two new variants are added to the `McpCommand` enum in `mcp-server/src/lib.rs`:

```rust
/// Start a loaded (Ready) launcher instance.
StartInstance {
instance_id: String,
response: oneshot::Sender<Result<String, String> >,
},
/// Unload a stopped (Ready) launcher instance.
UnloadInstance {
instance_id: String,
response: oneshot::Sender<Result<String, String> >,
},
```

### 5.2 Tool Definitions

Four symmetric tools are defined in `mcp-server/src/tools.rs`:

| Tool                       | Description                                                                                                                                                            | Parameters                                                                                                                         |
|----------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `launcher_load_instance`   | Loads a new instance from a TOML config file into Ready state. Automatically starts it if `auto_start = true` in the config. Set `persist = true` to survive restarts. | `instance_id` (required), `config_path` (required), `instance_type` (optional, default "gtk"), `persist` (optional, default false) |
| `launcher_start_instance`  | Starts a loaded (Ready) instance — builds its window or headless areas.                                                                                                | `instance_id` (required)                                                                                                           |
| `launcher_stop_instance`   | Stops a running instance — closes its window and removes areas, returning it to Ready state. Plugins remain loaded.                                                    | `instance_id` (required)                                                                                                           |
| `launcher_unload_instance` | Unloads a stopped (Ready) instance — unloads plugins, removes config watchers, and deletes the instance.                                                               | `instance_id` (required)                                                                                                           |
| `launcher_list_instances`  | Lists all loaded instances with their ID, type, window state, and lifecycle state.                                                                                     | (none)                                                                                                                             |

### 5.3 send_command_and_wait

The `send_command_and_wait` function in `tools.rs` is extended with two new match arms for `StartInstance` and `UnloadInstance`, following the same pattern as
the existing `StopInstance` and `LoadInstance` arms.

### 5.4 process_mcp_command

The `process_mcp_command` function in `main.rs` is extended with two new match arms:

```rust
McpCommand::StartInstance { instance_id, response } => {
let result = host.start_instance( & instance_id);
let _ = response.send(result);
}
McpCommand::UnloadInstance { instance_id, response } => {
let result = host.unload_instance( & instance_id);
let _ = response.send(result);
}
```

---

## 6. REST API (Web Server)

The embedded web server (`smearor-swipe-launcher/src/web/`) already serves web launcher instances via axum routes. It is extended with REST endpoints for
instance lifecycle control, allowing external tools, scripts, and dashboards to manage launcher instances via HTTP without MCP or broker access.

### 6.1 Architecture

The existing `WebAppState` in `smearor-swipe-launcher/src/web/routes.rs` already holds `instances: Arc<Mutex<HashMap<String, LauncherInstance>>>` and
`broker_sender: UnboundedSender<FfiEnvelope>`. To call `LauncherHost` lifecycle methods (`load_instance`, `start_instance`, `stop_instance`, `unload_instance`,
`list_instances`), the `WebAppState` gains a command channel:

```rust
pub struct WebAppState {
    pub instances: Arc<Mutex<HashMap<String, LauncherInstance>>>,
    pub broker_sender: UnboundedSender<FfiEnvelope>,
    pub template_engine: TemplateEngine,
    pub ws_manager: Arc<WebSocketManager>,
    /// Channel to send lifecycle commands to the LauncherHost.
    /// Reuses the same McpCommand enum and process_mcp_command pipeline
    /// as the MCP server, ensuring consistent behavior between MCP and REST.
    pub host_command_sender: async_channel::UnboundedSender<McpCommand>,
}
```

The `host_command_sender` is the same `async_channel::Sender<McpCommand>` already used by the MCP server. In `main.rs`, the existing sender is passed to
`WebServer::new()` as a separate parameter (not part of `WebServerConfig`, since config structs should not hold runtime channels). This means REST endpoints and
MCP tools share the same command processing pipeline (`process_mcp_command` in `main.rs`), ensuring identical behavior and validation.

### 6.2 Endpoints

All endpoints are mounted under `/api/` to distinguish them from the existing web instance serving routes (`/instances/{id}`). They are added to
`build_router()` in `smearor-swipe-launcher/src/web/server.rs`:

| Method   | Path                         | Description                                    | McpCommand Variant |
|----------|------------------------------|------------------------------------------------|--------------------|
| `GET`    | `/api/instances`             | List all loaded instances with lifecycle state | `ListInstances`    |
| `POST`   | `/api/instances`             | Load a new instance from a config file         | `LoadInstance`     |
| `POST`   | `/api/instances/{id}/start`  | Start a loaded (Ready) instance                | `StartInstance`    |
| `POST`   | `/api/instances/{id}/stop`   | Stop a running instance                        | `StopInstance`     |
| `DELETE` | `/api/instances/{id}`        | Unload a stopped (Ready) instance              | `UnloadInstance`   |
| `POST`   | `/api/instances/{id}/reload` | Reload an instance (stop + unload + load)      | `ReloadInstance`   |

### 6.3 Request/Response Formats

#### GET `/api/instances`

**Response:** `200 OK`

```json
[
  {
    "instance_id": "main",
    "instance_type": "gtk",
    "has_window": true,
    "lifecycle": "running"
  },
  {
    "instance_id": "tools-overlay",
    "instance_type": "gtk",
    "has_window": false,
    "lifecycle": "ready"
  }
]
```

#### POST `/api/instances`

**Request body:**

```json
{
  "instance_id": "tools-overlay",
  "config_path": "/home/user/.config/smearor/launcher/config-tools.toml",
  "instance_type": "gtk",
  "persist": false
}
```

**Response:** `200 OK`

```json
{
  "ok": true,
  "message": "Instance 'tools-overlay' loaded"
}
```

**Error response:** `400 Bad Request`

```json
{
  "ok": false,
  "message": "Instance 'tools-overlay' already exists"
}
```

#### POST `/api/instances/{id}/start`

**Response:** `200 OK`

```json
{
  "ok": true,
  "message": "Instance 'tools-overlay' started"
}
```

**Error response:** `409 Conflict` (invalid lifecycle state)

```json
{
  "ok": false,
  "message": "Invalid lifecycle transition: Running → Starting"
}
```

#### POST `/api/instances/{id}/stop`

**Response:** `200 OK`

```json
{
  "ok": true,
  "message": "Instance 'tools-overlay' stopped"
}
```

#### DELETE `/api/instances/{id}`

**Response:** `200 OK`

```json
{
  "ok": true,
  "message": "Instance 'tools-overlay' unloaded"
}
```

**Error response:** `404 Not Found`

```json
{
  "ok": false,
  "message": "Instance 'tools-overlay' not found"
}
```

### 6.4 Handler Implementation

Each REST handler is an async function in `smearor-swipe-launcher/src/web/routes.rs` that:

1. Constructs the appropriate `McpCommand` variant with a `oneshot::channel()`
2. Sends the command via `state.host_command_sender.send(cmd).await`
3. Awaits the `oneshot::Receiver<Result<String, String>>` response
4. Maps the result to an HTTP status code (`200` for success, `400`/`404`/`409` for errors)
5. Returns a JSON response body

Example handler for `POST /api/instances/{id}/start`:

```rust
pub async fn handle_start_instance(
    Path(instance_id): Path<String>,
    State(state): State<Arc<WebAppState>>,
) -> impl IntoResponse {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let cmd = McpCommand::StartInstance {
        instance_id,
        response: tx,
    };

    if state.host_command_sender.send(cmd).await.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(InstanceActionResponse {
                ok: false,
                message: "Host command channel closed".to_string(),
            }),
        )
            .into_response();
    }

    match rx.await {
        Ok(Ok(msg)) => (
            StatusCode::OK,
            Json(InstanceActionResponse { ok: true, message: msg }),
        )
            .into_response(),
        Ok(Err(err)) => (
            StatusCode::CONFLICT,
            Json(InstanceActionResponse { ok: false, message: err }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(InstanceActionResponse {
                ok: false,
                message: "Host did not respond".to_string(),
            }),
        )
            .into_response(),
    }
}
```

### 6.5 Router Registration

The `build_router()` method in `smearor-swipe-launcher/src/web/server.rs` is extended:

```rust
fn build_router(&self) -> Router<Arc<WebAppState>> {
    Router::new()
        // Existing web instance routes
        .route("/instances", get(list_web_instances))
        .route("/instances/{id}", get(serve_instance_page))
        .route("/instances/{id}/ws", get(handle_websocket))
        .route("/instances/{id}/{plugin_id}/{action}", post(handle_action))
        // REST API for instance lifecycle control
        .route("/api/instances", get(handle_list_instances).post(handle_load_instance))
        .route("/api/instances/{id}/start", post(handle_start_instance))
        .route("/api/instances/{id}/stop", post(handle_stop_instance))
        .route("/api/instances/{id}/reload", post(handle_reload_instance))
        .route("/api/instances/{id}", delete(handle_unload_instance))
        // Static assets
        .route("/static/style.css", get(serve_static_css))
        .route("/static/app.js", get(serve_static_js))
        .route("/static/nerdfont.css", get(serve_static_nerdfont_css))
        .route("/static/nerdfont.woff2", get(serve_static_nerdfont_woff2))
}
```

### 6.6 Authentication

The REST API endpoints are protected by the same `auth_middleware` already applied to all web server routes. If `auth_token` is configured in `WebServerConfig`,
requests must include `Authorization: Bearer <token>`. If no `auth_token` is configured, the endpoints are open (consistent with the existing web instance
routes).

### 6.7 Web Server Initialization

In `main.rs`, the `WebServer::new()` call is updated to pass the `mcp_command_sender` as a separate parameter:

```rust
if services_config.web.enabled {
let web_config = crate::web::WebServerConfig {
port: services_config.web.port,
enabled: true,
bind_address: services_config.web.bind_address.clone(),
auth_token: services_config.web.auth_token.clone(),
allowed_origins: services_config.web.allowed_origins.clone(),
};
host.start_web_server(web_config, mcp_command_sender.clone());
}
```

The `mcp_command_sender` is passed as a separate parameter to `WebServer::new()` and stored in `WebAppState`. It is **not** part of `WebServerConfig`, since
config structs should only contain serializable configuration values, not runtime channel handles.

---

## 7. Config Integration

### 7.1 Launcher Instance Config (`~/.config/smearor/launcher/config.toml`)

```toml
[launcher]
auto_start = true          # default: instance starts automatically
instance_type = "gtk"     # default: gtk

# Set to false to load the instance without starting it.
# It can be started later via launcher_start_instance MCP tool
# or core.instance.start broker message.
# auto_start = false

# Optional: auto-stop the instance after N seconds.
# When the TTL expires, the instance is stopped (returns to Ready).
# It is not unloaded and can be started again.
# Useful for transient overlays, temporary menus, or secondary launchers.
# auto_stop_ttl = 30

# Optional: event-driven auto-start via message broker topic.
# When a message is sent to this topic, the instance starts (if in Ready state).
# Useful for OSDs that react to service status messages.
# auto_start_topic = "audio.status.volume"

# Optional: event-driven auto-stop via message broker topic.
# When a message is sent to this topic, the instance stops (if in Running state).
# Useful for immediate dismissal without waiting for the TTL.
# auto_stop_topic = "display.off"
```

### 7.2 Deferred-Start Instance Config (`~/.config/smearor/launcher/config-side3.toml`)

```toml
[launcher]
auto_start = false
instance_type = "gtk"

[[areas]]
# ... area definitions ...
```

This config is discovered at startup, loaded into `Ready` state, and **persisted** (because it was discovered as a config file). It is not started
automatically. It can be started at runtime via:

- MCP tool: `launcher_start_instance` with `instance_id = "config-side3"`
- Broker message: `core.instance.start` with `InstanceStartMessage { instance_id: "config-side3" }`

After a restart, the instance is restored from `instances.toml` in `Ready` state.

### 7.3 Transient Instance (Runtime, Not Persisted)

A transient instance is loaded at runtime via MCP tool or broker message and is **not** written to `instances.toml`. On the next restart, it does not reappear.

**MCP tool call:**

```json
{
  "tool": "launcher_load_instance",
  "arguments": {
    "instance_id": "tools-overlay",
    "config_path": "~/.config/smearor/launcher/config-tools.toml",
    "instance_type": "gtk",
    "persist": false
  }
}
```

**Broker message:** `core.instance.load` with `InstanceLoadMessage { instance_id: "tools-overlay", persist: false, ... }`

The instance is loaded into `Ready` state. Call `launcher_start_instance` to open its window. When no longer needed, call `launcher_stop_instance` then
`launcher_unload_instance` to completely remove it. On the next process restart, it is gone.

### 7.4 Instances State File (`~/.config/smearor/instances.toml`)

Only persistent instances appear in the state file. Transient instances are never written:

```toml
# Persisted dynamic launcher instances.
# Automatically managed by the launcher — do not edit manually.

[[instances]]
instance_id = "side3"
config_path = "/home/user/.config/smearor/launcher/config-side3.toml"
instance_type = "gtk"
lifecycle = "ready"

[[instances]]
instance_id = "macropad_5"
config_path = "/home/user/.config/smearor/launcher/config-macropad.toml"
instance_type = "headless"
lifecycle = "running"
```

A transient instance like `tools-overlay` does **not** appear here. It exists only in memory during the running process.

---

## 8. Implementation Phases

### Phase 1: Model Crate — Lifecycle Enum and Messages

**Dependencies:** None

**Tasks:**

- Replace `InstanceLifecycleEvent` with `LauncherInstanceLifecycle` in `lifecycle_event.rs`
- Add `TOPIC_CORE_INSTANCE_START` and `TOPIC_CORE_INSTANCE_UNLOAD` to `topics.rs`
- Create `start_message.rs` with `InstanceStartMessage`
- Create `unload_message.rs` with `InstanceUnloadMessage`
- Update `InstanceStatusMessage` to use `LauncherInstanceLifecycle` as `event` field
- Add `persist: bool` field to `InstanceLoadMessage` with `#[serde(default)]`
- Add JSON converters for `InstanceStartMessage` and `InstanceUnloadMessage`
- Update `InstanceStatusMessageConverter` for new lifecycle enum
- Update `register_json_converters` to register new converters
- Update `lib.rs` re-exports

**Exit Criteria:** `cargo build -p smearor-model-instance-control` succeeds

### Phase 2: Launcher Core — Lifecycle Field and Transition Logic

**Dependencies:** Phase 1

**Tasks:**

- Add `validate_transition_to` and `validate_transition_from` methods to `LauncherInstanceLifecycle` in `model/instance-control/src/lifecycle_event.rs`
- Add `LauncherInstanceLifecycleTransitionError` error type with `thiserror` in `model/instance-control/src/lifecycle_event.rs`
- Add `lifecycle: Mutex<LauncherInstanceLifecycle>` field to `LauncherInstance`
- Initialize lifecycle to `Ready` in `LauncherInstance::new`
- Create `smearor-swipe-launcher/src/instance/lifecycle.rs` with `LifecycleGuard` RAII struct
- Export `lifecycle` module from `instance/mod.rs`
- Add `lifecycle: String` field to `PersistedInstance` with `#[serde(default = "default_running")]`
- Update `persist_instance` to write the lifecycle state
- Add `auto_start: bool` field to `SwipeLauncherSettings` with `#[serde(default = "default_true")]`
- Add `auto_stop_ttl: Option<u64>` field to `SwipeLauncherSettings` with `#[serde(default)]`
- Add `auto_start_topic: Option<String>` field to `SwipeLauncherSettings` with `#[serde(default)]`
- Add `auto_stop_topic: Option<String>` field to `SwipeLauncherSettings` with `#[serde(default)]`
- Add `auto_stop_handle: Mutex<Option<tokio::task::JoinHandle<()>>>` field to `LauncherInstance`

**Exit Criteria:** `cargo build -p smearor-swipe-launcher` succeeds (may have warnings about unused fields)

### Phase 3: Launcher Core — Host Methods

**Dependencies:** Phase 2

**Tasks:**

- Refactor `load_instance`: add `persist: bool` parameter; remove `build_window` / `build_headless` calls; add lifecycle transitions (`Loading → Ready`); add
  `auto_start` check (call `start_instance` if `auto_start = true`); only call `persist_instance` if `persist = true`
- Implement `start_instance`: validate `Ready → Starting`, acquire `LifecycleGuard` with rollback `Ready`, build window/headless, set `Running`, mark guard
  complete, broadcast, persist; if `auto_stop_ttl` is set, spawn TTL timer task and store `JoinHandle`
- Refactor `stop_instance`: validate `Running → Stopping`, cancel auto-stop TTL timer, acquire `LifecycleGuard` with rollback `Running`, close window/remove
  areas, set `Ready`, mark guard complete, broadcast, persist; remove plugin unloading, config watcher removal, instance removal, unpersist
- Implement `unload_instance`: validate `Ready → Unloading`, cancel auto-stop TTL timer, acquire `LifecycleGuard` with rollback `Ready`, unload plugins, remove
  watchers, remove from map, unpersist, mark guard complete
- Refactor `reload_instance`: save `previous_state` before stop/unload; call `load_instance` with suppressed auto_start; restore `Running` if previous state was
  `Running`, otherwise leave in `Ready`
- Update `list_instances`: add `lifecycle` field to JSON output
- Update `load_persisted_instances`: read `lifecycle` from state, call `load_instance` with `persist = true`, start if was running and auto_start allows
- Add broker routing for `TOPIC_CORE_INSTANCE_START` and `TOPIC_CORE_INSTANCE_UNLOAD` in `route_message`
- Add broker subscription for `auto_start_topic` and `auto_stop_topic` in `load_instance` (subscribe) and `unload_instance` (unsubscribe)
- Add broker handler for `auto_start_topic` messages: call `start_instance` (silently ignore if already `Running`)
- Add broker handler for `auto_stop_topic` messages: call `stop_instance` (silently ignore if already `Ready`)

**Exit Criteria:** `cargo build -p smearor-swipe-launcher` succeeds. All lifecycle transitions work correctly.

### Phase 4: MCP Server — Tools and Commands

**Dependencies:** Phase 3

**Tasks:**

- Add `McpCommand::StartInstance` and `McpCommand::UnloadInstance` variants to `mcp-server/src/lib.rs`
- Add `persist` field to `McpCommand::LoadInstance` (or pass through via parameters)
- Add `launcher_start_instance` tool definition to `mcp-server/src/tools.rs`
- Add `launcher_unload_instance` tool definition to `mcp-server/src/tools.rs`
- Update descriptions of `launcher_load_instance`, `launcher_stop_instance`, `launcher_list_instances`
- Add `StartInstance` and `UnloadInstance` match arms to `send_command_and_wait`
- Add `StartInstance` and `UnloadInstance` match arms to `process_mcp_command` in `main.rs`
- Update `LoadInstance` match arm to pass `persist` parameter through to `load_instance`

**Exit Criteria:** `cargo build` succeeds for entire workspace. MCP tools are discoverable via `tools/list`.

### Phase 5: REST API (Web Server)

**Dependencies:** Phase 4

**Tasks:**

- Add `mcp_command_sender: async_channel::Sender<McpCommand>` field to `WebAppState`
- Pass `mcp_command_sender` as separate parameter to `WebServer::new()` from `main.rs`
- Implement `handle_list_instances` handler (`GET /api/instances`)
- Implement `handle_load_instance` handler (`POST /api/instances`)
- Implement `handle_start_instance` handler (`POST /api/instances/{id}/start`)
- Implement `handle_stop_instance` handler (`POST /api/instances/{id}/stop`)
- Implement `handle_unload_instance` handler (`DELETE /api/instances/{id}`)
- Implement `handle_reload_instance` handler (`POST /api/instances/{id}/reload`)
- Register all `/api/*` routes in `build_router()`
- Define `InstanceActionResponse` and `InstanceLoadRequest` JSON structs

**Exit Criteria:** `cargo build` succeeds. REST endpoints return correct responses for valid and invalid lifecycle transitions.

### Phase 6: Startup Loop Integration

**Dependencies:** Phase 5

**Tasks:**

- Replace the startup loop in `main.rs` (lines 96-120) with `host.load_instance` calls
- `load_instance` handles `auto_start` internally, so the loop only needs to call `load_instance` per config path
- Remove the separate `build_headless` / `css_watcher.watch_instance_css` calls from the startup loop (now handled by `load_instance` and `start_instance`)
- Pass `mcp_command_sender.clone()` to `WebServer::new()` for REST API support

**Exit Criteria:** Launcher starts correctly with multiple configs. `auto_start = false` configs are loaded but not started. REST API is accessible when web
server is enabled.

### Phase 7: Integration Tests

**Dependencies:** Phase 5

**Tasks:**

- Test load → start → stop → unload cycle via MCP tools (transient and persistent)
- Test auto_start = true (instance starts automatically after load)
- Test auto_start = false (instance stays in Ready after load)
- Test invalid transitions (start on Running instance → error, unload on Running instance → error)
- Test reload (stop + unload + load + start if auto_start)
- Test list_instances shows correct lifecycle states
- Test transient instance (persist = false) is not written to instances.toml
- Test persistent instance (persist = true) is written to instances.toml
- Test transient instance does not reappear after restart
- Test persisted instances restore correct lifecycle after restart
- Test broker `core.instance.load` with `persist = false` creates a transient instance
- Test broker `core.instance.load` with `persist = true` creates a persistent instance
- Test broker messages for start and unload
- Test REST API: `GET /api/instances` returns correct list with lifecycle states
- Test REST API: `POST /api/instances` loads a transient instance
- Test REST API: `POST /api/instances/{id}/start` starts a Ready instance
- Test REST API: `POST /api/instances/{id}/stop` stops a Running instance
- Test REST API: `DELETE /api/instances/{id}` unloads a Ready instance
- Test REST API: invalid transitions return appropriate HTTP status codes (409)
- Test REST API: missing instance returns 404
- Test REST API: auth token required when configured

**Exit Criteria:** All tests pass. No panics or invalid state transitions.

---

## 9. Dependencies

| Crate                    | New Dependencies                                          |
|--------------------------|-----------------------------------------------------------|
| `model/instance-control` | None (already depends on `serde`, `stabby`, `plugin-api`) |
| `smearor-swipe-launcher` | None (already depends on `model/instance-control`)        |
| `mcp-server`             | None (already depends on `tokio`, `serde_json`)           |

No new crate dependencies are introduced.

---

## 10. Testing Checklist

- [ ] Load instance with `auto_start = true` → instance reaches `Running`
- [ ] Load instance with `auto_start = false` → instance stays in `Ready`
- [ ] Start a `Ready` instance via MCP tool → instance reaches `Running`
- [ ] Start a `Ready` instance via broker message → instance reaches `Running`
- [ ] Stop a `Running` instance → instance returns to `Ready`, plugins still loaded
- [ ] Unload a `Ready` instance → instance removed from map, plugins unloaded
- [ ] Unload a `Running` instance → error returned (must stop first)
- [ ] Start a `Running` instance → error returned (already running)
- [ ] Load duplicate instance ID → error returned
- [ ] Reload a running instance → stop + unload + load + start (restores Running)
- [ ] Reload a ready instance → unload + load (stays in Ready)
- [ ] Reload preserves previous lifecycle state, ignores `auto_start` in changed config
- [ ] Config watcher triggers reload on `Ready` instance → instance stays in `Ready`
- [ ] `list_instances` shows correct `lifecycle` field for each instance
- [ ] Persisted instances restore to correct lifecycle after process restart
- [ ] Persisted `lifecycle = "running"` instance auto-starts after restart (if `auto_start = true`)
- [ ] Persisted `lifecycle = "ready"` instance stays in `Ready` after restart
- [ ] Broker `core.instance.start` message triggers `start_instance`
- [ ] Broker `core.instance.unload` message triggers `unload_instance`
- [ ] `InstanceStatusMessage` broadcasts correct lifecycle state on each transition
- [ ] REST `GET /api/instances` returns correct list with lifecycle states
- [ ] REST `POST /api/instances` loads a transient or persistent instance
- [ ] REST `POST /api/instances/{id}/start` starts a Ready instance
- [ ] REST `POST /api/instances/{id}/stop` stops a Running instance
- [ ] REST `DELETE /api/instances/{id}` unloads a Ready instance
- [ ] REST invalid transitions return HTTP 409
- [ ] REST missing instance returns HTTP 404
- [ ] REST auth token enforced when configured
- [ ] No `unwrap()` or `expect()` in new production code paths
- [ ] `start_instance` failure (build_window error) rolls back to `Ready`
- [ ] `stop_instance` failure rolls back to `Running`
- [ ] `unload_instance` failure rolls back to `Ready`, instance remains in map
- [ ] `start_instance` panic during build_window does not leave instance stuck in `Starting`
- [ ] `LifecycleGuard` rolls back on drop without `complete()`
- [ ] Config without `auto_start` field defaults to `true` (backward compatible)
- [ ] Config without `auto_stop_ttl` field defaults to `None` (no auto-stop, backward compatible)
- [ ] Instance with `auto_stop_ttl = 5` auto-stops after 5 seconds → returns to `Ready`
- [ ] Instance with `auto_stop_ttl` set can be re-started after auto-stop
- [ ] Manual `stop_instance` before TTL expires cancels the timer (no double-stop)
- [ ] Re-`start_instance` after TTL stop aborts stale timer handle (no premature close from old task)
- [ ] Rapid stop → start → stop cycle does not leave orphaned timer tasks
- [ ] `unload_instance` cancels the TTL timer
- [ ] Instance without `auto_stop_ttl` never auto-stops
- [ ] `auto_start_topic` message starts a `Ready` instance → `Running`
- [ ] `auto_start_topic` message on already-`Running` instance is silently ignored (no error)
- [ ] `auto_stop_topic` message stops a `Running` instance → `Ready`
- [ ] `auto_stop_topic` message on already-`Ready` instance is silently ignored (no error)
- [ ] `auto_start_topic` + `auto_stop_ttl` combination: message opens OSD, TTL auto-closes it
- [ ] Broker subscription for `auto_start_topic` / `auto_stop_topic` is created on `load_instance` and removed on `unload_instance`
- [ ] Config without `auto_start_topic` / `auto_stop_topic` defaults to `None` (no subscription, backward compatible)

---

## 11. Future Enhancements

- **Auto-restart policy**: A config field `auto_restart = true` that automatically restarts an instance if it crashes or its window is closed unexpectedly
  (currently, closing a GTK window quits the entire application).
- **Instance groups**: Grouping instances so that starting/stopping a group starts/stops all instances in the group.
- **Health checks**: Periodic health checks for running instances, automatic transition to `Stopping` if an instance becomes unresponsive.
- **Web UI control panel**: A web-based dashboard that consumes the REST API (`/api/instances`) and provides a visual interface showing all instances and their
  lifecycle states, with buttons to start/stop/unload them.
- **D-Bus interface**: Expose instance lifecycle control via D-Bus so external applications (e.g. a desktop environment panel) can control launcher instances.

---

## 12. Documentation

### 12.1 Book (`book/`)

The project's mdBook documentation must be updated to reflect the new instance lifecycle management:

- **New chapter**: `book/src/lifecycle.md` — documents the `LauncherInstanceLifecycle` state machine, state transitions, `LifecycleGuard` RAII pattern, and the
  `auto_start` / `auto_stop_ttl` / `auto_start_topic` / `auto_stop_topic` configuration options.
- **Update `book/src/SUMMARY.md`**: Add link to the new lifecycle chapter.
- **Update existing chapters**: Any chapter referencing instance creation or window building should reference the new `load_instance` / `start_instance` /
  `stop_instance` / `unload_instance` methods and the lifecycle states.
- **Diagrams**: Include a state transition diagram (can be ASCII art or a Mermaid block) showing the six states and their valid transitions.

### 12.2 README

The project's `README.md` must be updated:

- **Quick Start**: Mention `auto_start = false` as an option for instances that should be started on-demand.
- **Configuration**: Document the new `[launcher]` fields (`auto_start`, `auto_stop_ttl`, `auto_start_topic`, `auto_stop_topic`) with brief descriptions and
  default values.
- **REST API**: Add a section listing the `/api/instances` endpoints with example `curl` commands.
- **MCP Tools**: List the five lifecycle MCP tools (`launcher_load_instance`, `launcher_start_instance`, `launcher_stop_instance`, `launcher_unload_instance`,
  `launcher_reload_instance`) with brief descriptions.
