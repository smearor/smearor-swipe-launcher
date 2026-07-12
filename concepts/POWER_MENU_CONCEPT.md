# Concept: Power Menu Service & Widget

This document describes the concept for a **Power Menu Service** and a **Power Menu Widget** in the *Smearor Swipe Launcher*. The service communicates with
`systemd-logind` via **D-Bus** using the [`zbus`](https://crates.io/crates/zbus) crate to perform session and system power actions. The widget provides a
compact GTK4 menu with countdown overlays, inhibitor warnings, and scheduled actions.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/power`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/power`):** Singleton background service that interfaces with `org.freedesktop.login1` via D-Bus, queries capabilities and
   inhibitors, manages scheduled actions, and broadcasts status updates.
3. **Widget Crate (`plugins/power`):** Pure GTK4 UI that displays power action buttons, countdown overlays, inhibitor warnings, and scheduled action status.

---

## 1. Feature Scope

The Power Menu covers all common session and system states a power user needs on a Linux system:

| Action                 | Description                                                                    |
|------------------------|--------------------------------------------------------------------------------|
| **Shutdown**           | Shuts the system down gracefully.                                              |
| **Reboot**             | Restarts the system.                                                           |
| **Suspend**            | Puts the system into RAM sleep (S3).                                           |
| **Hibernate**          | Saves RAM to disk (swap) and powers off (if configured on the system).         |
| **Lock Screen**        | Locks the current session (compatible with Gnome/GDM or Hyprland/hyprlock).    |
| **Log out**            | Terminates the current X11/Wayland session and returns to the display manager. |
| **Reboot to Firmware** | Reboots directly into BIOS/UEFI settings (extremely useful for power users).   |

---

## 2. Recommended Libraries

The clean, architecturally correct way on Linux is to communicate via **D-Bus** with `org.freedesktop.login1` (systemd-logind) instead of shelling out to
`systemctl poweroff` or similar commands.

- **`zbus`:** The standard Rust library for modern, asynchronous D-Bus communication.
    - Direct communication with `org.freedesktop.login1`.
    - Fast, secure, and no sudo password prompts required (logind grants the active user these rights via Polkit).
    - For **Lock** and **Logout**, signals can be sent to `org.freedesktop.login1.Session` via the same D-Bus connection.

---

## 3. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Power Menu Widget        |                 | Power Service              |
| (subscribed to           |                 | (Singleton)                |
|  service.power.status)   |                 |                            |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Command Message                         |
             |  (power action, schedule, cancel)           |
             |===========================================> |
             |  Topic: "service.power.command"             |
             |                                             |
             |                                             |  2. zbus D-Bus call
             |                                             |     org.freedesktop.login1
             |                                             |     .PowerOff / .Reboot / .Suspend
             |                                             |     .Hibernate / .SetRebootToFirmware
             |                                             |     org.freedesktop.login1.Session
             |                                             |     .Lock / .Terminate
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.power.status"
             |                                             |     Payload: PowerStatusMessage { ... }
+--------------------------+                 +----------------------------+
             |                                             |
             |                                             |  4. Inhibitor Query
             |                                             |     org.freedesktop.login1
             |                                             |     .ListInhibitors
             |                                             |
             |                                             |  5. Capability Query
             |                                             |     org.freedesktop.login1
             |                                             |     .CanPowerOff / .CanReboot / ...
+--------------------------+                 +----------------------------+
```

The service also registers **MCP resources** and **MCP tools** so that AI clients can query system capabilities and trigger power actions.

---

## 4. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path              | Responsibility                                                                |
|-------------|-------------------|-------------------------------------------------------------------------------|
| **Model**   | `model/power/`    | Shared structs, enums, topics, and message formats                            |
| **Service** | `services/power/` | D-Bus communication, capability/inhibitor queries, scheduled actions, MCP     |
| **Widget**  | `plugins/power/`  | GTK4 menu UI, countdown overlay, inhibitor warnings, scheduled action display |

---

## 5. Model Crate (`model/power`)

### 5.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.power.command";
pub const TOPIC_STATUS: &str = "service.power.status";
```

### 5.2 Power Action Enum

```rust
/// All power actions supported by the service.
/// Each variant maps to a specific D-Bus call on `org.freedesktop.login1`.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum PowerAction {
    /// Shut the system down gracefully.
    Shutdown,
    /// Restart the system.
    Reboot,
    /// Put the system into RAM sleep (S3).
    Suspend,
    /// Save RAM to disk (swap) and power off.
    Hibernate,
    /// Lock the current session.
    Lock,
    /// Terminate the current session and return to the display manager.
    Logout,
    /// Reboot directly into BIOS/UEFI firmware settings.
    RebootToFirmware,
    /// Cancel a running countdown or scheduled action.
    #[default]
    Cancel,
}
```

### 5.3 Command Message (Widget -> Service)

```rust
/// Actions the power service can perform on request.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum PowerCommandAction {
    /// Execute a power action immediately (with countdown if configured).
    #[default]
    Execute,
    /// Schedule a power action for the future.
    Schedule,
    /// Cancel a running countdown or scheduled action.
    Cancel,
    /// Refresh capabilities and inhibitors from the system.
    Refresh,
}

/// Command message sent by widgets or MCP clients to the power service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct PowerCommandMessage {
    /// The action to execute.
    pub action: PowerCommandAction,
    /// The power action to perform (ignored for `Cancel` and `Refresh`).
    pub power_action: PowerAction,
    /// Delay in minutes for scheduled actions (only used with `Schedule`).
    pub delay_minutes: u32,
}
```

