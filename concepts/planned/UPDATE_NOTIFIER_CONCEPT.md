# Concept: Update Notifier Service & Widget

This document describes the concept for an **Update Notifier Service** and an **Update Notifier Widget** in the *Smearor Swipe Launcher*. The service
periodically checks whether system package updates are available by invoking distribution-specific CLI commands. The widget displays the update status and
allows the user to trigger an update with a single click.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/update-notifier`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/update-notifier`):** Singleton background service that queries the package manager via CLI commands, broadcasts update status, and
   executes update commands.
3. **Widget Crate (`plugins/update-notifier`):** Pure GTK4 UI that displays update status, package count, and triggers updates on click.

---

## 1. Problem & Motivation

There is no cross-distribution mechanism to check for available system updates. Each Linux distribution uses its own package manager with its own CLI interface:

- **Arch-based distributions:** `checkupdates` lists available updates from the Arch repositories.
- **Debian-based distributions:** `apt-get -s upgrade` simulates an upgrade and lists packages that would be upgraded.

The Update Notifier abstracts these distribution-specific commands behind a **provider trait**. The service queries the active provider periodically and
broadcasts the result. The widget renders the status and allows the user to initiate an update.

---

## 2. Feature Scope

| Feature                   | Description                                                                               |
|---------------------------|-------------------------------------------------------------------------------------------|
| **Update Check**          | Periodically queries the package manager for available updates and counts the packages.   |
| **Status Display**        | Shows whether the system is up-to-date or updates are available, including package count. |
| **Update Execution**      | Allows the user to trigger a system update directly from the widget.                      |
| **Update In Progress**    | Shows a spinning icon and "Updating..." text while the update is running.                 |
| **Provider Abstraction**  | Pluggable provider trait supports Arch-based and Debian-based distributions.              |
| **Configurable Provider** | The provider can be selected via configuration.                                           |

---

## 3. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Update Notifier Widget   |                 | Update Notifier Service    |
| (subscribed to           |                 | (Singleton)                |
|  service.update_notifier |                 |                            |
|  .status)                |                 |                            |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Command Message                         |
             |  (check now, start update)                  |
             |===========================================> |
             |  Topic: "service.update_notifier.command"   |
             |                                             |
             |                                             |  2. CLI command execution
             |                                             |     checkupdates (Arch)
             |                                             |     apt-get -s upgrade (Debian)
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.update_notifier.status"
             |                                             |     Payload: UpdateStatusMessage { ... }
+--------------------------+                 +----------------------------+
```

---

## 4. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path                        | Responsibility                                                        |
|-------------|-----------------------------|-----------------------------------------------------------------------|
| **Model**   | `model/update-notifier/`    | Shared structs, enums, topics, and message formats                    |
| **Service** | `services/update-notifier/` | CLI command execution, provider abstraction, update status broadcasts |
| **Widget**  | `plugins/update-notifier/`  | GTK4 UI with icon, text, and click-to-update action                   |

---

## 5. Model Crate (`model/update-notifier`)

### 5.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.update_notifier.command";
pub const TOPIC_STATUS: &str = "service.update_notifier.status";
```

### 5.2 Update State Enum

```rust
/// Current state of the update system.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum UpdateState {
    /// No updates are available; the system is up-to-date.
    #[default]
    UpToDate,
    /// Updates are available and waiting to be installed.
    UpdatesAvailable,
    /// An update process is currently running.
    Updating,
}
```

### 5.3 Update Status Message (Service -> Widget)

```rust
/// Update status message broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct UpdateStatusMessage {
    /// Current update state.
    pub state: UpdateState,
    /// Whether updates are available.
    pub updates_available: bool,
    /// Number of packages that can be updated.
    pub package_count: u32,
    /// Timestamp of the last check as ISO-8601 string.
    pub last_checked: stabby::string::String,
}
```

### 5.4 Command Action Enum

```rust
/// Actions the update notifier service can perform on request.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum UpdateCommandAction {
    /// Check for available updates immediately.
    #[default]
    CheckNow,
    /// Start the system update process.
    StartUpdate,
}
```

### 5.5 Command Message (Widget -> Service)

