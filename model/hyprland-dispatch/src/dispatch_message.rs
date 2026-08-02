use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::SystemDispatchKind;
use crate::dispatch::SystemDispatchOps;
use crate::dispatch::ToggleDispatchKind;
use crate::dispatch::ToggleDispatchOps;
use crate::dispatch::WindowDispatchKind;
use crate::dispatch::WindowDispatchOps;
use crate::dispatch::WorkspaceDispatchKind;
use crate::dispatch::WorkspaceDispatchOps;

/// Topic for window-related dispatch commands.
pub const TOPIC_WINDOW_DISPATCH: &str = "service.hyprland.dispatch.window";

/// Topic for workspace-related dispatch commands.
pub const TOPIC_WORKSPACE_DISPATCH: &str = "service.hyprland.dispatch.workspace";

/// Topic for toggle-related dispatch commands.
pub const TOPIC_TOGGLE_DISPATCH: &str = "service.hyprland.dispatch.toggle";

/// Topic for system-related dispatch commands.
pub const TOPIC_SYSTEM_DISPATCH: &str = "service.hyprland.dispatch.system";

/// Window-related dispatch message sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandWindowDispatchMessage {
    /// The dispatch kind selector.
    pub kind: WindowDispatchKind,
    /// The dispatch options payload.
    pub ops: WindowDispatchOps,
}

impl Default for HyprlandWindowDispatchMessage {
    fn default() -> Self {
        Self {
            kind: WindowDispatchKind::default(),
            ops: WindowDispatchOps::default(),
        }
    }
}

impl TypedMessage for HyprlandWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowDispatchMessage");
}

impl MessageTopic for HyprlandWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_WINDOW_DISPATCH
    }
}

impl SharedMessage for HyprlandWindowDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WINDOW_DISPATCH
    }
}

/// Workspace-related dispatch message sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandWorkspaceDispatchMessage {
    /// The dispatch kind selector.
    pub kind: WorkspaceDispatchKind,
    /// The dispatch options payload.
    pub ops: WorkspaceDispatchOps,
}

impl Default for HyprlandWorkspaceDispatchMessage {
    fn default() -> Self {
        Self {
            kind: WorkspaceDispatchKind::default(),
            ops: WorkspaceDispatchOps::default(),
        }
    }
}

impl TypedMessage for HyprlandWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWorkspaceDispatchMessage");
}

impl MessageTopic for HyprlandWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_DISPATCH
    }
}

impl SharedMessage for HyprlandWorkspaceDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_DISPATCH
    }
}

/// Toggle-related dispatch message sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandToggleDispatchMessage {
    /// The dispatch kind selector.
    pub kind: ToggleDispatchKind,
    /// The dispatch options payload.
    pub ops: ToggleDispatchOps,
}

impl Default for HyprlandToggleDispatchMessage {
    fn default() -> Self {
        Self {
            kind: ToggleDispatchKind::default(),
            ops: ToggleDispatchOps::default(),
        }
    }
}

impl TypedMessage for HyprlandToggleDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandToggleDispatchMessage");
}

impl MessageTopic for HyprlandToggleDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_TOGGLE_DISPATCH
    }
}

impl SharedMessage for HyprlandToggleDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_TOGGLE_DISPATCH
    }
}

/// System-related dispatch message sent by widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandSystemDispatchMessage {
    /// The dispatch kind selector.
    pub kind: SystemDispatchKind,
    /// The dispatch options payload.
    pub ops: SystemDispatchOps,
}

impl Default for HyprlandSystemDispatchMessage {
    fn default() -> Self {
        Self {
            kind: SystemDispatchKind::default(),
            ops: SystemDispatchOps::default(),
        }
    }
}

impl TypedMessage for HyprlandSystemDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandSystemDispatchMessage");
}

impl MessageTopic for HyprlandSystemDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_SYSTEM_DISPATCH
    }
}

impl SharedMessage for HyprlandSystemDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_SYSTEM_DISPATCH
    }
}
