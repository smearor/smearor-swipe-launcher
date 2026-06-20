# Concept: Hyprland Service Plugin

This concept describes the full implementation of the **Hyprland Service** in the *Smearor Swipe Launcher*. It provides a singleton service that exposes all
`hyprland` crate control commands and dispatchers as typed messages over the FFI-stable plugin message system.

The service is strictly split according to the SOA architecture:

1. **The Hyprland Service Plugin:** A background singleton service (Logic/Controller) that interfaces with the Hyprland compositor via the `hyprland` crate.
2. **Widget Plugins (optional):** Pure UI components that send command messages to the service and optionally subscribe to status broadcasts.

---

## 1. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Hyprland Widget          |                 | Hyprland Service           |
| (e.g., Workspace Switcher)|                | (Central Singleton)        |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Command Message                         |
             |===========================================> |
             |  Topic: "service.hyprland.dispatch"           |
             |  Payload: {                                 |
             |    "action": "Workspace",                   |
             |    "identifier": { "Name": "workspace_2" }|
             |  }                                          |
             |                                             |
             |                                             |  2. hyprland::dispatch::call
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.hyprland.status"
             |                                             |     Payload: { ... }
```

---

## 2. Crate Structure

Following the workspace conventions (`AGENTS.md`), the system is split into a shared model crate and a service crate.

```
model/hyprland/
  Cargo.toml
  src/
    lib.rs
    messages/
      mod.rs
      command/
        mod.rs
        kill.rs                  # KillCommandMessage
        notify.rs                # NotifyCommandMessage
        output_create.rs         # OutputCreateCommandMessage
        output_remove.rs         # OutputRemoveCommandMessage
        plugin_load.rs           # PluginLoadCommandMessage
        plugin_unload.rs         # PluginUnloadCommandMessage
        reload.rs                # ReloadCommandMessage
        set_cursor.rs            # SetCursorCommandMessage
        set_error.rs             # SetErrorCommandMessage
        set_prop.rs              # SetPropCommandMessage
        switch_xkb_layout.rs     # SwitchXkbLayoutCommandMessage
      dispatch/
        mod.rs
        custom.rs                # CustomDispatchMessage
        set_cursor.rs            # SetCursorDispatchMessage
        exec.rs                  # ExecDispatchMessage
        pass.rs                  # PassDispatchMessage
        global.rs                # GlobalDispatchMessage
        kill_active_window.rs    # KillActiveWindowDispatchMessage
        close_window.rs          # CloseWindowDispatchMessage
        workspace.rs             # WorkspaceDispatchMessage
        move_to_workspace.rs     # MoveToWorkspaceDispatchMessage
        ...                      # One file per remaining dispatch type
      shared/
        mod.rs
        direction.rs             # HyprlandDirection
        corner.rs                # HyprlandCorner
        fullscreen_type.rs       # HyprlandFullscreenType
        cycle_direction.rs       # HyprlandCycleDirection
        window_switch_direction.rs # HyprlandWindowSwitchDirection
        lock_type.rs             # HyprlandLockType
        swap_with_master_param.rs # HyprlandSwapWithMasterParam
        focus_master_param.rs    # HyprlandFocusMasterParam
        workspace_options.rs     # HyprlandWorkspaceOptions
        position.rs              # HyprlandPosition
        notify_icon.rs           # HyprlandNotifyIcon
        output_backend.rs        # HyprlandOutputBackend
        window_identifier.rs       # HyprlandWindowIdentifier
        workspace_identifier.rs    # HyprlandWorkspaceIdentifier
        workspace_identifier_with_special.rs # HyprlandWorkspaceIdentifierWithSpecial
        monitor_identifier.rs    # HyprlandMonitorIdentifier
        window_move.rs           # HyprlandWindowMove
        switch_xkb_layout_cmd.rs # HyprlandSwitchXkbLayoutCmd
        prop_type.rs             # HyprlandPropType
        color.rs                 # HyprlandColor
      dispatch_action.rs         # HyprlandDispatchAction
      dispatch_message.rs        # HyprlandDispatchMessage

services/hyprland/
  Cargo.toml
  src/
    lib.rs            # service_plugin!(HyprlandService)
    service.rs        # HyprlandService struct and trait impls
    config.rs         # Service configuration parsing
```

---

## 3. Model Crate: Message Types

All FFI-relevant types carry `#[stabby::stabby]`. Each message type implements `TypedMessage`, `MessageTopic`, and has a `Stabby` counterpart with `From`/`Into`
conversions.

---

### 3.1 Shared Enums & Identifiers (`messages/shared/`)

Each shared enum or struct lives in its own file under `messages/shared/`, following `AGENTS.md`. The `messages/shared/mod.rs` re-exports all of them.