### 5.4 Inhibitor Info

```rust
/// Information about a single inhibitor lock that blocks a power action.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct InhibitorInfo {
    /// Process name that holds the inhibitor lock.
    pub process_name: stabby::string::String,
    /// Reason for the inhibitor lock.
    pub reason: stabby::string::String,
    /// What the inhibitor blocks (e.g., "shutdown", "sleep", "idle").
    pub what: stabby::string::String,
    /// Who registered the inhibitor (e.g., "APT", "Firefox").
    pub who: stabby::string::String,
}
```

### 5.5 System Capabilities

```rust
/// Capabilities of the system as reported by systemd-logind.
/// Each field corresponds to a `Can*` D-Bus property on `org.freedesktop.login1.Manager`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct PowerCapabilities {
    /// Whether the system can be shut down.
    pub can_shutdown: bool,
    /// Whether the system can be rebooted.
    pub can_reboot: bool,
    /// Whether the system can be suspended.
    pub can_suspend: bool,
    /// Whether the system can be hibernated.
    pub can_hibernate: bool,
    /// Whether the system can reboot to firmware/UEFI.
    pub can_reboot_to_firmware: bool,
    /// Whether the session can be locked.
    pub can_lock: bool,
    /// Whether the session can be terminated (log out).
    pub can_logout: bool,
}
```

### 5.6 Scheduled Action Info

```rust
/// Information about a currently scheduled power action.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ScheduledActionInfo {
    /// The power action that is scheduled.
    pub action: PowerAction,
    /// Remaining time in seconds until the action executes.
    pub remaining_seconds: u64,
    /// Total originally scheduled delay in seconds.
    pub total_delay_seconds: u64,
}
```

### 5.7 Status Message (Service -> Widget)

```rust
/// Complete power status message broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct PowerStatusMessage {
    /// System capabilities as reported by systemd-logind.
    pub capabilities: PowerCapabilities,
    /// List of active inhibitor locks.
    pub inhibitors: stabby::vec::Vec<InhibitorInfo>,
    /// Currently scheduled action, if any.
    pub scheduled_action: stabby::option::Option<ScheduledActionInfo>,
    /// Whether a countdown is currently active for an immediate action.
    pub countdown_active: bool,
    /// Remaining seconds in the countdown (0 if no countdown is active).
    pub countdown_remaining_seconds: u32,
    /// The power action currently being counted down.
    pub countdown_action: PowerAction,
    /// Timestamp of the last status refresh as ISO-8601 string.
    pub last_updated: stabby::string::String,
}
```

### 5.8 Nerd Font Icon Mapping

Each power action maps to a Material Design Nerd Font icon for consistent GTK4 rendering.

| Action             | Icon | Unicode     | Nerd Font Name    |
|--------------------|------|-------------|-------------------|
| Shutdown           | 󰐥   | `\u{f0425}` | `nf-md-power`     |
| Reboot             | 󰑐   | `\u{f0450}` | `nf-md-restart`   |
| Suspend            | 󰤓   | `\u{f0913}` | `nf-md-sleep`     |
| Hibernate          | 󰜡   | `\u{f0721}` | `nf-md-snowflake` |
| Lock Screen        | 󰌾   | `\u{f033e}` | `nf-md-lock`      |
| Log out            | 󰍃   | `\u{f0343}` | `nf-md-logout`    |
| Reboot to Firmware | 󰘩   | `\u{f0629}` | `nf-md-chip`      |
| Cancel/Close       | 󰅖   | `\u{f0156}` | `nf-md-close`     |

The mapping is defined in the model crate as a utility function:

```rust
/// Returns the Nerd Font icon name for a given power action.
pub fn power_action_icon(action: &PowerAction) -> &'static str {
    match action {
        PowerAction::Shutdown => "nf-md-power",
        PowerAction::Reboot => "nf-md-restart",
        PowerAction::Suspend => "nf-md-sleep",
        PowerAction::Hibernate => "nf-md-snowflake",
        PowerAction::Lock => "nf-md-lock",
        PowerAction::Logout => "nf-md-logout",
        PowerAction::RebootToFirmware => "nf-md-chip",
        PowerAction::Cancel => "nf-md-close",
    }
}
```

### 5.9 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::command::PowerCommandAction;
pub use messages::command::PowerCommandMessage;
pub use messages::capabilities::PowerCapabilities;
pub use messages::icon::power_action_icon;
pub use messages::inhibitor::InhibitorInfo;
pub use messages::power_action::PowerAction;
pub use messages::scheduled::ScheduledActionInfo;
pub use messages::status::PowerStatusMessage;
```

---

## 6. Service Crate (`services/power`)

### 6.1 File Structure

- `service.rs` - `PowerService` struct and trait implementations
- `config.rs` - `PowerServiceConfig` struct and parsing
- `dbus.rs` - D-Bus proxy definitions and communication logic
- `scheduler.rs` - Scheduled action and countdown timer logic
- `lib.rs` - `service_plugin!` macro invocation

### 6.2 Service Implementation

```rust
pub struct PowerService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PowerServiceConfig,
    pub state: Arc<RwLock<PowerStatusMessage>>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<PowerCommand>,
}

