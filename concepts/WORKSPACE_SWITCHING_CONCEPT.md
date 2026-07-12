# Workspace Switching Concept

## 1. Goal

Provide compositor-independent workspace change detection and broadcasting for the launcher. When the
user switches workspaces, the launcher re-evaluates layout profiles and rebuilds its areas
dynamically — regardless of which Wayland compositor is running.

This concept describes three service implementations that all broadcast the same generic
`WorkspaceChangedEvent` (defined in `model/workspace`):

1. **`services/hyprland`** — Hyprland-specific, via `hyprland` crate IPC (already implemented, see
   `HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md`)
2. **`services/wayland`** — wlroots-based compositors, via `ext-workspace-unstable-v1` Wayland protocol
3. **`services/gnome`** — GNOME/Mutter, via D-Bus

All three services broadcast over the same topic `"compositor::workspace_changed"`. The launcher
core has no knowledge of which compositor is running — it only consumes the generic event.

## 2. Scope

- **In scope:**
    - Architecture and roadmap for `services/wayland` and `services/gnome`
    - How each service detects workspace changes and maps them to `WorkspaceChangedEvent`
    - Monitor-index resolution for each compositor
    - Configuration model
- **Out of scope:**
    - Hyprland-specific implementation details (already implemented, see
      `HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md`)
    - Layout profile matching logic (already implemented in `LAYOUT_PROFILE_CONCEPT.md`)
    - Widget-level workspace state (future concept)

## 3. Current State

### 3.1 Generic Model (`model/workspace`)

`WorkspaceChangedEvent` is already defined in `model/workspace/src/messages.rs`:

```rust
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceChangedEvent {
    pub workspace_name: stabby::string::String,
    pub workspace_id: i32,
    pub monitor_index: u32,
}
```

The topic `TOPIC_WORKSPACE_CHANGED = "compositor::workspace_changed"` is compositor-independent.

### 3.2 Hyprland Service (`services/hyprland`)

Already implemented. The `HyprlandService` spawns an event listener thread that connects to
Hyprland's IPC socket, listens for `WorkspaceChanged` events, resolves the monitor index via
`hyprctl monitors`, and broadcasts `WorkspaceChangedEvent` to all launcher instances.

### 3.3 Launcher Integration

`LauncherHost` routes `WorkspaceChangedEvent` messages to all `LauncherInstance`s. Each instance
calls `on_workspace_changed(workspace_id, monitor_index)` which invokes
`get_layout_for_context(None, Some(monitor_index), Some(workspace_id))` and rebuilds areas if the
resolved layout differs from the current one.

### 3.4 Layout Trigger Infrastructure

`LayoutTrigger` in `smearor-swipe-launcher/src/config/layout/trigger.rs` supports:

| Trigger                 | Fields                          | Description                          |
|-------------------------|---------------------------------|--------------------------------------|
| `Workspace(i32)`        | workspace                       | Match by workspace ID on any monitor |
| `MonitorIndex(u32)`     | monitor                         | Match by GDK monitor index           |
| `MonitorIndexWorkspace` | monitor: u32, workspace: i32    | Match by monitor index + workspace   |
| `MonitorWorkspace`      | monitor: String, workspace: i32 | Match by connector name + workspace  |

## 4. Architecture

### 4.1 Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Compositor Services                               │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
│  │  HyprlandService │  │  WaylandService  │  │  GnomeService    │       │
│  │  (IPC socket)    │  │  (ext-workspace) │  │  (D-Bus)         │       │
│  │                  │  │                  │  │                  │       │
│  │  workspace       │  │  workspace      │  │  workspace       │       │
│  │  change event ───┼──┤  change event ───┼──┤  change event ───┼──┐   │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘  │   │
│                                                                     │   │
│  All three broadcast the same WorkspaceChangedEvent via FfiCoreContext │   │
└─────────────────────────────────────────────────────────────────────┼──┘
                                                                      │
                                                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Launcher Core                                     │