| Rust Type                                | `#[stabby::stabby]` | Source                                                      |
|------------------------------------------|---------------------|-------------------------------------------------------------|
| `HyprlandDirection`                      | yes                 | `hyprland::dispatch::Direction`                             |
| `HyprlandCorner`                         | yes                 | `hyprland::dispatch::Corner`                                |
| `HyprlandFullscreenType`                 | yes                 | `hyprland::dispatch::FullscreenType`                        |
| `HyprlandCycleDirection`                 | yes                 | `hyprland::dispatch::CycleDirection`                        |
| `HyprlandWindowSwitchDirection`          | yes                 | `hyprland::dispatch::WindowSwitchDirection`                 |
| `HyprlandLockType`                       | yes                 | `hyprland::dispatch::LockType`                              |
| `HyprlandSwapWithMasterParam`            | yes                 | `hyprland::dispatch::SwapWithMasterParam`                   |
| `HyprlandFocusMasterParam`               | yes                 | `hyprland::dispatch::FocusMasterParam`                      |
| `HyprlandWorkspaceOptions`               | yes                 | `hyprland::dispatch::WorkspaceOptions`                      |
| `HyprlandPosition`                       | yes                 | `hyprland::dispatch::Position`                              |
| `HyprlandNotifyIcon`                     | yes                 | `hyprland::ctl::notify::Icon`                               |
| `HyprlandOutputBackend`                  | yes                 | `hyprland::ctl::output::OutputBackends`                     |
| `HyprlandWindowIdentifier`               | yes                 | `hyprland::dispatch::WindowIdentifier`                      |
| `HyprlandWorkspaceIdentifier`            | yes                 | `hyprland::dispatch::WorkspaceIdentifier`                   |
| `HyprlandWorkspaceIdentifierWithSpecial` | yes                 | `hyprland::dispatch::WorkspaceIdentifierWithSpecial`        |
| `HyprlandMonitorIdentifier`              | yes                 | `hyprland::dispatch::MonitorIdentifier`                     |
| `HyprlandWindowMove`                     | yes                 | `hyprland::dispatch::WindowMove`                            |
| `HyprlandSwitchXkbLayoutCmd`             | yes                 | `hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes` |
| `HyprlandPropType`                       | yes                 | `hyprland::ctl::set_prop::PropType`                         |
| `HyprlandColor`                          | yes                 | `hyprland::ctl::Color`                                      |

---

### 3.2 CTL Command Messages (`messages/command/`)

One message struct per `hyprland::ctl` sub-module command, each in its own file under `messages/command/`. Topic: `service.hyprland.ctl`.

| Message Type                    | Topic                  | Maps To                                  |
|---------------------------------|------------------------|------------------------------------------|
| `KillCommandMessage`            | `service.hyprland.ctl` | `hyprland::ctl::kill::call`              |
| `NotifyCommandMessage`          | `service.hyprland.ctl` | `hyprland::ctl::notify::call`            |
| `OutputCreateCommandMessage`    | `service.hyprland.ctl` | `hyprland::ctl::output::create`          |
| `OutputRemoveCommandMessage`    | `service.hyprland.ctl` | `hyprland::ctl::output::remove`          |
| `PluginLoadCommandMessage`      | `service.hyprland.ctl` | `hyprland::ctl::plugin::load`            |
| `PluginUnloadCommandMessage`    | `service.hyprland.ctl` | `hyprland::ctl::plugin::unload`          |
| `ReloadCommandMessage`          | `service.hyprland.ctl` | `hyprland::ctl::reload::call`            |
| `SetCursorCommandMessage`       | `service.hyprland.ctl` | `hyprland::ctl::set_cursor::call`        |
| `SetErrorCommandMessage`        | `service.hyprland.ctl` | `hyprland::ctl::set_error::call`         |
| `SetPropCommandMessage`         | `service.hyprland.ctl` | `hyprland::ctl::set_prop::call`          |
| `SwitchXkbLayoutCommandMessage` | `service.hyprland.ctl` | `hyprland::ctl::switch_xkb_layout::call` |

Each struct carries `#[stabby::stabby(no_opt)]` and contains the fields required by the corresponding `hyprland` function.

---

### 3.3 Dispatch Messages (`messages/dispatch/`)

One message struct per `DispatchType` variant, each in its own file under `messages/dispatch/`. Topic: `service.hyprland.dispatch`.