/// Internal command union for the service event loop.
pub enum PowerCommand {
    /// Execute a power action (with countdown if configured).
    Execute(PowerAction),
    /// Schedule a power action for the future.
    Schedule(PowerAction, u64),
    /// Cancel a running countdown or scheduled action.
    Cancel,
    /// Refresh capabilities and inhibitors from the system.
    Refresh,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<PowerCommandMessage>>` - Processes commands from widgets and MCP clients
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 6.3 Configuration

```rust
/// Configuration for the power service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PowerServiceConfig {
    /// Countdown duration in seconds before executing a power action.
    /// Set to 0 to disable the countdown and execute immediately.
    pub countdown_seconds: u32,
    /// Whether to query inhibitors before showing the widget.
    pub enable_inhibitor_detection: bool,
    /// Whether to enable scheduled actions (sleep timer).
    pub enable_scheduled_actions: bool,
    /// Interval in seconds for refreshing capabilities and inhibitors.
    pub refresh_interval_seconds: u64,
    /// Custom lock command (e.g., "hyprlock"). If empty, uses D-Bus session lock.
    pub lock_command: stabby::string::String,
    /// Custom logout command (e.g., "hyprctl dispatch exit"). If empty, uses D-Bus session terminate.
    pub logout_command: stabby::string::String,
}

impl Default for PowerServiceConfig {
    fn default() -> Self {
        Self {
            countdown_seconds: 3,
            enable_inhibitor_detection: true,
            enable_scheduled_actions: true,
            refresh_interval_seconds: 30,
            lock_command: stabby::string::String::from(""),
            logout_command: stabby::string::String::from(""),
        }
    }
}
```

### 6.4 D-Bus Communication

The service uses `zbus` to communicate with `org.freedesktop.login1`. The D-Bus proxy interface is defined in `dbus.rs`:

```rust
/// D-Bus proxy for `org.freedesktop.login1.Manager`.
#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    fn power_off(&self, interactive: bool) -> zbus::Result<()>;
    fn reboot(&self, interactive: bool) -> zbus::Result<()>;
    fn suspend(&self, interactive: bool) -> zbus::Result<()>;
    fn hibernate(&self, interactive: bool) -> zbus::Result<()>;
    fn can_power_off(&self) -> zbus::Result<String>;
    fn can_reboot(&self) -> zbus::Result<String>;
    fn can_suspend(&self) -> zbus::Result<String>;
    fn can_hibernate(&self) -> zbus::Result<String>;
    fn can_reboot_to_firmware(&self) -> zbus::Result<String>;
    fn set_reboot_to_firmware(&self, enable: bool) -> zbus::Result<()>;
    fn list_inhibitors(&self) -> zbus::Result<Vec<(String, String, String, String, u32, u32)>>;
}

/// D-Bus proxy for `org.freedesktop.login1.Session`.
#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait LoginSession {
    fn lock(&self) -> zbus::Result<()>;
    fn terminate(&self) -> zbus::Result<()>;
}
```

**Action mapping:**

| `PowerAction`      | D-Bus Call                                                     |
|--------------------|----------------------------------------------------------------|
| `Shutdown`         | `LoginManager::power_off(false)`                               |
| `Reboot`           | `LoginManager::reboot(false)`                                  |
| `Suspend`          | `LoginManager::suspend(false)`                                 |
| `Hibernate`        | `LoginManager::hibernate(false)`                               |
| `Lock`             | `LoginSession::lock()` or custom `lock_command`                |
| `Logout`           | `LoginSession::terminate()` or custom `logout_command`         |
| `RebootToFirmware` | `LoginManager::set_reboot_to_firmware(true)` + `reboot(false)` |

### 6.5 Capability and Inhibitor Queries

On startup and at the configured refresh interval, the service queries all `Can*` properties from `org.freedesktop.login1.Manager` and builds a
`PowerCapabilities` struct. If `enable_inhibitor_detection` is set, it also calls `ListInhibitors` and maps the result to a list of `InhibitorInfo`.

```rust
async fn refresh_capabilities(connection: &zbus::Connection) -> PowerCapabilities {
    let manager = LoginManagerProxy::new(connection).await;
    match manager {
        Ok(proxy) => PowerCapabilities {
            can_shutdown: proxy.can_power_off().await.unwrap_or_default() == "yes",
            can_reboot: proxy.can_reboot().await.unwrap_or_default() == "yes",
            can_suspend: proxy.can_suspend().await.unwrap_or_default() == "yes",
            can_hibernate: proxy.can_hibernate().await.unwrap_or_default() == "yes",
            can_reboot_to_firmware: proxy.can_reboot_to_firmware().await.unwrap_or_default() == "yes",
            can_lock: true,
            can_logout: true,
        },
        Err(_) => PowerCapabilities::default(),
    }
}

