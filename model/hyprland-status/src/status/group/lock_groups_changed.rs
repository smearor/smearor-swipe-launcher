use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when the lock-groups state changes.
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockGroupsStateChangedStatusMessage {
    /// Whether groups are now locked.
    pub is_locked: bool,
}

impl TypedMessage for LockGroupsStateChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::LockGroupsStateChangedStatusMessage");
}