| Message Type                                        | Maps To `DispatchType`                                                      |
|-----------------------------------------------------|-----------------------------------------------------------------------------|
| `CustomDispatchMessage`                             | `Custom(&str, &str)`                                                        |
| `SetCursorDispatchMessage`                          | `SetCursor(&str, u16)`                                                      |
| `ExecDispatchMessage`                               | `Exec(&str)`                                                                |
| `PassDispatchMessage`                               | `Pass(WindowIdentifier)`                                                    |
| `GlobalDispatchMessage`                             | `Global(&str)`                                                              |
| `KillActiveWindowDispatchMessage`                   | `KillActiveWindow`                                                          |
| `CloseWindowDispatchMessage`                        | `CloseWindow(WindowIdentifier)`                                             |
| `WorkspaceDispatchMessage`                          | `Workspace(WorkspaceIdentifierWithSpecial)`                                 |
| `MoveToWorkspaceDispatchMessage`                    | `MoveToWorkspace(WorkspaceIdentifierWithSpecial, Option<WindowIdentifier>)` |
| `MoveToWorkspaceSilentDispatchMessage`              | `MoveToWorkspaceSilent(...)`                                                |
| `MoveFocusedWindowToWorkspaceDispatchMessage`       | `MoveFocusedWindowToWorkspace(WorkspaceIdentifier)`                         |
| `MoveFocusedWindowToWorkspaceSilentDispatchMessage` | `MoveFocusedWindowToWorkspaceSilent(...)`                                   |
| `ToggleFloatingDispatchMessage`                     | `ToggleFloating(Option<WindowIdentifier>)`                                  |
| `ToggleFullscreenDispatchMessage`                   | `ToggleFullscreen(FullscreenType)`                                          |
| `ToggleFakeFullscreenDispatchMessage`               | `ToggleFakeFullscreen`                                                      |
| `ToggleDpmsDispatchMessage`                         | `ToggleDPMS(bool, Option<&str>)`                                            |
| `TogglePseudoDispatchMessage`                       | `TogglePseudo`                                                              |
| `TogglePinDispatchMessage`                          | `TogglePin`                                                                 |
| `MoveFocusDispatchMessage`                          | `MoveFocus(Direction)`                                                      |
| `MoveWindowDispatchMessage`                         | `MoveWindow(WindowMove)`                                                    |
| `CenterWindowDispatchMessage`                       | `CenterWindow`                                                              |
| `ResizeActiveDispatchMessage`                       | `ResizeActive(Position)`                                                    |
| `MoveActiveDispatchMessage`                         | `MoveActive(Position)`                                                      |
| `ResizeWindowPixelDispatchMessage`                  | `ResizeWindowPixel(Position, WindowIdentifier)`                             |
| `MoveWindowPixelDispatchMessage`                    | `MoveWindowPixel(Position, WindowIdentifier)`                               |
| `CycleWindowDispatchMessage`                        | `CycleWindow(CycleDirection)`                                               |
| `SwapWindowDispatchMessage`                         | `SwapWindow(CycleDirection)`                                                |
| `FocusWindowDispatchMessage`                        | `FocusWindow(WindowIdentifier)`                                             |
| `FocusMonitorDispatchMessage`                       | `FocusMonitor(MonitorIdentifier)`                                           |
| `ChangeSplitRatioDispatchMessage`                   | `ChangeSplitRatio(f32)`                                                     |
| `ToggleOpaqueDispatchMessage`                       | `ToggleOpaque`                                                              |
| `MoveCursorToCornerDispatchMessage`                 | `MoveCursorToCorner(Corner)`                                                |
| `MoveCursorDispatchMessage`                         | `MoveCursor(i64, i64)`                                                      |
| `WorkspaceOptionDispatchMessage`                    | `WorkspaceOption(WorkspaceOptions)`                                         |
| `RenameWorkspaceDispatchMessage`                    | `RenameWorkspace(WorkspaceId, Option<&str>)`                                |
| `ExitDispatchMessage`                               | `Exit`                                                                      |
| `ForceRendererReloadDispatchMessage`                | `ForceRendererReload`                                                       |
| `MoveCurrentWorkspaceToMonitorDispatchMessage`      | `MoveCurrentWorkspaceToMonitor(MonitorIdentifier)`                          |
| `MoveWorkspaceToMonitorDispatchMessage`             | `MoveWorkspaceToMonitor(WorkspaceIdentifier, MonitorIdentifier)`            |
| `SwapActiveWorkspacesDispatchMessage`               | `SwapActiveWorkspaces(MonitorIdentifier, MonitorIdentifier)`                |
| `BringActiveToTopDispatchMessage`                   | `BringActiveToTop`                                                          |
| `ToggleSpecialWorkspaceDispatchMessage`             | `ToggleSpecialWorkspace(Option<String>)`                                    |
| `FocusUrgentOrLastDispatchMessage`                  | `FocusUrgentOrLast`                                                         |
| `FocusCurrentOrLastDispatchMessage`                 | `FocusCurrentOrLast`                                                        |
| `ToggleSplitDispatchMessage`                        | `ToggleSplit`                                                               |
| `SwapWithMasterDispatchMessage`                     | `SwapWithMaster(SwapWithMasterParam)`                                       |
| `FocusMasterDispatchMessage`                        | `FocusMaster(FocusMasterParam)`                                             |
| `AddMasterDispatchMessage`                          | `AddMaster`                                                                 |
| `RemoveMasterDispatchMessage`                       | `RemoveMaster`                                                              |
| `OrientationLeftDispatchMessage`                    | `OrientationLeft`                                                           |
| `OrientationRightDispatchMessage`                   | `OrientationRight`                                                          |
| `OrientationTopDispatchMessage`                     | `OrientationTop`                                                            |
| `OrientationBottomDispatchMessage`                  | `OrientationBottom`                                                         |
| `OrientationCenterDispatchMessage`                  | `OrientationCenter`                                                         |
| `OrientationNextDispatchMessage`                    | `OrientationNext`                                                           |
| `OrientationPrevDispatchMessage`                    | `OrientationPrev`                                                           |
| `ToggleGroupDispatchMessage`                        | `ToggleGroup`                                                               |
| `ChangeGroupActiveDispatchMessage`                  | `ChangeGroupActive(WindowSwitchDirection)`                                  |
| `LockGroupsDispatchMessage`                         | `LockGroups(LockType)`                                                      |
| `MoveIntoGroupDispatchMessage`                      | `MoveIntoGroup(Direction)`                                                  |
| `MoveOutOfGroupDispatchMessage`                     | `MoveOutOfGroup`                                                            |

---

### 3.4 Unified Dispatch Action Enum (`messages/dispatch_action.rs`)

To allow a single message handler to route all dispatch commands, a unified enum wraps every dispatch message. It lives in its own file, while the envelope
struct lives in `messages/dispatch_message.rs`.

#### Model Crate `lib.rs`

```rust
pub mod messages;

pub use messages::command::*;
pub use messages::dispatch::*;
pub use messages::dispatch_action::HyprlandDispatchAction;
pub use messages::dispatch_message::HyprlandDispatchMessage;
pub use messages::shared::*;
```

#### `messages/mod.rs`

```rust
pub mod command;
pub mod dispatch;
pub mod dispatch_action;
pub mod dispatch_message;
pub mod shared;
```