async fn refresh_inhibitors(connection: &zbus::Connection) -> Vec<InhibitorInfo> {
    let manager = LoginManagerProxy::new(connection).await;
    match manager {
        Ok(proxy) => {
            let raw = proxy.list_inhibitors().await.unwrap_or_default();
            raw.into_iter().map(|(what, who, why, process_name, _, _)| InhibitorInfo {
                what: stabby::string::String::from(what),
                who: stabby::string::String::from(who),
                reason: stabby::string::String::from(why),
                process_name: stabby::string::String::from(process_name),
            }).collect()
        }
        Err(_) => Vec::new(),
    }
}
```

### 6.6 Countdown Logic

When a power action is requested with `countdown_seconds > 0`, the service does not execute the action immediately. Instead, it starts a countdown timer
and broadcasts status updates every second with `countdown_active = true` and `countdown_remaining_seconds` decreasing. If a `Cancel` command arrives
during the countdown, the action is aborted and `countdown_active` is set to `false`.

```rust
async fn run_countdown(
    action: PowerAction,
    seconds: u32,
    state: Arc<RwLock<PowerStatusMessage>>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
    cancel_token: tokio::sync::CancellationToken,
) {
    for remaining in (1..=seconds).rev() {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                let mut current = state.write().await;
                current.countdown_active = false;
                current.countdown_remaining_seconds = 0;
                let cancelled_status = current.clone();
                drop(current);
                broadcaster.broadcast_topic(TOPIC_STATUS, cancelled_status);
                return;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                let mut current = state.write().await;
                current.countdown_active = true;
                current.countdown_remaining_seconds = remaining;
                current.countdown_action = action.clone();
                let status = current.clone();
                drop(current);
                broadcaster.broadcast_topic(TOPIC_STATUS, status);
            }
        }
    }

    // Countdown expired, execute the action
    execute_power_action(action).await;
    let mut current = state.write().await;
    current.countdown_active = false;
    current.countdown_remaining_seconds = 0;
    let status = current.clone();
    drop(current);
    broadcaster.broadcast_topic(TOPIC_STATUS, status);
}
```

### 6.7 Scheduled Actions (Sleep Timer)

When `enable_scheduled_actions` is set, the service accepts `Schedule` commands with a `delay_minutes` parameter. The scheduler stores the scheduled
action and broadcasts periodic status updates with `scheduled_action` populated. The widget can display the remaining time. A `Cancel` command aborts
the scheduled action.

```rust
async fn run_scheduled_action(
    action: PowerAction,
    delay_seconds: u64,
    state: Arc<RwLock<PowerStatusMessage>>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
    cancel_token: tokio::sync::CancellationToken,
) {
    let start = Instant::now();
    loop {
        let elapsed = start.elapsed().as_secs();
        let remaining = delay_seconds.saturating_sub(elapsed);

        if remaining == 0 {
            execute_power_action(action).await;
            let mut current = state.write().await;
            current.scheduled_action = stabby::option::Option::None;
            let status = current.clone();
            drop(current);
            broadcaster.broadcast_topic(TOPIC_STATUS, status);
            return;
        }

        tokio::select! {
            _ = cancel_token.cancelled() => {
                let mut current = state.write().await;
                current.scheduled_action = stabby::option::Option::None;
                let cancelled_status = current.clone();
                drop(current);
                broadcaster.broadcast_topic(TOPIC_STATUS, cancelled_status);
                return;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                let mut current = state.write().await;
                current.scheduled_action = stabby::option::Option::Some(ScheduledActionInfo {
                    action: action.clone(),
                    remaining_seconds: remaining,
                    total_delay_seconds: delay_seconds,
                });
                let status = current.clone();
                drop(current);
                broadcaster.broadcast_topic(TOPIC_STATUS, status);
            }
        }
    }
}
```

### 6.8 Background Update Loop

On initialization, the service spawns a dedicated OS thread with a single-threaded Tokio runtime. The runtime runs an update loop that refreshes
capabilities and inhibitors at the configured interval, and processes incoming commands.

```rust
async fn run_update_loop(
    config: PowerServiceConfig,
    state: Arc<RwLock<PowerStatusMessage>>,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<PowerCommand>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    let connection = zbus::Connection::system().await;
    let mut interval = tokio::time::interval(Duration::from_secs(config.refresh_interval_seconds));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut countdown_cancel: Option<tokio::sync::CancellationToken> = None;
    let mut schedule_cancel: Option<tokio::sync::CancellationToken> = None;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Ok(ref conn) = connection {
                    let capabilities = refresh_capabilities(conn).await;
                    let inhibitors = if config.enable_inhibitor_detection {
                        refresh_inhibitors(conn).await
                    } else {
                        Vec::new()
                    };
                    let mut current = state.write().await;
                    current.capabilities = capabilities;
                    current.inhibitors = inhibitors.into();
                    current.last_updated = stabby::string::String::from(current_iso8601());
                    let status = current.clone();
                    drop(current);
                    broadcaster.broadcast_topic(TOPIC_STATUS, status);
                }
            }
            Some(command) = command_receiver.recv() => {
                match command {
                    PowerCommand::Execute(action) => {
                        if let Some(token) = countdown_cancel.take() {
                            token.cancel();
                        }
                        if config.countdown_seconds > 0 && action != PowerAction::Cancel {
                            let token = tokio::sync::CancellationToken::new();
                            countdown_cancel = Some(token.clone());
                            let state_clone = state.clone();
                            let broadcaster_clone = broadcaster.clone();
                            tokio::spawn(async move {
                                run_countdown(action, config.countdown_seconds, state_clone, broadcaster_clone, token).await;
                            });
                        } else {
                            execute_power_action(action).await;
                        }
                    }
                    PowerCommand::Schedule(action, delay_minutes) => {
                        if let Some(token) = schedule_cancel.take() {
                            token.cancel();
                        }
                        let token = tokio::sync::CancellationToken::new();
                        schedule_cancel = Some(token.clone());
                        let state_clone = state.clone();
                        let broadcaster_clone = broadcaster.clone();
                        let delay_seconds = delay_minutes * 60;
                        tokio::spawn(async move {
                            run_scheduled_action(action, delay_seconds, state_clone, broadcaster_clone, token).await;
                        });
                    }
                    PowerCommand::Cancel => {
                        if let Some(token) = countdown_cancel.take() {
                            token.cancel();
                        }
                        if let Some(token) = schedule_cancel.take() {
                            token.cancel();
                        }
                    }
                    PowerCommand::Refresh => {
                        if let Ok(ref conn) = connection {
                            let capabilities = refresh_capabilities(conn).await;
                            let inhibitors = if config.enable_inhibitor_detection {
                                refresh_inhibitors(conn).await
                            } else {
                                Vec::new()
                            };
                            let mut current = state.write().await;
                            current.capabilities = capabilities;
                            current.inhibitors = inhibitors.into();
                            current.last_updated = stabby::string::String::from(current_iso8601());
                            let status = current.clone();
                            drop(current);
                            broadcaster.broadcast_topic(TOPIC_STATUS, status);
                        }
                    }
                }
            }
        }
    }
}
```

### 6.9 MCP Resources

The service registers the following MCP resources via the Plugin-Resource-Registry:

| URI                         | Description                                                                    | Source type           |
|-----------------------------|--------------------------------------------------------------------------------|-----------------------|
| `power://capabilities`      | JSON indicating what the system can do (e.g., `can_suspend`, `can_hibernate`). | `PowerCapabilities`   |
| `power://inhibitors`        | List of all processes currently blocking a shutdown or sleep.                  | `Vec<InhibitorInfo>`  |
| `power://scheduled_actions` | Shows whether a sleep timer or scheduled reboot is currently active.           | `ScheduledActionInfo` |