│                                                                         │
│  LauncherHost receives WorkspaceChangedEvent                             │
│  → routes to all LauncherInstances                                       │
│  → each instance calls on_workspace_changed(workspace_id, monitor_index) │
│  → get_layout_for_context() → rebuild_areas()                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Event Flow (all compositors)

1. Compositor emits a workspace change event (IPC, Wayland protocol, or D-Bus signal).
2. The service-specific event listener receives and parses it.
3. The service resolves the monitor index (compositor-specific query).
4. The service constructs a `WorkspaceChangedEvent` with `workspace_id` and `monitor_index`.
5. The service broadcasts the event via `FfiCoreContext` to all launcher instances.
6. Each launcher instance re-evaluates its layout profile and rebuilds areas if needed.

### 4.3 Service Selection

Only one workspace tracking service should be active at a time. The user enables the appropriate
service in `services.toml` based on their compositor:

| Compositor                  | Service             | Config Key                         |
|-----------------------------|---------------------|------------------------------------|
| Hyprland                    | `services/hyprland` | `enable_workspace_tracking = true` |
| sway, river, Labwc, Wayfire | `services/wayland`  | `enable_workspace_tracking = true` |
| GNOME/Mutter                | `services/gnome`    | `enable_workspace_tracking = true` |

If multiple services are active, multiple `WorkspaceChangedEvent`s may fire for the same workspace
change. The launcher handles this gracefully (idempotent rebuild), but it is wasteful.

## 5. Service: `services/wayland` (ext-workspace-unstable-v1)

### 5.1 Background

The `ext-workspace-unstable-v1` protocol (formerly `wlr-workspace-unstable-v1`) is a Wayland
protocol extension developed by the wlroots community. It provides workspace group and workspace
handle objects that report state changes via events.

**Supported compositors:** sway, river, Labwc, Wayfire, Hyprland (but Hyprland has its own
dedicated service via IPC — use whichever is preferred).

**Not supported:** GNOME/Mutter (uses D-Bus instead, see Section 6).

### 5.2 Protocol Overview

The protocol defines:

- `workspace_manager` — global singleton, lists workspace groups
- `workspace_group` — represents a group of workspaces (typically per monitor)
- `workspace` — individual workspace with `id`, `name`, `coordinates`, and `state` (active, urgent, hidden)

Key events:

| Event                               | Description                                      |
|-------------------------------------|--------------------------------------------------|
| `workspace_manager.done`            | All changes have been sent, apply them           |
| `workspace_group.workspace_added`   | A new workspace was created                      |
| `workspace_group.workspace_removed` | A workspace was destroyed                        |
| `workspace.state`                   | Workspace state changed (active, hidden, urgent) |
| `workspace.name`                    | Workspace name changed                           |

### 5.3 Crate Structure

```
services/wayland/
├── Cargo.toml
├── src/
│   ├── lib.rs          — service_plugin!(WaylandWorkspaceService);
│   ├── config.rs       — WaylandWorkspaceServiceConfig
│   └── service.rs      — WaylandWorkspaceService struct + event listener
```

### 5.4 Configuration

```rust
/// Configuration for the Wayland workspace tracking service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct WaylandWorkspaceServiceConfig {
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
}
```

In `services.toml`:

```toml
[[services]]
id = "wayland"
path = "target/release/libsmearor_wayland_service.so"

[wayland]
enable_workspace_tracking = true
```

### 5.5 Implementation

The service uses the `wayland-client` crate to connect to the Wayland display and bind the
`ext_workspace_manager_v1` global. A dedicated event listener thread runs the Wayland event loop.

```rust
pub struct WaylandWorkspaceService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: Arc<WaylandWorkspaceServiceConfig>,
}
```

#### Event Listener Thread