```rust
/// Unified enum for all Hyprland dispatch commands.
#[stabby::stabby]
#[derive(Clone, Debug)]
pub enum HyprlandDispatchAction {
    Custom(CustomDispatchMessage),
    SetCursor(SetCursorDispatchMessage),
    Exec(ExecDispatchMessage),
    Pass(PassDispatchMessage),
    Global(GlobalDispatchMessage),
    KillActiveWindow(KillActiveWindowDispatchMessage),
    CloseWindow(CloseWindowDispatchMessage),
    Workspace(WorkspaceDispatchMessage),
    MoveToWorkspace(MoveToWorkspaceDispatchMessage),
    MoveToWorkspaceSilent(MoveToWorkspaceSilentDispatchMessage),
    MoveFocusedWindowToWorkspace(MoveFocusedWindowToWorkspaceDispatchMessage),
    MoveFocusedWindowToWorkspaceSilent(MoveFocusedWindowToWorkspaceSilentDispatchMessage),
    ToggleFloating(ToggleFloatingDispatchMessage),
    ToggleFullscreen(ToggleFullscreenDispatchMessage),
    ToggleFakeFullscreen(ToggleFakeFullscreenDispatchMessage),
    ToggleDpms(ToggleDpmsDispatchMessage),
    TogglePseudo(TogglePseudoDispatchMessage),
    TogglePin(TogglePinDispatchMessage),
    MoveFocus(MoveFocusDispatchMessage),
    MoveWindow(MoveWindowDispatchMessage),
    CenterWindow(CenterWindowDispatchMessage),
    ResizeActive(ResizeActiveDispatchMessage),
    MoveActive(MoveActiveDispatchMessage),
    ResizeWindowPixel(ResizeWindowPixelDispatchMessage),
    MoveWindowPixel(MoveWindowPixelDispatchMessage),
    CycleWindow(CycleWindowDispatchMessage),
    SwapWindow(SwapWindowDispatchMessage),
    FocusWindow(FocusWindowDispatchMessage),
    FocusMonitor(FocusMonitorDispatchMessage),
    ChangeSplitRatio(ChangeSplitRatioDispatchMessage),
    ToggleOpaque(ToggleOpaqueDispatchMessage),
    MoveCursorToCorner(MoveCursorToCornerDispatchMessage),
    MoveCursor(MoveCursorDispatchMessage),
    WorkspaceOption(WorkspaceOptionDispatchMessage),
    RenameWorkspace(RenameWorkspaceDispatchMessage),
    Exit(ExitDispatchMessage),
    ForceRendererReload(ForceRendererReloadDispatchMessage),
    MoveCurrentWorkspaceToMonitor(MoveCurrentWorkspaceToMonitorDispatchMessage),
    MoveWorkspaceToMonitor(MoveWorkspaceToMonitorDispatchMessage),
    SwapActiveWorkspaces(SwapActiveWorkspacesDispatchMessage),
    BringActiveToTop(BringActiveToTopDispatchMessage),
    ToggleSpecialWorkspace(ToggleSpecialWorkspaceDispatchMessage),
    FocusUrgentOrLast(FocusUrgentOrLastDispatchMessage),
    FocusCurrentOrLast(FocusCurrentOrLastDispatchMessage),
    ToggleSplit(ToggleSplitDispatchMessage),
    SwapWithMaster(SwapWithMasterDispatchMessage),
    FocusMaster(FocusMasterDispatchMessage),
    AddMaster(AddMasterDispatchMessage),
    RemoveMaster(RemoveMasterDispatchMessage),
    OrientationLeft(OrientationLeftDispatchMessage),
    OrientationRight(OrientationRightDispatchMessage),
    OrientationTop(OrientationTopDispatchMessage),
    OrientationBottom(OrientationBottomDispatchMessage),
    OrientationCenter(OrientationCenterDispatchMessage),
    OrientationNext(OrientationNextDispatchMessage),
    OrientationPrev(OrientationPrevDispatchMessage),
    ToggleGroup(ToggleGroupDispatchMessage),
    ChangeGroupActive(ChangeGroupActiveDispatchMessage),
    LockGroups(LockGroupsDispatchMessage),
    MoveIntoGroup(MoveIntoGroupDispatchMessage),
    MoveOutOfGroup(MoveOutOfGroupDispatchMessage),
}

/// The main dispatch envelope sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug)]
pub struct HyprlandDispatchMessage {
    pub action: HyprlandDispatchAction,
}
```

---

## 4. Service Implementation (`services/hyprland`)

### 4.1 Service Struct

```rust
use hyprland::dispatch::Dispatch;
use hyprland::ctl::*;
use smearor_hyprland_model::*;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use tokio::sync::mpsc;

pub struct HyprlandService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<HyprlandCommand>,
}

/// Internal union of all command types the service handles.
pub enum HyprlandCommand {
    Dispatch(HyprlandDispatchMessage),
    CtlKill(KillCommandMessage),
    CtlNotify(NotifyCommandMessage),
    CtlOutputCreate(OutputCreateCommandMessage),
    CtlOutputRemove(OutputRemoveCommandMessage),
    CtlPluginLoad(PluginLoadCommandMessage),
    CtlPluginUnload(PluginUnloadCommandMessage),
    CtlReload(ReloadCommandMessage),
    CtlSetCursor(SetCursorCommandMessage),
    CtlSetError(SetErrorCommandMessage),
    CtlSetProp(SetPropCommandMessage),
    CtlSwitchXkbLayout(SwitchXkbLayoutCommandMessage),
}
```

### 4.2 Service Construction

The service constructor follows the pattern from the existing `app-launcher` service: it spawns a dedicated OS thread with a single-threaded Tokio runtime for
async command handling.

