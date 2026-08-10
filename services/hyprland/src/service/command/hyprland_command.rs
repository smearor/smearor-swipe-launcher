use crate::service::ctl::handle_ctl_keyword_get;
use crate::service::ctl::handle_ctl_keyword_set;
use crate::service::ctl::handle_ctl_kill;
use crate::service::ctl::handle_ctl_notify;
use crate::service::ctl::handle_ctl_output_create;
use crate::service::ctl::handle_ctl_output_remove;
use crate::service::ctl::handle_ctl_plugin_load;
use crate::service::ctl::handle_ctl_plugin_unload;
use crate::service::ctl::handle_ctl_reload;
use crate::service::ctl::handle_ctl_send_shortcut;
use crate::service::ctl::handle_ctl_set_cursor;
use crate::service::ctl::handle_ctl_set_error;
use crate::service::ctl::handle_ctl_set_prop;
use crate::service::ctl::handle_ctl_switch_xkb_layout;
use crate::service::dispatch::handle_create_workspace;
use crate::service::dispatch::handle_dispatch_system;
use crate::service::dispatch::handle_dispatch_toggle;
use crate::service::dispatch::handle_dispatch_window;
use crate::service::dispatch::handle_dispatch_workspace;
use crate::service::dispatch::handle_switch_workspace;
use crate::service::shared_state::HyprlandSharedState;
use crate::service::state::handle_monitors_request;
use crate::service::state::handle_snapshot_request;
use crate::service::state::handle_state_request;
use crate::service::state::handle_version_request;
use crate::service::state::handle_windows_request;
use smearor_hyprland_model::HyprlandSystemDispatchMessage;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::HyprlandWindowDispatchMessage;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::KeywordGetCommandMessage;
use smearor_hyprland_model::KeywordSetCommandMessage;
use smearor_hyprland_model::KillCommandMessage;
use smearor_hyprland_model::NotifyCommandMessage;
use smearor_hyprland_model::OutputCreateCommandMessage;
use smearor_hyprland_model::OutputRemoveCommandMessage;
use smearor_hyprland_model::PluginLoadCommandMessage;
use smearor_hyprland_model::PluginUnloadCommandMessage;
use smearor_hyprland_model::ReloadCommandMessage;
use smearor_hyprland_model::SendShortcutCommandMessage;
use smearor_hyprland_model::SetCursorCommandMessage;
use smearor_hyprland_model::SetErrorCommandMessage;
use smearor_hyprland_model::SetPropCommandMessage;
use smearor_hyprland_model::SwitchXkbLayoutCommandMessage;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use std::sync::Arc;
use std::sync::Mutex;

/// Internal union of all command types the service handles.
pub enum HyprlandCommand {
    WindowDispatch(HyprlandWindowDispatchMessage),
    WorkspaceDispatch(HyprlandWorkspaceDispatchMessage),
    ToggleDispatch(HyprlandToggleDispatchMessage),
    SystemDispatch(HyprlandSystemDispatchMessage),
    SwitchWorkspace(SwitchWorkspaceMessage),
    CreateWorkspace(CreateWorkspaceMessage),
    SnapshotRequest(WorkspaceSnapshotRequestMessage),
    StateRequest,
    WindowsRequest,
    MonitorsRequest,
    VersionRequest,
    CtlKill(KillCommandMessage),
    CtlKeywordGet(KeywordGetCommandMessage),
    CtlKeywordSet(KeywordSetCommandMessage),
    CtlNotify(NotifyCommandMessage),
    CtlOutputCreate(OutputCreateCommandMessage),
    CtlOutputRemove(OutputRemoveCommandMessage),
    CtlPluginLoad(PluginLoadCommandMessage),
    CtlPluginUnload(PluginUnloadCommandMessage),
    CtlReload(ReloadCommandMessage),
    CtlSetCursor(SetCursorCommandMessage),
    CtlSetError(SetErrorCommandMessage),
    CtlSetProp(SetPropCommandMessage),
    CtlSendShortcut(SendShortcutCommandMessage),
    CtlSwitchXkbLayout(SwitchXkbLayoutCommandMessage),
}

impl HyprlandCommand {
    pub(crate) async fn handle(self, broadcaster: &MessageBroadcasterInner, shared_state: &Arc<Mutex<HyprlandSharedState>>) {
        match self {
            HyprlandCommand::WindowDispatch(message) => handle_dispatch_window(message).await,
            HyprlandCommand::WorkspaceDispatch(message) => handle_dispatch_workspace(message).await,
            HyprlandCommand::ToggleDispatch(message) => handle_dispatch_toggle(message).await,
            HyprlandCommand::SystemDispatch(message) => handle_dispatch_system(message).await,
            HyprlandCommand::SwitchWorkspace(message) => handle_switch_workspace(message).await,
            HyprlandCommand::CreateWorkspace(message) => handle_create_workspace(message).await,
            HyprlandCommand::SnapshotRequest(message) => handle_snapshot_request(message, broadcaster, shared_state).await,
            HyprlandCommand::CtlKill(message) => handle_ctl_kill(message).await,
            HyprlandCommand::CtlKeywordGet(message) => handle_ctl_keyword_get(message, broadcaster).await,
            HyprlandCommand::CtlKeywordSet(message) => handle_ctl_keyword_set(message).await,
            HyprlandCommand::CtlNotify(message) => handle_ctl_notify(message).await,
            HyprlandCommand::CtlOutputCreate(message) => handle_ctl_output_create(message).await,
            HyprlandCommand::CtlOutputRemove(message) => handle_ctl_output_remove(message).await,
            HyprlandCommand::CtlPluginLoad(message) => handle_ctl_plugin_load(message).await,
            HyprlandCommand::CtlPluginUnload(message) => handle_ctl_plugin_unload(message).await,
            HyprlandCommand::CtlReload(message) => handle_ctl_reload(message).await,
            HyprlandCommand::CtlSetCursor(message) => handle_ctl_set_cursor(message).await,
            HyprlandCommand::CtlSetError(message) => handle_ctl_set_error(message).await,
            HyprlandCommand::CtlSetProp(message) => handle_ctl_set_prop(message).await,
            HyprlandCommand::CtlSendShortcut(message) => handle_ctl_send_shortcut(message).await,
            HyprlandCommand::CtlSwitchXkbLayout(message) => handle_ctl_switch_xkb_layout(message).await,
            HyprlandCommand::StateRequest => handle_state_request(broadcaster, shared_state).await,
            HyprlandCommand::WindowsRequest => handle_windows_request(shared_state).await,
            HyprlandCommand::MonitorsRequest => handle_monitors_request(shared_state).await,
            HyprlandCommand::VersionRequest => handle_version_request(shared_state).await,
        }
    }
}