```rust
// Spawn event listener thread
std::thread::spawn(move | | {
// Connect to Wayland display
let display = match wayland_client::Display::connect_to_env() {
Ok(display) => display,
Err(error) => {
error ! ("Wayland workspace service: failed to connect to display: {error}");
return;
}
};

let mut event_queue = display.create_event_queue();
let attached_display = ( * display).clone().attach(event_queue.token());

// Bind ext_workspace_manager_v1 global
// Implementation requires a generated protocol module from
// ext-workspace-unstable-v1.xml

// Run event loop
loop {
if let Err(error) = event_queue.dispatch_blocking() {
error ! ("Wayland event queue error: {error}, reconnecting in 5s");
std::thread::sleep(Duration::from_secs(5));
// Reconnect logic
}
}
});
```

#### Workspace State Handling

When a `workspace.state` event fires with the `active` state:

1. Extract the workspace `id` and `name` from the workspace handle.
2. Determine which monitor (output) the workspace group belongs to.
3. Map the Wayland output to a GDK monitor index (see Section 5.6).
4. Construct a `WorkspaceChangedEvent` and broadcast it.

#### Reconnection Strategy

If the Wayland connection is lost (compositor restart), the service reconnects:

```rust
loop {
let display = match wayland_client::Display::connect_to_env() {
Ok(display) => display,
Err(error) => {
error ! ("Wayland workspace service: failed to connect: {error}, retrying in 5s");
std::thread::sleep(Duration::from_secs(5));
continue;
}
};

// ... bind globals, add handlers ...

// Run event loop until error
loop {
if let Err(error) = event_queue.dispatch_blocking() {
error ! ("Wayland event loop stopped: {error}");
break;
}
}
}
```

### 5.6 Monitor Index Resolution

The `ext-workspace-unstable-v1` protocol associates workspaces with workspace groups, not directly
with outputs. However, compositors typically create one workspace group per output. To map a
workspace group to a GDK monitor index:

1. **Option A — Output name matching:** The Wayland `xdg_output` protocol provides the output's
   `name` and `description`. Match this against GDK's `Monitor::connector()` value.
2. **Option B — Output order:** Use the order of `wl_output` globals as they appear in the registry.
   This matches GDK's monitor index in most cases, but is not guaranteed.
3. **Option C — Fallback:** If no monitor can be resolved, set `monitor_index = 0`.

**Recommended:** Option A (output name matching) for accuracy, with Option C as fallback.

### 5.7 Dependencies

| Crate                               | Purpose                                                     |
|-------------------------------------|-------------------------------------------------------------|
| `wayland-client`                    | Wayland client library                                      |
| `wayland-protocols`                 | Protocol definitions (includes `ext-workspace-unstable-v1`) |
| `tokio`                             | Async runtime for the service worker                        |
| `tracing`                           | Logging                                                     |
| `stabby`                            | ABI-stable types                                            |
| `smearor-model-compositor`          | `WorkspaceChangedEvent` and topic                           |
| `smearor-swipe-launcher-plugin-api` | Service trait, FFI types                                    |

## 6. Service: `services/gnome` (D-Bus)

### 6.1 Background

GNOME/Mutter does not implement the `ext-workspace-unstable-v1` Wayland protocol. Instead,
workspace state is exposed via D-Bus on the session bus:

- `org.gnome.Shell` — the GNOME Shell D-Bus interface
- `org.gnome.Mutter.DisplayConfig` — monitor information
- `org.gnome.Shell.Introspect` — introspection API (available in newer GNOME versions)

GNOME does not expose a direct workspace-changed D-Bus signal. The approach is to poll the
`org.gnome.Shell.Eval` interface, which executes JavaScript in the GNOME Shell process and returns
the current workspace index. Monitor information is resolved via
`org.gnome.Mutter.DisplayConfig`.

### 6.2 D-Bus Interfaces

