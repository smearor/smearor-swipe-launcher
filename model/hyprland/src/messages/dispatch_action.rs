use crate::messages::dispatch::ExecDispatchMessageStabby;
use crate::messages::dispatch::KillActiveWindowDispatchMessageStabby;
use crate::messages::dispatch::MoveFocusDispatchMessageStabby;
use crate::messages::dispatch::ToggleFullscreenDispatchMessageStabby;
use crate::messages::dispatch::WorkspaceDispatchMessageStabby;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::dispatch::TOPIC_DISPATCH;

/// Unified enum for all Hyprland dispatch commands.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug)]
pub enum HyprlandDispatchAction {
    Exec(ExecDispatchMessageStabby),
    KillActiveWindow(KillActiveWindowDispatchMessageStabby),
    MoveFocus(MoveFocusDispatchMessageStabby),
    ToggleFullscreen(ToggleFullscreenDispatchMessageStabby),
    Workspace(WorkspaceDispatchMessageStabby),
}

impl TypedMessage for HyprlandDispatchAction {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandDispatchAction");
}

impl MessageTopic for HyprlandDispatchAction {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for HyprlandDispatchAction {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