```rust
/// Command message sent by widgets to the update notifier service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct UpdateCommandMessage {
    /// The action to execute.
    pub action: UpdateCommandAction,
}
```

### 5.6 Nerd Font Icon Mapping

Each update state maps to a Material Design Nerd Font icon for consistent GTK4 rendering.

| State             | Icon | Nerd Font Name |
|-------------------|------|----------------|
| Up-to-date        | 󰗠    | `nf-md-check`  |
| Updates available | 󰚰    | `nf-md-update` |
| Updating          | 󰚰    | `nf-md-update` |

The mapping is defined in the model crate as a utility function:

```rust
/// Returns the Nerd Font icon name for an update state.
pub fn update_state_icon(state: &UpdateState) -> &'static str {
    match state {
        UpdateState::UpToDate => "nf-md-check",
        UpdateState::UpdatesAvailable => "nf-md-update",
        UpdateState::Updating => "nf-md-update",
    }
}
```

### 5.7 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::command::UpdateCommandAction;
pub use messages::command::UpdateCommandMessage;
pub use messages::icon::update_state_icon;
pub use messages::state::UpdateState;
pub use messages::status::UpdateStatusMessage;
```

### 5.8 File Structure

```
model/update-notifier/
  Cargo.toml
  src/
    lib.rs
    json_converters.rs
    messages/
      mod.rs
      command.rs              # UpdateCommandAction, UpdateCommandMessage
      icon.rs                 # update_state_icon
      state.rs                # UpdateState
      status.rs               # UpdateStatusMessage
```

---

## 6. Service Crate (`services/update-notifier`)

### 6.1 File Structure

- `service.rs` - `UpdateNotifierService` struct and trait implementations
- `config.rs` - `UpdateNotifierServiceConfig` struct and parsing
- `provider.rs` - `UpdateProvider` trait definition
- `provider_arch.rs` - Arch-based distribution provider (`checkupdates`)
- `provider_debian.rs` - Debian-based distribution provider (`apt-get -s upgrade`)
- `lib.rs` - `service_plugin!` macro invocation

### 6.2 Provider Trait

The provider trait abstracts distribution-specific update checking and update execution:

```rust
/// Trait for distribution-specific update providers.
pub trait UpdateProvider: Send + Sync {
    /// Returns the number of packages that can be updated.
    fn check_updates(&self) -> Result<u32, UpdateProviderError>;

    /// Starts the system update process.
    fn start_update(&self) -> Result<(), UpdateProviderError>;
}
```

### 6.3 Arch-Based Provider (`provider_arch.rs`)

Uses the `checkupdates` command, which is part of the `pacman-contrib` package on Arch Linux. The command outputs one line per available update. The provider
counts the output lines.

```rust
/// Update provider for Arch-based distributions.
pub struct ArchUpdateProvider;

impl UpdateProvider for ArchUpdateProvider {
    fn check_updates(&self) -> Result<u32, UpdateProviderError> {
        let output = std::process::Command::new("checkupdates")
            .output()
            .map_err(|e| UpdateProviderError::CommandFailed(e.to_string()))?;

        let count = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .count() as u32;

        Ok(count)
    }

    fn start_update(&self) -> Result<(), UpdateProviderError> {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("sudo pacman -Syu --noconfirm")
            .spawn()
            .map_err(|e| UpdateProviderError::CommandFailed(e.to_string()))?;

        Ok(())
    }
}
```

### 6.4 Debian-Based Provider (`provider_debian.rs`)

Uses `apt-get -s upgrade` to simulate an upgrade. The output contains lines starting with `Inst` for each package that would be upgraded. The provider counts
those lines.

```rust
/// Update provider for Debian-based distributions.
pub struct DebianUpdateProvider;

impl UpdateProvider for DebianUpdateProvider {
    fn check_updates(&self) -> Result<u32, UpdateProviderError> {
        let output = std::process::Command::new("apt-get")
            .arg("-s")
            .arg("upgrade")
            .output()
            .map_err(|e| UpdateProviderError::CommandFailed(e.to_string()))?;

        let count = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.starts_with("Inst "))
            .count() as u32;