| Interface                        | Method / Signal                            | Description                                   |
|----------------------------------|--------------------------------------------|-----------------------------------------------|
| `org.gnome.Shell.Eval`           | `Eval(code: String) -> (bool, String)`     | Execute JS in GNOME Shell, return result      |
| `org.gnome.Shell.Introspect`     | `GetWindows() -> HashMap<u32, WindowInfo>` | Window list with workspace IDs (GNOME 46+)    |
| `org.gnome.Mutter.DisplayConfig` | `GetResources() / GetMonitors()`           | Monitor list with connector names and indices |
| `org.gnome.Mutter.DisplayConfig` | `MonitorsChanged` (signal)                 | Monitor configuration changed                 |

**Approach:** Poll `org.gnome.Shell.Eval` every 500ms to query the active workspace. When the
workspace changes, query `org.gnome.Mutter.DisplayConfig` for monitor index resolution and
broadcast a `WorkspaceChangedEvent`.

### 6.3 Crate Structure

```
services/gnome/
├── Cargo.toml
├── src/
│   ├── lib.rs          — service_plugin!(GnomeWorkspaceService);
│   ├── config.rs       — GnomeWorkspaceServiceConfig
│   └── service.rs      — GnomeWorkspaceService struct + D-Bus listener
```

### 6.4 Configuration

```rust
/// Configuration for the GNOME workspace tracking service.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct GnomeWorkspaceServiceConfig {
    /// Enable workspace change event tracking and broadcasting.
    #[serde(default)]
    pub enable_workspace_tracking: bool,
}
```

In `services.toml`:

```toml
[[services]]
id = "gnome"
path = "target/release/libsmearor_gnome_service.so"

[gnome]
enable_workspace_tracking = true
```

### 6.5 Implementation

The service uses `zbus` to connect to the GNOME session D-Bus and listen for workspace change
signals.

```rust
pub struct GnomeWorkspaceService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: Arc<GnomeWorkspaceServiceConfig>,
}
```

#### D-Bus Listener Thread

```rust
// Spawn D-Bus listener thread
std::thread::spawn(move | | {
let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
Ok(rt) => rt,
Err(error) => {
error ! ("GNOME workspace service: failed to create runtime: {error}");
return;
}
};

rt.block_on(async move {
loop {
match listen_for_workspace_changes( & event_sender).await {
Ok(()) => {
debug ! ("GNOME D-Bus listener exited cleanly, reconnecting in 5s");
}
Err(error) => {
error ! ("GNOME D-Bus listener stopped: {error}, reconnecting in 5s");
}
}
tokio::time::sleep(Duration::from_secs(5)).await;
}
});
});
```

#### Workspace Change Detection

1. **Poll `org.gnome.Shell.Eval`** with a JavaScript snippet that returns the current workspace
   index. This is a D-Bus method call, done in the async listener thread.
2. **Listen for `org.gnome.Mutter.DisplayConfig` `MonitorsChanged`** to update the monitor map
   when monitors are connected or disconnected.
3. **Alternatively**, use the `org.gnome.Shell.Introspect` interface (GNOME 46+) to get the active
   workspace via `GetWindows()` — this avoids `Eval` which may be restricted in some GNOME versions.

**Polling approach:**

```rust
async fn listen_for_workspace_changes(
    event_sender: &mpsc::UnboundedSender<GnomeEvent>,
) -> Result<(), zbus::Error> {
    let connection = zbus::Connection::session().await?;

    // Subscribe to the org.gnome.Shell Eval-based polling
    // or the WindowManager::switch-workspace signal if available

    // Poll current workspace every 500ms (or listen for signal)
    let mut last_workspace: i32 = -1;
    loop {
        let current = query_current_workspace(&connection).await?;
        if current != last_workspace {
            last_workspace = current;
            let monitor_index = query_monitor_index(&connection).await.unwrap_or(0);
            let event = WorkspaceChangedEvent {
                workspace_name: current.to_string().into(),
                workspace_id: current,
                monitor_index,
            };
            let _ = event_sender.send(GnomeEvent::WorkspaceChanged(event));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Query the current workspace index via org.gnome.Shell.Eval.
async fn query_current_workspace(connection: &zbus::Connection) -> Result<i32, zbus::Error> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.gnome.Shell",
        "/org/gnome/Shell",
        "org.gnome.Shell",
    ).await?;

    let result: zbus::zvariant::Value = proxy
        .call("Eval", &("global.get_window_actors().find(a => a.meta_window.has_focus())?.get_workspace()"))
        .await?;

    // Parse the result (GNOME Shell returns a tuple (success, value_string))
    // ...
}
```