Example `power://capabilities` response:

```json
{
  "can_shutdown": true,
  "can_reboot": true,
  "can_suspend": true,
  "can_hibernate": false,
  "can_reboot_to_firmware": true,
  "can_lock": true,
  "can_logout": true
}
```

Example `power://inhibitors` response:

```json
[
  {
    "process_name": "apt",
    "reason": "APT is running a system upgrade",
    "what": "shutdown:sleep",
    "who": "APT"
  }
]
```

### 6.10 MCP Tools

The service registers the following MCP tools via the Plugin-Tool-Registry:

| Tool                           | Description                                                        | Parameters                                                            |
|--------------------------------|--------------------------------------------------------------------|-----------------------------------------------------------------------|
| `system_power_action`          | Executes the desired power action immediately.                     | `action: string` (shutdown, reboot, suspend, hibernate, lock, logout) |
| `system_schedule_power_action` | Schedules a shutdown or reboot in the future.                      | `action: string`, `delay_minutes: integer`                            |
| `system_cancel_power_action`   | Cancels a running shutdown timer or scheduled action.              | -                                                                     |
| `system_reboot_to_uefi`        | Sets the firmware reboot flag and reboots directly into BIOS/UEFI. | -                                                                     |

> **MCP tool naming convention:** Tool names use `snake_case` with underscores, never dots. Dots in tool names cause schema validation failures in LLM
> gateways (Windsurf, Claude, etc.). This is consistent with existing tools like `sysinfo_refresh` and `get_current_time`.

**Example JSON schema for `system_power_action`:**

```json
{
  "name": "system_power_action",
  "description": "Executes the desired power action immediately.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": [
          "shutdown",
          "reboot",
          "suspend",
          "hibernate",
          "lock",
          "logout"
        ],
        "description": "The power action to execute"
      }
    },
    "required": [
      "action"
    ]
  }
}
```

**Example JSON schema for `system_schedule_power_action`:**

```json
{
  "name": "system_schedule_power_action",
  "description": "Schedules a shutdown or reboot in the future. Useful for prompts like: 'Shut down the computer in 30 minutes.'",
  "inputSchema": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": [
          "shutdown",
          "reboot"
        ],
        "description": "The power action to schedule"
      },
      "delay_minutes": {
        "type": "integer",
        "minimum": 1,
        "description": "Delay in minutes before the action executes"
      }
    },
    "required": [
      "action",
      "delay_minutes"
    ]
  }
}
```

---

## 7. Widget Crate (`plugins/power`)

### 7.1 Overview

The Power Menu Widget is a GTK4 menu that displays buttons for each power action. It subscribes to `service.power.status` and updates its display based on
system capabilities and active inhibitors. When a power action is clicked, a countdown overlay appears with a cancel button. The widget also supports
displaying scheduled action status.