        Ok(count)
    }

    fn start_update(&self) -> Result<(), UpdateProviderError> {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("sudo apt-get upgrade -y")
            .spawn()
            .map_err(|e| UpdateProviderError::CommandFailed(e.to_string()))?;

        Ok(())
    }
}
```

### 6.5 Service Implementation

```rust
pub struct UpdateNotifierService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: UpdateNotifierServiceConfig,
    pub state: Arc<RwLock<UpdateStatusMessage>>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<UpdateCommand>,
}

/// Internal command union for the service event loop.
pub enum UpdateCommand {
    /// Check for available updates immediately.
    CheckNow,
    /// Start the system update process.
    StartUpdate,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<UpdateCommandMessage>>` - Processes commands from widgets
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 6.6 Configuration

```rust
/// Configuration for the update notifier service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UpdateNotifierServiceConfig {
    /// Interval in milliseconds for checking available updates.
    pub check_interval_ms: u64,
    /// The update provider to use ("arch" or "debian").
    pub provider: UpdateProviderType,
}

/// Supported update provider types.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum UpdateProviderType {
    /// Arch-based distribution provider (checkupdates).
    #[default]
    Arch,
    /// Debian-based distribution provider (apt-get -s upgrade).
    Debian,
}

impl Default for UpdateNotifierServiceConfig {
    fn default() -> Self {
        Self {
            check_interval_ms: 300_000,
            provider: UpdateProviderType::Arch,
        }
    }
}
```

### 6.7 Service Event Loop

The service spawns a dedicated OS thread with a single-threaded Tokio runtime, following the pattern from the existing `app-launcher` and `hyprland` services:

1. On startup, perform an initial update check.
2. Start a periodic timer based on `check_interval_ms`.
3. On each timer tick, query the provider for the update count.
4. Broadcast `UpdateStatusMessage` on `TOPIC_STATUS`.
5. When a `StartUpdate` command arrives, set state to `Updating`, broadcast status, execute the update, then re-check and broadcast the final state.

```rust
async fn run_event_loop(
    config: UpdateNotifierServiceConfig,
    provider: Box<dyn UpdateProvider>,
    state: Arc<RwLock<UpdateStatusMessage>>,
    broker: MessageBrokerHandle,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<UpdateCommand>,
) {
    // Initial check
    check_and_broadcast(&provider, &state, &broker).await;

    let mut interval = tokio::time::interval(
        std::time::Duration::from_millis(config.check_interval_ms),
    );

    loop {
        tokio::select! {
            _ = interval.tick() => {
                check_and_broadcast(&provider, &state, &broker).await;
            }
            Some(command) = command_receiver.recv() => {
                match command {
                    UpdateCommand::CheckNow => {
                        check_and_broadcast(&provider, &state, &broker).await;
                    }
                    UpdateCommand::StartUpdate => {
                        set_state(&state, UpdateState::Updating, &broker).await;
                        if let Err(error) = provider.start_update() {
                            tracing::error!("Update failed: {error}");
                        }
                        check_and_broadcast(&provider, &state, &broker).await;
                    }
                }
            }
        }
    }
}
```

---

## 7. Widget Crate (`plugins/update-notifier`)

### 7.1 File Structure

- `widget.rs` - `UpdateNotifierWidget` struct and trait implementations
- `config.rs` - `UpdateNotifierWidgetConfig` struct and parsing
- `lib.rs` - `widget_plugin!` macro invocation

### 7.2 Widget Display States

The widget renders differently based on the current `UpdateState`:

| State            | Icon           | Text                              | Click Action    |
|------------------|----------------|-----------------------------------|-----------------|
| UpToDate         | `nf-md-check`  | "Everything up to date"           | Check now       |
| UpdatesAvailable | `nf-md-update` | "{count} packages can be updated" | Start update    |
| Updating         | `nf-md-update` | "Updating..."                     | None (disabled) |

When the state is `Updating`, the icon rotates using a GTK4 CSS spin animation to provide visual feedback that the update is in progress.

### 7.3 Widget Implementation

```rust
pub struct UpdateNotifierWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: UpdateNotifierWidgetConfig,
    pub state: Arc<RwLock<UpdateStatusMessage>>,
    pub broker: MessageBrokerHandle,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<UpdateStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster` - Sends command messages to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget

### 7.4 Widget Configuration

```rust
/// Configuration for the update notifier widget.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UpdateNotifierWidgetConfig {
    /// Whether to show the package count text.
    pub show_package_count: bool,
    /// Whether to show the "Updating..." text during updates.
    pub show_updating_text: bool,
    /// Custom label for the up-to-date state.
    pub up_to_date_label: Option<String>,
    /// Custom label for the updates-available state (use {count} as placeholder).
    pub updates_available_label: Option<String>,
    /// Custom label for the updating state.
    pub updating_label: Option<String>,
}