```rust
impl HyprlandService {
    pub(crate) fn new(
        config: PluginConfig,
        core_context: Option<FfiCoreContext>,
    ) -> Result<Self, PluginConstructionErrorWrapper> {
        let _service_config: HyprlandServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(
                PluginConstructionError::FailedToParseWidgetConfig,
                e.to_string().into(),
            ))?;

        let (command_sender, mut command_receiver) =
            tokio::sync::mpsc::unbounded_channel::<HyprlandCommand>();

        let service = HyprlandService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
        };

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(error) => {
                    tracing::error!("Hyprland Service: failed to create tokio runtime: {error}");
                    return;
                }
            };

            rt.block_on(async move {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        HyprlandCommand::Dispatch(message) => {
                            handle_dispatch(message).await;
                        }
                        HyprlandCommand::CtlKill(message) => {
                            handle_ctl_kill(message).await;
                        }
                        HyprlandCommand::CtlNotify(message) => {
                            handle_ctl_notify(message).await;
                        }
                        HyprlandCommand::CtlOutputCreate(message) => {
                            handle_ctl_output_create(message).await;
                        }
                        HyprlandCommand::CtlOutputRemove(message) => {
                            handle_ctl_output_remove(message).await;
                        }
                        HyprlandCommand::CtlPluginLoad(message) => {
                            handle_ctl_plugin_load(message).await;
                        }
                        HyprlandCommand::CtlPluginUnload(message) => {
                            handle_ctl_plugin_unload(message).await;
                        }
                        HyprlandCommand::CtlReload(message) => {
                            handle_ctl_reload(message).await;
                        }
                        HyprlandCommand::CtlSetCursor(message) => {
                            handle_ctl_set_cursor(message).await;
                        }
                        HyprlandCommand::CtlSetError(message) => {
                            handle_ctl_set_error(message).await;
                        }
                        HyprlandCommand::CtlSetProp(message) => {
                            handle_ctl_set_prop(message).await;
                        }
                        HyprlandCommand::CtlSwitchXkbLayout(message) => {
                            handle_ctl_switch_xkb_layout(message).await;
                        }
                    }
                }
            });
        });

        Ok(service)
    }
}
```

### 4.3 Dispatch Handler Example

```rust
async fn handle_dispatch(message: HyprlandDispatchMessage) {
    use hyprland::dispatch::*;

    let result = match message.action {
        HyprlandDispatchAction::Exec(payload) => {
            DispatchType::Exec(&payload.command).dispatch().await
        }
        HyprlandDispatchAction::Workspace(payload) => {
            let identifier = convert_workspace_identifier(payload.identifier);
            DispatchType::Workspace(identifier).dispatch().await
        }
        HyprlandDispatchAction::MoveFocus(payload) => {
            let direction = convert_direction(payload.direction);
            DispatchType::MoveFocus(direction).dispatch().await
        }
        HyprlandDispatchAction::KillActiveWindow(_) => {
            DispatchType::KillActiveWindow.dispatch().await
        }
        // ... map all other variants
    };

    if let Err(error) = result {
        tracing::error!("Hyprland dispatch failed: {error}");
    }
}
```

### 4.4 CTL Handler Example

```rust
async fn handle_ctl_notify(message: NotifyCommandMessage) {
    let icon = convert_icon(message.icon);
    let color = convert_color(message.color);
    if let Err(error) = hyprland::ctl::notify::call(icon, message.time_ms, color, &message.message).await {
        tracing::error!("Hyprland notify failed: {error}");
    }
}

async fn handle_ctl_reload(_message: ReloadCommandMessage) {
    if let Err(error) = hyprland::ctl::reload::call().await {
        tracing::error!("Hyprland reload failed: {error}");
    }
}
```

### 4.5 Required Trait Implementations

```rust
impl MessageHandler<FfiEnvelopePayload<HyprlandDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<KillCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<KillCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlKill(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<NotifyCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<NotifyCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlNotify(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<OutputCreateCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<OutputCreateCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlOutputCreate(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<OutputRemoveCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<OutputRemoveCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlOutputRemove(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<PluginLoadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<PluginLoadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlPluginLoad(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<PluginUnloadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<PluginUnloadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlPluginUnload(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<ReloadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ReloadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlReload(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<SetCursorCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetCursorCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetCursor(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<SetErrorCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetErrorCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetError(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<SetPropCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetPropCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetProp(message.into_inner()));
    }
}

impl MessageHandler<FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSwitchXkbLayout(message.into_inner()));
    }
}

impl MessageBroadcaster for HyprlandService {}
impl PluginMetaGetter for HyprlandService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}
impl AsRef<Option<FfiCoreContext>> for HyprlandService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}
impl Service for HyprlandService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            match envelope.type_id {
                id if id == FfiEnvelopePayload::<HyprlandDispatchMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<HyprlandDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<KillCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<KillCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<NotifyCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<NotifyCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<OutputCreateCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<OutputCreateCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<OutputRemoveCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<OutputRemoveCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginLoadCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<PluginLoadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginUnloadCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<PluginUnloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ReloadCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<ReloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetCursorCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<SetCursorCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetErrorCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<SetErrorCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetPropCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<SetPropCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SwitchXkbLayoutCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>>::handle_envelope_message(self, envelope);
                }
                _ => {}
            }
        }
    }
}
```

### 4.6 Service Crate `lib.rs`

```rust
pub mod config;
pub mod service;

use crate::service::HyprlandService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(HyprlandService);
```

---

## 5. Configuration

### 5.1 Service Config (`config.rs`)

```rust
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct HyprlandServiceConfig {
    /// Optional path override for the Hyprland socket.
    pub socket_path: Option<String>,
}

impl HyprlandServiceConfig {
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }
}
```

### 5.2 Config TOML

```toml
[services.hyprland]
path = "target/debug/libhyprland_service.so"
```

### 5.3 Workspace Cargo.toml Integration

Add the two new crates to the workspace root `Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing members
    "model/hyprland",
    "services/hyprland",
]
```