### 7.2 File Structure

- `widget.rs` - `PowerWidget` struct and trait implementations
- `config.rs` - `PowerWidgetConfig` struct and parsing
- `menu.rs` - Menu button rendering and layout
- `overlay.rs` - Countdown overlay and inhibitor warning rendering
- `lib.rs` - `widget_plugin!` macro invocation

### 7.3 Widget Configuration

```rust
/// Configuration for the power menu widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PowerWidgetConfig {
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Spacing between buttons.
    pub spacing: i32,
    /// Whether to show the shutdown button.
    pub show_shutdown: bool,
    /// Whether to show the reboot button.
    pub show_reboot: bool,
    /// Whether to show the suspend button.
    pub show_suspend: bool,
    /// Whether to show the hibernate button.
    pub show_hibernate: bool,
    /// Whether to show the lock screen button.
    pub show_lock: bool,
    /// Whether to show the logout button.
    pub show_logout: bool,
    /// Whether to show the reboot-to-firmware button.
    pub show_reboot_to_firmware: bool,
    /// Whether to show inhibitor warnings.
    pub show_inhibitor_warnings: bool,
    /// Whether to show the countdown overlay.
    pub show_countdown_overlay: bool,
    /// Whether to show the scheduled action status.
    pub show_scheduled_status: bool,
    /// Button size in pixels.
    pub button_size: i32,
    /// Icon size in pixels.
    pub icon_size: i32,
    /// Background color of the widget.
    pub background_color: Option<String>,
    /// Message topic for single-click (opens the power menu area).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
}

impl Default for PowerWidgetConfig {
    fn default() -> Self {
        Self {
            width: 200,
            height: 240,
            spacing: 8,
            show_shutdown: true,
            show_reboot: true,
            show_suspend: true,
            show_hibernate: true,
            show_lock: true,
            show_logout: true,
            show_reboot_to_firmware: true,
            show_inhibitor_warnings: true,
            show_countdown_overlay: true,
            show_scheduled_status: true,
            button_size: 48,
            icon_size: 24,
            background_color: None,
            click_topic: None,
            click_payload: None,
        }
    }
}
```

### 7.4 Widget Implementation

```rust
pub struct PowerWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PowerWidgetConfig,
    pub current_status: Option<PowerStatusMessage>,
}
```

> **GTK widget references:** GTK4 widgets (`gtk4::Box`, `gtk4::Button`, `gtk4::Label`) are **not** `Send` or `Sync`. They must not be stored in
> `Arc<RwLock<...>>` inside the plugin struct. Instead, widget references are captured inside `glib::clone!` closures or passed directly to
> `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state (`config`, `current_status`).

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<PowerStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 7.5 Menu Layout

The widget renders a vertical or grid layout of buttons, one per enabled power action. Each button displays the corresponding Nerd Font icon. Buttons for
actions that are not supported by the system (according to `PowerCapabilities`) are hidden or grayed out.

```
+----------------------+
|  󰐥  󰑐  󰤓  󰜡       |
|  Shutdown Reboot Suspend Hibernate
|
|  󰌾  󰍃  󰘩           |
|  Lock    Logout  UEFI
+----------------------+
```

### 7.6 Inhibitor Warning

When `show_inhibitor_warnings` is set and the status message contains inhibitors, the widget displays a small warning banner above the buttons:

```
+----------------------+
|  ⚠ Sleep blocked by: |
|    APT (system upgrade) |
|  󰐥  󰑐  󰤓  󰜡       |
+----------------------+
```

The warning uses `nf-md-alert` or `nf-md-alert_circle` as the icon. The inhibitor `who` and `reason` fields are displayed as text.

### 7.7 Countdown Overlay

When `show_countdown_overlay` is set and the status message indicates `countdown_active = true`, the widget overlays a countdown display on top of the
menu:

```
+----------------------+
|                      |
|     Shutting down    |
|        in 3...       |
|                      |
|    [ 󰅖 Cancel ]     |
|                      |
+----------------------+
```

The countdown overlay shows:

- The power action being executed (with its icon).
- The remaining seconds as a large number.
- A cancel button (`nf-md-close`) that sends a `Cancel` command to the service.

All GTK updates happen via `glib::MainContext::spawn_local` to ensure thread safety.

### 7.8 Scheduled Action Display

When `show_scheduled_status` is set and the status message contains a `scheduled_action`, the widget displays a small status line at the bottom:

```
+----------------------+
|  󰐥  󰑐  󰤓  󰜡       |
|  󰌾  󰍃  󰘩           |
|                      |
|  󰐥 in 45:00  [󰅖]   |
+----------------------+
```

The status line shows the scheduled action icon, a formatted countdown (`MM:SS`), and a cancel button.

### 7.9 State Synchronization

The widget subscribes to `service.power.status`. When a new `PowerStatusMessage` arrives:

1. The message is deserialized and stored in `current_status`.
2. The menu is re-rendered: buttons are shown/hidden based on `capabilities`.
3. Inhibitor warnings are updated.
4. The countdown overlay is shown or hidden based on `countdown_active`.
5. The scheduled action status is updated.
6. All GTK updates happen via `glib::MainContext::spawn_local`.

---

## 8. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
| Power Widget      |<--------|                   |-------->| Power Service     |
| (menu in area)    |  Status |   Event Broker    | Command | (Singleton)       |
+---------+---------+ Broadcast +-------------------+ Broadcast +-------------------+
          |                                                 |
          | Click: send PowerCommandMessage                 | zbus D-Bus
          | Longpress: (optional)                           | org.freedesktop.login1
          v                                                 v
+-------------------+                               +-------------------+
| Countdown overlay |                               | systemd-logind    |
| (local state)     |                               | /org/freedesktop/ |
+-------------------+                               |   login1          |
                                                    +-------------------+
```