impl UpdateNotifierWidgetConfig {
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
```

### 7.5 Widget Rendering

The widget uses a vertical `gtk4::Box` containing:

1. An icon label (`gtk4::Label`) displaying the Nerd Font icon.
2. A text label (`gtk4::Label`) displaying the status text.

On click, the widget sends an `UpdateCommandMessage` to the service:

- If state is `UpToDate` or `UpdatesAvailable`, send `CheckNow` or `StartUpdate` respectively.
- If state is `Updating`, the click handler is disabled.

GTK updates are performed via `glib::MainContext::spawn_local` to ensure thread safety. The widget subscribes to `TOPIC_STATUS` and re-renders on every incoming
message.

### 7.6 CSS Spin Animation

The rotating icon during updates is achieved via a CSS class:

```css
.update-notifier-spinning {
    animation: update-notifier-spin 1s linear infinite;
}

@keyframes update-notifier-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
}
```

The CSS class is added to the icon label when the state transitions to `Updating` and removed when the state changes away from `Updating`.

---

## 8. Configuration TOML

### 8.1 Service Configuration

```toml
[services.update_notifier]
path = "target/debug/libupdate_notifier_service.so"

[services.update_notifier.config]
check_interval_ms = 300000
provider = "arch"
```

### 8.2 Widget Configuration

```toml
[[areas.left.widgets]]
plugin = "target/debug/libupdate_notifier_widget.so"

[areas.left.widgets.config]
show_package_count = true
show_updating_text = true
up_to_date_label = "Everything up to date"
updates_available_label = "{count} packages can be updated"
updating_label = "Updating..."
```

---

## 9. Implementation Phases

### Phase 1: Foundation — Model Crate (`model/update-notifier`)

**Goal:** Define all shared messages, topics, and configuration types.

**Order:**

1. Create the crate `model/update-notifier` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/messages/state.rs` and declare the `UpdateState` enum.
3. Create `src/messages/status.rs` and declare the `UpdateStatusMessage` struct.
4. Create `src/messages/command.rs` and declare the `UpdateCommandAction` enum and `UpdateCommandMessage` struct.
5. Create `src/messages/icon.rs` and implement the `update_state_icon` mapping function.
6. Add `#[stabby::stabby]` to all FFI-relevant types.
7. Re-export all public types in `src/lib.rs`.
8. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.
- The icon mapping function returns correct icon names for all variants.

---

### Phase 2: Backend — Service Crate (`services/update-notifier`)