### 6.6 Monitor Index Resolution

GNOME provides monitor information via `org.gnome.Mutter.DisplayConfig`:

1. Call `org.gnome.Mutter.DisplayConfig.GetResources()` (or `GetMonitors()` in newer versions).
2. The returned monitor list includes connector names and logical indices.
3. Match the connector name against GDK's `Monitor::connector()` to get the GDK monitor index.

If no connector name can be matched, fall back to `monitor_index = 0`.

### 6.7 Dependencies

| Crate                               | Purpose                           |
|-------------------------------------|-----------------------------------|
| `zbus`                              | D-Bus client (async)              |
| `tokio`                             | Async runtime                     |
| `tracing`                           | Logging                           |
| `stabby`                            | ABI-stable types                  |
| `smearor-model-compositor`          | `WorkspaceChangedEvent` and topic |
| `smearor-swipe-launcher-plugin-api` | Service trait, FFI types          |

## 7. Workspace Event Reference

### 7.1 ext-workspace-unstable-v1 (wlroots compositors)

| Event                               | Data             | Use Case                 |
|-------------------------------------|------------------|--------------------------|
| `workspace.state` (active)          | workspace handle | Active workspace changed |
| `workspace_group.workspace_added`   | workspace handle | New workspace created    |
| `workspace_group.workspace_removed` | workspace handle | Workspace destroyed      |
| `workspace.name`                    | string           | Workspace renamed        |

### 7.2 GNOME D-Bus

| Method / Signal                                           | Data                           | Use Case                           |
|-----------------------------------------------------------|--------------------------------|------------------------------------|
| `org.gnome.Shell.Eval` (polling)                          | JS return value                | Query current workspace            |
| `org.gnome.Mutter.DisplayConfig`                          | monitor list                   | Monitor index resolution           |
| `org.gnome.Shell.Introspect.GetWindows`                   | window list with workspace IDs | Infer workspace change (GNOME 46+) |
| `org.gnome.Mutter.DisplayConfig.MonitorsChanged` (signal) | —                              | Monitor hotplug detection          |

## 8. Edge Cases

### 8.1 General

- **Multiple services active:** If both `services/hyprland` and `services/wayland` are active on
  Hyprland, two events fire per workspace change. The launcher rebuilds twice (wasteful but
  harmless). Document that only one should be enabled.
- **Rapid workspace switching:** Each change triggers a layout rebuild. A debounce (e.g. 100ms)
  can be added to the event worker if flicker becomes an issue.
- **No compositor running:** The service fails to connect, logs an error, and retries. The
  launcher continues without workspace tracking.

### 8.2 Wayland (ext-workspace)

- **Protocol not supported:** The compositor does not advertise `ext_workspace_manager_v1`. The
  service logs a warning and exits. No events are broadcast.
- **Workspace groups not per-output:** Some compositors may use a single workspace group for all
  outputs. In this case, monitor index resolution falls back to 0.
- **Special workspaces:** The protocol does not distinguish special workspaces. All workspaces
  have a numeric ID.

### 8.3 GNOME (D-Bus)

- **GNOME Shell not running:** `zbus` connection fails. The service retries.
- **Polling approach:** Polling `org.gnome.Shell.Eval` every 500ms adds minor CPU overhead. The
  `Eval` method may be restricted in some GNOME versions — in that case, fall back to
  `org.gnome.Shell.Introspect.GetWindows()` (GNOME 46+).
