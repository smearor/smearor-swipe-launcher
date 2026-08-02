use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::status::GroupEvent;
use crate::status::LayerEvent;
use crate::status::SystemEvent;
use crate::status::WindowEvent;
use crate::status::WorkspaceEvent;

/// Topic for window-related status event broadcasts.
pub const TOPIC_HYPRLAND_WINDOW_STATUS: &str = "service.hyprland.window.status";

/// Topic for workspace-related status event broadcasts.
pub const TOPIC_HYPRLAND_WORKSPACE_STATUS: &str = "service.hyprland.workspace.status";

/// Topic for window group-related status event broadcasts.
pub const TOPIC_HYPRLAND_GROUP_STATUS: &str = "service.hyprland.group.status";

/// Topic for layer shell surface status event broadcasts.
pub const TOPIC_HYPRLAND_LAYER_STATUS: &str = "service.hyprland.layer.status";

/// Topic for system-level status event broadcasts.
pub const TOPIC_HYPRLAND_SYSTEM_STATUS: &str = "service.hyprland.system.status";

/// Window-related status event broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandWindowStatusMessage {
    /// The window status event payload.
    pub event: WindowEvent,
}

impl Default for HyprlandWindowStatusMessage {
    fn default() -> Self {
        Self {
            event: WindowEvent::ActiveChanged(Default::default()),
        }
    }
}

impl TypedMessage for HyprlandWindowStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowStatusMessage");
}

impl MessageTopic for HyprlandWindowStatusMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_WINDOW_STATUS
    }
}

/// Workspace-related status event broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandWorkspaceStatusMessage {
    /// The workspace status event payload.
    pub event: WorkspaceEvent,
}

impl Default for HyprlandWorkspaceStatusMessage {
    fn default() -> Self {
        Self {
            event: WorkspaceEvent::FullscreenStateChanged(Default::default()),
        }
    }
}

impl TypedMessage for HyprlandWorkspaceStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWorkspaceStatusMessage");
}

impl MessageTopic for HyprlandWorkspaceStatusMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_WORKSPACE_STATUS
    }
}

/// Window group-related status event broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandGroupStatusMessage {
    /// The group status event payload.
    pub event: GroupEvent,
}

impl Default for HyprlandGroupStatusMessage {
    fn default() -> Self {
        Self {
            event: GroupEvent::Toggled(Default::default()),
        }
    }
}

impl TypedMessage for HyprlandGroupStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandGroupStatusMessage");
}

impl MessageTopic for HyprlandGroupStatusMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_GROUP_STATUS
    }
}

/// Layer shell surface status event broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandLayerStatusMessage {
    /// The layer status event payload.
    pub event: LayerEvent,
}

impl Default for HyprlandLayerStatusMessage {
    fn default() -> Self {
        Self {
            event: LayerEvent::Opened(Default::default()),
        }
    }
}

impl TypedMessage for HyprlandLayerStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandLayerStatusMessage");
}

impl MessageTopic for HyprlandLayerStatusMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_LAYER_STATUS
    }
}

/// System-level status event broadcast by the service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandSystemStatusMessage {
    /// The system status event payload.
    pub event: SystemEvent,
}

impl Default for HyprlandSystemStatusMessage {
    fn default() -> Self {
        Self {
            event: SystemEvent::ConfigReloaded(Default::default()),
        }
    }
}

impl TypedMessage for HyprlandSystemStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandSystemStatusMessage");
}

impl MessageTopic for HyprlandSystemStatusMessage {
    fn topic() -> &'static str {
        TOPIC_HYPRLAND_SYSTEM_STATUS
    }
}