---

## 9. Configuration Example

### 9.1 Service Registration in `services.toml`

```toml
[[services]]
id = "power"
path = "target/release/libsmearor_power_service.so"

[power]
countdown_seconds = 3
enable_inhibitor_detection = true
enable_scheduled_actions = true
refresh_interval_seconds = 30
lock_command = "hyprlock"
logout_command = "hyprctl dispatch exit"
```

### 9.2 Widget Configuration in `config.toml`

```toml
[[scroll_band.plugins]]
id = "power_widget"
path = "target/release/libsmearor_power_widget.so"

[power_widget]
width = 200
height = 240
show_shutdown = true
show_reboot = true
show_suspend = true
show_hibernate = true
show_lock = true
show_logout = true
show_reboot_to_firmware = true
show_inhibitor_warnings = true
show_countdown_overlay = true
show_scheduled_status = true
button_size = 48
icon_size = 24

# Click opens the power menu area
click_topic = "area.open"
click_payload = { area_id = "power_area" }
```

### 9.3 Minimal Widget Configuration (shutdown + reboot + lock only)

```toml
[[scroll_band.plugins]]
id = "power_widget"
path = "target/release/libsmearor_power_widget.so"

[power_widget]
show_shutdown = true
show_reboot = true
show_suspend = false
show_hibernate = false
show_lock = true
show_logout = false
show_reboot_to_firmware = false
```

---

## 10. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Power Menu feature. The order is chosen so that each layer
is built on top of already-tested foundations.

### Phase 1: Foundation — Model Crate (`model/power`)

**Goal:** Define all shared messages, topics, and configuration types.

**Order:**

1. Create the crate `model/power` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_COMMAND` and `TOPIC_STATUS`.
3. Create one file per message struct:
    - `src/messages/power_action.rs` -> `PowerAction` enum
    - `src/messages/command.rs` -> `PowerCommandAction` and `PowerCommandMessage`
    - `src/messages/capabilities.rs` -> `PowerCapabilities`
    - `src/messages/inhibitor.rs` -> `InhibitorInfo`
    - `src/messages/scheduled.rs` -> `ScheduledActionInfo`
    - `src/messages/status.rs` -> `PowerStatusMessage`
    - `src/messages/icon.rs` -> `power_action_icon` mapping function
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.
- The `power_action_icon` function returns correct icon names for all `PowerAction` variants.

---

### Phase 2: Backend — Service Crate (`services/power`)

**Goal:** Communicate with systemd-logind via D-Bus and publish power status.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/power` with a `Cargo.toml` that depends on the `model/power` crate, the project plugin API, `zbus`, `tokio`, and `tracing`.
2. Create `src/config.rs` with `PowerServiceConfig` and its default values.
3. Create `src/dbus.rs` and implement the D-Bus proxy definitions for `org.freedesktop.login1.Manager` and `org.freedesktop.login1.Session`.
4. Implement `refresh_capabilities` and `refresh_inhibitors` functions.
5. Implement `execute_power_action` that maps each `PowerAction` to the corresponding D-Bus call.
6. Create `src/scheduler.rs` and implement `run_countdown` and `run_scheduled_action`.
7. Create `src/service.rs` with `PowerService` and all required trait implementations.
8. Implement `run_update_loop` to refresh capabilities/inhibitors at the configured interval and process incoming commands.
9. Register MCP resources (`power://capabilities`, `power://inhibitors`, `power://scheduled_actions`) and MCP tools (`system_power_action`,
   `system_schedule_power_action`, `system_cancel_power_action`, `system_reboot_to_uefi`).
10. Wire `service_plugin!` in `src/lib.rs`.
11. Add unit tests for capability parsing and action mapping.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for capability parsing and action mapping produce correct results.
- Running the service broadcasts `TOPIC_STATUS` at least once per refresh interval.
- MCP resources are registered and return JSON when queried.
- The `system_power_action` tool triggers the corresponding D-Bus call.
- The `system_schedule_power_action` tool schedules a delayed action.
- The `system_cancel_power_action` tool cancels a running countdown or scheduled action.
- The `system_reboot_to_uefi` tool sets the firmware flag and reboots.
- Countdown and scheduled action timers can be cancelled mid-flight.

---

### Phase 3: Display — Widget Crate (`plugins/power`)

