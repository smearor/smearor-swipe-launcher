use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Emitted when the ignore-group-lock state changes.
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoreGroupLockStateChangedStatusMessage {
    /// Whether ignore-group-lock is now enabled.
    pub is_enabled: bool,
}

impl TypedMessage for IgnoreGroupLockStateChangedStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::IgnoreGroupLockStateChangedStatusMessage");
}