- **GNOME on X11:** The D-Bus interface is the same, but monitor index mapping may differ from
  Wayland. GDK handles this transparently.
- **Workspace wrapping:** GNOME wraps around the last workspace. The service should handle
  workspace ID wrap-around gracefully.

## 9. Reconnection Strategy

Both services follow the same reconnection pattern as the Hyprland service:

```rust
loop {
// Connect to compositor (Wayland display / D-Bus session)
match connect().await {
Ok(connection) => {
// Run event loop until error
run_event_loop( & connection, & event_sender).await;
}
Err(error) => {
error ! ("Failed to connect: {error}, retrying in 5s");
}
}
tokio::time::sleep(Duration::from_secs(5)).await;
}
```

## 10. Affected Files

### 10.1 New Files

| File                              | Description                                                          |
|-----------------------------------|----------------------------------------------------------------------|
| `services/wayland/Cargo.toml`     | Crate manifest for Wayland workspace service                         |
| `services/wayland/src/lib.rs`     | `service_plugin!(WaylandWorkspaceService);`                          |
| `services/wayland/src/config.rs`  | `WaylandWorkspaceServiceConfig` struct                               |
| `services/wayland/src/service.rs` | `WaylandWorkspaceService` struct, event listener, monitor resolution |
| `services/gnome/Cargo.toml`       | Crate manifest for GNOME workspace service                           |
| `services/gnome/src/lib.rs`       | `service_plugin!(GnomeWorkspaceService);`                            |
| `services/gnome/src/config.rs`    | `GnomeWorkspaceServiceConfig` struct                                 |
| `services/gnome/src/service.rs`   | `GnomeWorkspaceService` struct, D-Bus listener, monitor resolution   |

### 10.2 Modified Files

| File                                  | Change                                                                         |
|---------------------------------------|--------------------------------------------------------------------------------|
| `Cargo.toml` (workspace)              | Add `services/wayland` and `services/gnome` to `members` and `default-members` |
| `Cargo.toml` (workspace deps)         | Add `wayland-client`, `wayland-protocols`, `zbus` to workspace dependencies    |
| `config_layout_profiles_example.toml` | Add examples for workspace-based profiles (already done)                       |

## 11. Implementation Roadmap

### Phase 1: Wayland Service (`services/wayland`)

| #    | Task                                                                    | Files                                       | Effort |
|------|-------------------------------------------------------------------------|---------------------------------------------|--------|
| 1.1  | Create crate structure and `Cargo.toml`                                 | `services/wayland/`                         | Small  |
| 1.2  | Implement `WaylandWorkspaceServiceConfig`                               | `services/wayland/src/config.rs`            | Small  |
| 1.3  | Implement `WaylandWorkspaceService` struct with `service_plugin!` macro | `services/wayland/src/lib.rs`, `service.rs` | Small  |
| 1.4  | Generate `ext-workspace-unstable-v1` protocol bindings                  | `services/wayland/src/`                     | Medium |
| 1.5  | Implement Wayland event listener thread with reconnection               | `services/wayland/src/service.rs`           | Medium |
| 1.6  | Implement workspace state handler → `WorkspaceChangedEvent`             | `services/wayland/src/service.rs`           | Medium |
| 1.7  | Implement monitor index resolution (output name matching)               | `services/wayland/src/service.rs`           | Medium |
| 1.8  | Add `wayland-client`, `wayland-protocols` to workspace deps             | `Cargo.toml`                                | Small  |
| 1.9  | Add `services/wayland` to workspace members                             | `Cargo.toml`                                | Small  |
| 1.10 | Build and verify                                                        | —                                           | Small  |

### Phase 2: GNOME Service (`services/gnome`)