**Goal:** Provide a GTK4 power menu with buttons, countdown overlay, and inhibitor warnings.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/power` with a `Cargo.toml` that depends on `model/power`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with `PowerWidgetConfig` including all visibility flags.
3. Create `src/menu.rs` and implement button rendering for each `PowerAction`:
    - Shutdown, Reboot, Suspend, Hibernate, Lock, Logout, RebootToFirmware.
    - Hide or gray out buttons for unsupported actions based on `PowerCapabilities`.
4. Create `src/overlay.rs` and implement:
    - Countdown overlay with remaining seconds and cancel button.
    - Inhibitor warning banner.
    - Scheduled action status line.
5. Create `src/widget.rs` with `PowerWidget` and all required trait implementations.
6. Implement click handling: send `PowerCommandMessage` with `Execute` action to the service.
7. Implement cancel button: send `PowerCommandMessage` with `Cancel` action.
8. Subscribe to `TOPIC_STATUS` and update `current_status` + re-render on every message.
9. Wire `widget_plugin!` in `src/lib.rs`.
10. Add an integration test that verifies the widget accepts `TOPIC_STATUS` and renders buttons.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The widget displays buttons for all enabled and supported power actions.
- The widget shows a countdown overlay when `countdown_active` is true.
- The widget shows inhibitor warnings when inhibitors are present.
- The widget shows scheduled action status when a scheduled action is active.
- Clicking a button sends the correct command to the service.
- Clicking cancel aborts the countdown.
- Unsupported actions are hidden or grayed out.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/power` and `services/power` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `power` in `config.toml`.
4. Add a sample widget configuration for the power widget.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The power widget receives messages and renders correctly.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that `TOPIC_STATUS` appears on the message broker.
2. Verify the widget displays buttons for all supported power actions.
3. Verify clicking a button triggers the countdown overlay.
4. Verify the cancel button aborts the countdown.
5. Verify inhibitor warnings appear when inhibitors are active.
6. Verify scheduled actions display the remaining time.
7. Verify MCP resources return valid JSON.
8. Verify the `system_power_action` tool triggers the correct D-Bus call.
9. Verify the `system_schedule_power_action` tool schedules a delayed action.
10. Verify the `system_cancel_power_action` tool cancels a running timer.
11. Verify the `system_reboot_to_uefi` tool sets the firmware flag and reboots.
12. Run `cargo test` for all three crates.
13. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- The widget renders correctly for all power actions.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.
- D-Bus communication works without requiring sudo or password prompts.
- Countdown and scheduled action timers are cancellable.
- MCP tools return valid JSON and execute the correct actions.

---

### Summary of Order

```
Phase 1: model/power
    |
    v
Phase 2: services/power
    |
    v
Phase 3: plugins/power
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

### Rationale

- **Model first:** Message formats and action definitions must exist before the service or widget can use them.
- **Service second:** The widget needs a running publisher to test against. D-Bus communication is the core logic.
- **Widget third:** The display widget depends on the service's status topic.
- **Wiring fourth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 11. Technical Notes

- **D-Bus over shell commands:** Using `zbus` to communicate with `org.freedesktop.login1` is the architecturally correct approach. It avoids spawning
  subprocesses, does not require sudo passwords (Polkit grants the active user these rights), and is faster and more secure.
- **Lock and Logout flexibility:** The service supports custom lock and logout commands via configuration. This allows compatibility with Hyprland
  (`hyprlock`, `hyprctl dispatch exit`) or Gnome/GDM. If no custom command is configured, the service falls back to D-Bus session lock/terminate.
- **Reboot to firmware:** The service calls `SetRebootToFirmware(true)` on `org.freedesktop.login1.Manager` before calling `Reboot(false)`. This sets the
  motherboard flag so the next boot enters BIOS/UEFI. The capability is checked via `CanRebootToFirmware`.
- **Inhibitor detection:** Before showing the widget, the service queries `ListInhibitors` from logind. If inhibitors are active (e.g., APT is running an
  upgrade, or a video player is inhibiting sleep), the widget displays a warning. This prevents accidental shutdowns during critical operations.
- **Countdown overlay:** The countdown prevents accidental triggers from touch/swipe gestures. The default is 3 seconds. The countdown can be disabled by
  setting `countdown_seconds = 0` in the service configuration.
- **Scheduled actions:** The sleep timer feature allows scheduling a shutdown or reboot in the future. This is useful for running long processes overnight.
  The scheduled action can be cancelled at any time.
- **No polling in the widget:** The widget updates exclusively through incoming messages. Periodic polling only happens in the service.
- **GTK widget ownership:** GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget
  references are captured in `glib::clone!` closures or `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.
- **MCP tool naming:** Tool names use `snake_case` with underscores, never dots. Dots cause schema validation failures in LLM gateways. This is consistent
  with existing tools (`sysinfo_refresh`, `get_current_time`, `weather_refresh`).
- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in `model/notifications`, `model/audio`, and `model/app-launcher`.
- **Capability-based UI:** The widget hides or grays out buttons for actions that the system does not support (e.g., `can_hibernate = false`). This prevents
  the user from clicking buttons that would fail.

---

## 12. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/power`, `services/power`, and `plugins/power`.
- **One struct per file:** Each message struct and each enum lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String` and optional strings use
  `stabby::option::Option<stabby::string::String>` to maintain ABI stability across compiler invocations.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `zbus`, `tokio`, and `tracing`; the widget uses `gtk4` and `glib`.

---

*End of document.*
