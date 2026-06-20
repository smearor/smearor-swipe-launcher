use crate::messages::dispatch::ExecDispatchMessageStabby;
use crate::messages::dispatch::KillActiveWindowDispatchMessageStabby;
use crate::messages::dispatch::MoveFocusDispatchMessageStabby;
use crate::messages::dispatch::MoveToWorkspaceDispatchMessageStabby;
use crate::messages::dispatch::ToggleFloatingDispatchMessageStabby;
use crate::messages::dispatch::ToggleFullscreenDispatchMessageStabby;
use crate::messages::dispatch::WorkspaceDispatchMessageStabby;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::dispatch::TOPIC_DISPATCH;

/// The kind of Hyprland dispatch command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandDispatchActionKind {
    #[default]
    Exec,
    KillActiveWindow,
    MoveFocus,
    MoveToWorkspace,
    ToggleFloating,
    ToggleFullscreen,
    Workspace,
}

/// The main dispatch envelope sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct HyprlandDispatchMessage {
    pub kind: HyprlandDispatchActionKind,
    pub exec: stabby::option::Option<ExecDispatchMessageStabby>,
    pub kill_active_window: stabby::option::Option<KillActiveWindowDispatchMessageStabby>,
    pub move_focus: stabby::option::Option<MoveFocusDispatchMessageStabby>,
    pub move_to_workspace: stabby::option::Option<MoveToWorkspaceDispatchMessageStabby>,
    pub toggle_floating: stabby::option::Option<ToggleFloatingDispatchMessageStabby>,
    pub toggle_fullscreen: stabby::option::Option<ToggleFullscreenDispatchMessageStabby>,
    pub workspace: stabby::option::Option<WorkspaceDispatchMessageStabby>,
}

impl TypedMessage for HyprlandDispatchActionKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandDispatchActionKind");
}

impl TypedMessage for HyprlandDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandDispatchMessage");
}

impl MessageTopic for HyprlandDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for HyprlandDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