Add `hyprland` to the shared workspace dependencies so the version is managed centrally:

```toml
[workspace.dependencies]
hyprland = "0.3.13"
```

The service crate then references it via `workspace = true`:

```toml
[dependencies]
hyprland = { workspace = true }
```

### 5.4 Minimum Viable Scope for the First Step

The first implementation goal is a simple workspace switch from a button widget. Therefore, the initial service only needs to handle a small subset of dispatch
commands. The recommended first slice is:

- `WorkspaceDispatchMessage` (workspace switch)
- `ExecDispatchMessage` (fallback for arbitrary commands)
- `KillActiveWindowDispatchMessage` (close active window)
- `MoveFocusDispatchMessage` (focus change)
- `ToggleFullscreenDispatchMessage` (fullscreen toggle)

All other 57 dispatch types and all 11 CTL commands can be left as `todo!()` or omitted in the first iteration. They can be added incrementally once the basic
command flow is proven.

---

## 6. Conversion Helpers

Because the `hyprland` crate types use `&str` lifetime parameters and our stabby model uses owned strings, the service must provide `From`/`Into` conversion
helpers that map model enums to their `hyprland` equivalents.

### 6.1 Example Conversions

```rust
fn convert_direction(direction: HyprlandDirection) -> hyprland::dispatch::Direction {
    match direction {
        HyprlandDirection::Up => hyprland::dispatch::Direction::Up,
        HyprlandDirection::Down => hyprland::dispatch::Direction::Down,
        HyprlandDirection::Left => hyprland::dispatch::Direction::Left,
        HyprlandDirection::Right => hyprland::dispatch::Direction::Right,
    }
}

fn convert_window_identifier(id: HyprlandWindowIdentifier) -> hyprland::dispatch::WindowIdentifier<'static> {
    match id {
        HyprlandWindowIdentifier::Address(addr) => {
            hyprland::dispatch::WindowIdentifier::Address(hyprland::shared::Address::new(addr.to_string()))
        }
        HyprlandWindowIdentifier::ClassRegularExpression(regex) => {
            hyprland::dispatch::WindowIdentifier::ClassRegularExpression(regex.leak())
        }
        HyprlandWindowIdentifier::Title(title) => {
            hyprland::dispatch::WindowIdentifier::Title(title.leak())
        }
        HyprlandWindowIdentifier::ProcessId(pid) => {
            hyprland::dispatch::WindowIdentifier::ProcessId(pid)
        }
    }
}

fn convert_workspace_identifier(ws: HyprlandWorkspaceIdentifierWithSpecial) -> hyprland::dispatch::WorkspaceIdentifierWithSpecial<'static> {
    match ws {
        HyprlandWorkspaceIdentifierWithSpecial::Id(id) => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(id)
        }
        HyprlandWorkspaceIdentifierWithSpecial::Relative(offset) => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Relative(offset)
        }
        HyprlandWorkspaceIdentifierWithSpecial::RelativeMonitor(offset) => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitor(offset)
        }
        HyprlandWorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(offset) => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(offset)
        }
        HyprlandWorkspaceIdentifierWithSpecial::RelativeOpen(offset) => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeOpen(offset)
        }
        HyprlandWorkspaceIdentifierWithSpecial::Previous => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Previous
        }
        HyprlandWorkspaceIdentifierWithSpecial::Empty => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Empty
        }
        HyprlandWorkspaceIdentifierWithSpecial::Name(name) => {
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(name.leak())
        }
        HyprlandWorkspaceIdentifierWithSpecial::Special(name) => {
            let opt: Option<String> = name.into();
            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(opt.as_deref())
        }
    }
}

fn convert_monitor_identifier(mon: HyprlandMonitorIdentifier) -> hyprland::dispatch::MonitorIdentifier<'static> {
    match mon {
        HyprlandMonitorIdentifier::Direction(dir) => {
            hyprland::dispatch::MonitorIdentifier::Direction(convert_direction(dir))
        }
        HyprlandMonitorIdentifier::Id(id) => {
            hyprland::dispatch::MonitorIdentifier::Id(id)
        }
        HyprlandMonitorIdentifier::Name(name) => {
            hyprland::dispatch::MonitorIdentifier::Name(name.leak())
        }
        HyprlandMonitorIdentifier::Current => {
            hyprland::dispatch::MonitorIdentifier::Current
        }
        HyprlandMonitorIdentifier::Relative(offset) => {
            hyprland::dispatch::MonitorIdentifier::Relative(offset)
        }
    }
}

fn convert_position(pos: HyprlandPosition) -> hyprland::dispatch::Position {
    match pos {
        HyprlandPosition::Delta(x, y) => hyprland::dispatch::Position::Delta(x, y),
        HyprlandPosition::Exact(x, y) => hyprland::dispatch::Position::Exact(x, y),
    }
}

fn convert_fullscreen_type(ft: HyprlandFullscreenType) -> hyprland::dispatch::FullscreenType {
    match ft {
        HyprlandFullscreenType::Real => hyprland::dispatch::FullscreenType::Real,
        HyprlandFullscreenType::Maximize => hyprland::dispatch::FullscreenType::Maximize,
        HyprlandFullscreenType::NoParam => hyprland::dispatch::FullscreenType::NoParam,
    }
}

fn convert_cycle_direction(cd: HyprlandCycleDirection) -> hyprland::dispatch::CycleDirection {
    match cd {
        HyprlandCycleDirection::Next => hyprland::dispatch::CycleDirection::Next,
        HyprlandCycleDirection::Previous => hyprland::dispatch::CycleDirection::Previous,
    }
}

fn convert_window_move(movement: HyprlandWindowMove) -> hyprland::dispatch::WindowMove<'static> {
    match movement {
        HyprlandWindowMove::Monitor(mon) => {
            hyprland::dispatch::WindowMove::Monitor(convert_monitor_identifier(mon))
        }
        HyprlandWindowMove::Direction(dir) => {
            hyprland::dispatch::WindowMove::Direction(convert_direction(dir))
        }
    }
}

fn convert_workspace_options(opt: HyprlandWorkspaceOptions) -> hyprland::dispatch::WorkspaceOptions {
    match opt {
        HyprlandWorkspaceOptions::AllPseudo => hyprland::dispatch::WorkspaceOptions::AllPseudo,
        HyprlandWorkspaceOptions::AllFloat => hyprland::dispatch::WorkspaceOptions::AllFloat,
    }
}

fn convert_lock_type(lock: HyprlandLockType) -> hyprland::dispatch::LockType {
    match lock {
        HyprlandLockType::Lock => hyprland::dispatch::LockType::Lock,
        HyprlandLockType::Unlock => hyprland::dispatch::LockType::Unlock,
        HyprlandLockType::ToggleLock => hyprland::dispatch::LockType::ToggleLock,
    }
}

fn convert_swap_with_master(param: HyprlandSwapWithMasterParam) -> hyprland::dispatch::SwapWithMasterParam {
    match param {
        HyprlandSwapWithMasterParam::Master => hyprland::dispatch::SwapWithMasterParam::Master,
        HyprlandSwapWithMasterParam::Child => hyprland::dispatch::SwapWithMasterParam::Child,
        HyprlandSwapWithMasterParam::Auto => hyprland::dispatch::SwapWithMasterParam::Auto,
    }
}

fn convert_focus_master_param(param: HyprlandFocusMasterParam) -> hyprland::dispatch::FocusMasterParam {
    match param {
        HyprlandFocusMasterParam::Master => hyprland::dispatch::FocusMasterParam::Master,
        HyprlandFocusMasterParam::Auto => hyprland::dispatch::FocusMasterParam::Auto,
    }
}

fn convert_window_switch_direction(dir: HyprlandWindowSwitchDirection) -> hyprland::dispatch::WindowSwitchDirection {
    match dir {
        HyprlandWindowSwitchDirection::Back => hyprland::dispatch::WindowSwitchDirection::Back,
        HyprlandWindowSwitchDirection::Forward => hyprland::dispatch::WindowSwitchDirection::Forward,
    }
}

fn convert_corner(corner: HyprlandCorner) -> hyprland::dispatch::Corner {
    match corner {
        HyprlandCorner::TopRight => hyprland::dispatch::Corner::TopRight,
        HyprlandCorner::TopLeft => hyprland::dispatch::Corner::TopLeft,
        HyprlandCorner::BottomRight => hyprland::dispatch::Corner::BottomRight,
        HyprlandCorner::BottomLeft => hyprland::dispatch::Corner::BottomLeft,
    }
}

fn convert_notify_icon(icon: HyprlandNotifyIcon) -> hyprland::ctl::notify::Icon {
    match icon {
        HyprlandNotifyIcon::NoIcon => hyprland::ctl::notify::Icon::NoIcon,
        HyprlandNotifyIcon::Warning => hyprland::ctl::notify::Icon::Warning,
        HyprlandNotifyIcon::Info => hyprland::ctl::notify::Icon::Info,
        HyprlandNotifyIcon::Hint => hyprland::ctl::notify::Icon::Hint,
        HyprlandNotifyIcon::Error => hyprland::ctl::notify::Icon::Error,
        HyprlandNotifyIcon::Confused => hyprland::ctl::notify::Icon::Confused,
    }
}

fn convert_output_backend(backend: HyprlandOutputBackend) -> hyprland::ctl::output::OutputBackends {
    match backend {
        HyprlandOutputBackend::Wayland => hyprland::ctl::output::OutputBackends::Wayland,
        HyprlandOutputBackend::X11 => hyprland::ctl::output::OutputBackends::X11,
        HyprlandOutputBackend::Headless => hyprland::ctl::output::OutputBackends::Headless,
        HyprlandOutputBackend::Auto => hyprland::ctl::output::OutputBackends::Auto,
    }
}

fn convert_color(color: HyprlandColor) -> hyprland::ctl::Color {
    hyprland::ctl::Color {
        red: color.red,
        green: color.green,
        blue: color.blue,
        alpha: color.alpha,
    }
}

fn convert_switch_xkb_layout_cmd(cmd: HyprlandSwitchXkbLayoutCmd) -> hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes {
    match cmd {
        HyprlandSwitchXkbLayoutCmd::Next => hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
        HyprlandSwitchXkbLayoutCmd::Previous => hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Previous,
        HyprlandSwitchXkbLayoutCmd::Id(id) => hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Id(id),
    }
}
```

