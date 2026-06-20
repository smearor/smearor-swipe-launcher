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
      ctl.rs          # All ctl command message types
      dispatch.rs     # All dispatch message types
      shared.rs       # Shared enums and identifiers

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

### 3.1 Shared Enums & Identifiers (`messages/shared.rs`)

These mirror the supporting types from `hyprland::dispatch` and `hyprland::ctl`.

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

### 3.2 CTL Command Messages (`messages/ctl.rs`)

One message struct per `hyprland::ctl` sub-module command. Topic: `service.hyprland.ctl`.

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

### 3.3 Dispatch Messages (`messages/dispatch.rs`)

One message struct per `DispatchType` variant. Topic: `service.hyprland.dispatch`.

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

### 3.4 Unified Dispatch Action Enum

To allow a single message handler to route all dispatch commands, a unified enum wraps every dispatch message.

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
use smearor_swipe_launcher_plugin_api::*;
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

```rust
impl HyprlandService {
    pub(crate) fn new(
        config: PluginConfig,
        core_context: Option<FfiCoreContext>,
    ) -> Result<Self, PluginConstructionErrorWrapper> {
        let (command_sender, mut command_receiver) =
            tokio::sync::mpsc::unbounded_channel::<HyprlandCommand>();

        let service = HyprlandService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
        };

        service.executor.spawn("hyprland_service", move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
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
                        // ... etc
                    }
                }
                Ok(())
            })
        })?;

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

// ... additional MessageHandler impls for each ctl command type

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
                id if id == FfiEnvelopePayload::<PluginLoadCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<PluginLoadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginUnloadCommandMessage>::TYPE_ID => {
                    MessageHandler::<FfiEnvelopePayload<PluginUnloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                // ... route all other ctl message type_ids
                _ => {}
            }
        }
    }
}
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
            hyprland::dispatch::WindowIdentifier::Address(addr.to_string().into())
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