| #   | Task                                                                    | Files                                     | Effort |
|-----|-------------------------------------------------------------------------|-------------------------------------------|--------|
| 2.1 | Create crate structure and `Cargo.toml`                                 | `services/gnome/`                         | Small  |
| 2.2 | Implement `GnomeWorkspaceServiceConfig`                                 | `services/gnome/src/config.rs`            | Small  |
| 2.3 | Implement `GnomeWorkspaceService` struct with `service_plugin!` macro   | `services/gnome/src/lib.rs`, `service.rs` | Small  |
| 2.4 | Implement D-Bus listener thread with `zbus`                             | `services/gnome/src/service.rs`           | Medium |
| 2.5 | Implement workspace polling via `org.gnome.Shell.Eval`                  | `services/gnome/src/service.rs`           | Medium |
| 2.6 | Implement monitor index resolution via `org.gnome.Mutter.DisplayConfig` | `services/gnome/src/service.rs`           | Medium |
| 2.7 | Add `zbus` to workspace deps                                            | `Cargo.toml`                              | Small  |
| 2.8 | Add `services/gnome` to workspace members                               | `Cargo.toml`                              | Small  |
| 2.9 | Build and verify                                                        | —                                         | Small  |

### Phase 3: Testing & Validation

| #   | Task                                                                | Effort |
|-----|---------------------------------------------------------------------|--------|
| 3.1 | Test Wayland service starts when `enable_workspace_tracking = true` | Small  |
| 3.2 | Test Wayland service detects workspace change on sway               | Medium |
| 3.3 | Test Wayland service detects workspace change on river              | Medium |
| 3.4 | Test GNOME service starts when `enable_workspace_tracking = true`   | Small  |
| 3.5 | Test GNOME service detects workspace change via polling             | Medium |
| 3.6 | Test monitor index resolution for both services                     | Medium |
| 3.7 | Test reconnection after compositor restart                          | Medium |
| 3.8 | Test layout profile switch on workspace change                      | Medium |

## 12. Dependencies

- **`model/workspace`** — All three services depend on `WorkspaceChangedEvent` and
  `TOPIC_WORKSPACE_CHANGED`. No new model crate is needed.
- **`HYPRLAND_WORKSPACE_TRACKING_CONCEPT.md`** — The Hyprland service is already implemented. This
  concept extends the same architecture to other compositors.
- **`LAYOUT_PROFILE_CONCEPT.md`** — Workspace tracking enables `Workspace(i32)`,
  `MonitorIndexWorkspace`, and `MonitorWorkspace` trigger variants at runtime.
- **`LAYER_SHELL_WINDOW_CONCEPT.md`** — The monitor index from the layer shell config is used in
  `on_workspace_changed()` to filter events by monitor.
- **`SERVICE_PLUGIN_CONCEPT.md`** — All services follow the SOA architecture with
  `service_plugin!` macro, `MessageBroadcaster`, and `FfiCoreContext`.

## 13. Comparison Matrix

| Feature               | Hyprland (IPC)                     | Wayland (ext-workspace)               | GNOME (D-Bus)           |
|-----------------------|------------------------------------|---------------------------------------|-------------------------|
| Compositors           | Hyprland                           | sway, river, Labwc, Wayfire           | GNOME/Mutter            |
| Protocol              | UNIX socket IPC                    | Wayland protocol                      | D-Bus session bus       |
| Latency               | Low (event-driven)                 | Low (event-driven)                    | Medium (polling, 500ms) |
| Monitor resolution    | `hyprctl monitors`                 | `xdg_output` name matching            | `Mutter.DisplayConfig`  |
| Special workspaces    | `WorkspaceType::Special` (id = -1) | Not distinguished                     | Not distinguished       |
| Reconnection          | 5s retry loop                      | 5s retry loop                         | 5s retry loop           |
| Extra dependencies    | `hyprland` crate                   | `wayland-client`, `wayland-protocols` | `zbus`                  |
| GNOME Shell extension | Not required                       | Not required                          | Not required            |