---

## 7. Dependencies

### 7.1 Model Crate (`model/hyprland/Cargo.toml`)

```toml
[package]
name = "smearor-hyprland-model"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
stabby = { workspace = true }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
```

### 7.2 Service Crate (`services/hyprland/Cargo.toml`)

```toml
[package]
name = "smearor-hyprland-service"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
hyprland = "0.3.13"
tokio = { workspace = true }
tracing = { workspace = true }
smearor-hyprland-model = { path = "../../model/hyprland" }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
```

---

## 8. Widget Example: Workspace Switcher

A minimal widget that sends workspace switch commands:

```rust
// Widget on_message (simplified)
fn on_tap(&self) {
    let message = HyprlandDispatchMessage {
        action: HyprlandDispatchAction::Workspace(WorkspaceDispatchMessage {
            identifier: HyprlandWorkspaceIdentifierWithSpecial::Name(
                self.config.workspace_name.clone().into()
            ),
        }),
    };
    self.publish_message(message);
}
```

---

## 10. Lifetime Handling & Avoiding String Leaks

### 10.1 The Problem

Many `hyprland` crate types carry a lifetime parameter (`DispatchType<'a>`, `WindowIdentifier<'a>`, etc.) and store string slices (`&'a str`). The stabby model
uses owned `stabby::string::String` values. Converting an owned string to a `&'a str` with `'static` lifetime requires either:

- Leaking the string with `String::leak()` (permanent memory leak).
- Keeping the owned string alive in the same scope where the `DispatchType` is used.

### 10.2 Why `leak()` Is Used in the Concept

The helper functions in Section 6 return `hyprland` types with `'static` lifetime:

```rust
fn convert_workspace_identifier(...) -> hyprland::dispatch::WorkspaceIdentifierWithSpecial<'static>
```

Because the returned value must outlive the function, the only way to attach a `&str` to it is `String::leak()`. Every call that carries a `Name`, `Title`,
`ClassRegularExpression`, or `Special` workspace name leaks memory permanently.

### 10.3 The Better Approach: In-Scope Owned Strings

`leak()` is **not required** if the owned string is kept alive in the same scope where the dispatch call happens. The dispatch call is short-lived, so a local
`String` is sufficient:

```rust
async fn handle_exec(payload: ExecDispatchMessage) {
    let command = payload.command.to_string();
    if let Err(error) = Dispatch::call(DispatchType::Exec(&command)).await {
        tracing::error!("Hyprland exec failed: {error}");
    }
}

async fn handle_workspace(payload: WorkspaceDispatchMessage) {
    match payload.identifier {
        HyprlandWorkspaceIdentifierWithSpecial::Name(name) => {
            let name_string = name.to_string();
            let identifier = hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(&name_string);
            if let Err(error) = Dispatch::call(DispatchType::Workspace(identifier)).await {
                tracing::error!("Hyprland workspace failed: {error}");
            }
        }
        HyprlandWorkspaceIdentifierWithSpecial::Id(id) => {
            let identifier = hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(id);
            if let Err(error) = Dispatch::call(DispatchType::Workspace(identifier)).await {
                tracing::error!("Hyprland workspace failed: {error}");
            }
        }
        // ... handle remaining variants without leaking
    }
}
```

### 10.4 Limitations of In-Scope Strings

This approach works only when the `DispatchType` and the owned string live in the same scope. It breaks when helper functions try to return a constructed
`DispatchType<'static>` to a caller, because the borrowed string would drop before the caller uses it.

Therefore, the service should **not** use generic helper functions that return `DispatchType<'static>` or `WindowIdentifier<'static>`. Instead, each handler
should construct the `hyprland` type directly inside its own scope.

### 10.5 Recommended Pattern

Replace the generic `convert_*` helpers with **per-handler conversion logic**:

```rust
async fn handle_close_window(payload: CloseWindowDispatchMessage) {
    match payload.window {
        HyprlandWindowIdentifier::Address(addr) => {
            let address = hyprland::shared::Address::new(addr.to_string());
            let id = hyprland::dispatch::WindowIdentifier::Address(address);
            let _ = Dispatch::call(DispatchType::CloseWindow(id)).await;
        }
        HyprlandWindowIdentifier::ClassRegularExpression(regex) => {
            let regex_string = regex.to_string();
            let id = hyprland::dispatch::WindowIdentifier::ClassRegularExpression(&regex_string);
            let _ = Dispatch::call(DispatchType::CloseWindow(id)).await;
        }
        HyprlandWindowIdentifier::Title(title) => {
            let title_string = title.to_string();
            let id = hyprland::dispatch::WindowIdentifier::Title(&title_string);
            let _ = Dispatch::call(DispatchType::CloseWindow(id)).await;
        }
        HyprlandWindowIdentifier::ProcessId(pid) => {
            let id = hyprland::dispatch::WindowIdentifier::ProcessId(pid);
            let _ = Dispatch::call(DispatchType::CloseWindow(id)).await;
        }
    }
}
```

### 10.6 When `leak()` Is Acceptable

If the helper-function style is strongly preferred for readability, `leak()` is technically safe but should be documented as a deliberate trade-off. It is
acceptable only for commands that are sent very infrequently (e.g., configuration reload, one-time output creation). It must be avoided for high-frequency
commands like workspace switching, window focus, or cursor movement.

### 10.7 Alternative: String-Based Command Builder

As a last resort, the service could bypass the typed `hyprland` API and send raw `hyprctl` strings over the Hyprland socket. This avoids all lifetime issues but
sacrifices compile-time safety and type checking. It is not recommended as the primary implementation strategy.

---

## 9. Summary

This concept defines:

- **One model crate** (`smearor-hyprland-model`) containing:
    - 19 shared stabby enums for identifiers, directions, and options.
    - 11 CTL command message structs.
    - 62 dispatch message structs.
    - 1 unified `HyprlandDispatchAction` enum for routing.

- **One service crate** (`smearor-hyprland-service`) containing:
    - `HyprlandService` singleton using `tokio::sync::mpsc` for async command processing.
    - A single async dispatch loop that maps every model message to the corresponding `hyprland` crate API call.
    - Full trait implementations per `AGENTS.md` (`MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, `Service`).

- **Lifetime handling**: The initial conversion helpers use `String::leak()`, which leaks memory. The recommended approach is to construct `hyprland` types
  inside each handler with locally owned `String` values, keeping the borrow inside the same scope as the dispatch call.