**Goal:** Implement the provider abstraction, CLI command execution, and status broadcasting.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/update-notifier` with a `Cargo.toml` that depends on the `model/update-notifier` crate, the project plugin API, `tokio`, and
   `tracing`.
2. Create `src/config.rs` with `UpdateNotifierServiceConfig` and `UpdateProviderType`.
3. Create `src/provider.rs` and define the `UpdateProvider` trait and `UpdateProviderError` error type.
4. Create `src/provider_arch.rs` and implement `ArchUpdateProvider` using `checkupdates`.
5. Create `src/provider_debian.rs` and implement `DebianUpdateProvider` using `apt-get -s upgrade`.
6. Create `src/service/loaded_service.rs` with `UpdateNotifierService` and all required trait implementations.
7. Implement the event loop with periodic check timer and command handling.
8. Implement `check_and_broadcast` helper that queries the provider and broadcasts `UpdateStatusMessage`.
9. Wire `service_plugin!` in `src/lib.rs`.
10. Add unit tests for the provider line-counting logic.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for provider line counting produce correct results.
- Running the service broadcasts `TOPIC_STATUS` at least once per check interval.
- The `StartUpdate` command triggers the provider's update method.
- No `unwrap`, `expect`, or `panic` remains in the new code.

---

### Phase 3: Display — Widget Crate (`plugins/update-notifier`)

**Goal:** Provide a GTK4 widget that displays update status and triggers updates on click.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/update-notifier` with a `Cargo.toml` that depends on `model/update-notifier`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with `UpdateNotifierWidgetConfig` and its `parse` method.
3. Create `src/widget.rs` with `UpdateNotifierWidget` and all required trait implementations.
4. Implement the widget rendering: icon label and text label in a vertical box.
5. Implement click handling: send `UpdateCommandMessage` with the appropriate action.
6. Subscribe to `TOPIC_STATUS` and update state + re-render on every message.
7. Add the CSS spin animation for the `Updating` state.
8. Wire `widget_plugin!` in `src/lib.rs`.
9. Add an integration test that verifies the widget accepts status messages and renders correctly.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The widget displays the correct icon and text for each `UpdateState`.
- Clicking the widget in `UpToDate` state sends a `CheckNow` command.
- Clicking the widget in `UpdatesAvailable` state sends a `StartUpdate` command.
- Clicking the widget in `Updating` state is disabled.
- The icon spins during the `Updating` state.
- No `unwrap`, `expect`, or `panic` remains in the new code.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/update-notifier` and `services/update-notifier` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `update_notifier` in `config.toml`.
4. Add a sample widget configuration for the update notifier widget.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The update notifier widget receives messages and renders correctly.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that `TOPIC_STATUS` appears on the message broker.
2. Verify the widget displays the correct initial state.
3. Verify the widget updates when the service broadcasts a new status.
4. Verify clicking the widget in `UpToDate` state triggers a check.
5. Verify clicking the widget in `UpdatesAvailable` state triggers an update.
6. Verify the widget shows the spinning icon during updates.
7. Verify the widget returns to `UpToDate` or `UpdatesAvailable` after the update completes.
8. Run `cargo test` for all three crates.
9. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- The widget renders correctly for all update states.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.

---

### Summary of Order

```
Phase 1: model/update-notifier
    |
    v
Phase 2: services/update-notifier
    |
    v
Phase 3: plugins/update-notifier
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

### Rationale

- **Model first:** Message formats, enums, and icon mappings must exist before the service or widget can use them.
- **Service second:** The widget needs a running publisher to test against. CLI command execution and provider abstraction are the core logic.
- **Widget third:** The display widget depends on the service's status topic.
- **Wiring fourth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 10. Technical Notes

- **CLI over library bindings:** There is no cross-distribution Rust library for package management. Using CLI commands (`checkupdates`, `apt-get -s upgrade`)
  is the most portable and maintainable approach. Each provider counts the output lines to determine the number of available updates.
- **Provider abstraction:** The `UpdateProvider` trait allows future providers for other distributions (Fedora `dnf check-update`, openSUSE
  `zypper list-updates`, etc.) without changing the service or widget code.
- **Sudo requirement:** The `start_update` method requires root privileges. The service spawns the update command with `sudo`. The user must have appropriate
  sudoers configuration (e.g., passwordless `pacman -Syu` or `apt-get upgrade`) for a seamless touch experience.
- **No polling in the widget:** The widget updates exclusively through incoming messages. Periodic checking only happens in the service.
- **GTK widget ownership:** GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget
  references are captured in `glib::clone!` closures or `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.
- **CSS spin animation:** The rotating icon uses a CSS keyframe animation applied via a style class. This is a lightweight approach that does not require
  JavaScript or manual frame updates.
- **Error handling:** Provider errors are logged via `tracing::error!` and do not crash the service. The service continues to operate and will retry on the next
  check interval.
- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in `model/power`, `model/notifications`, `model/audio`, and `model/network`.

---

## 11. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/update-notifier`, `services/update-notifier`, and `plugins/update-notifier`.
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
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `tokio` and `tracing`; the widget uses `gtk4` and `glib`.

---

*End of document.*
