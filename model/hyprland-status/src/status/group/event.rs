use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::status::group::GroupToggledStatusMessage;
use crate::status::group::IgnoreGroupLockStateChangedStatusMessage;
use crate::status::group::LockGroupsStateChangedStatusMessage;
use crate::status::group::WindowMovedIntoGroupStatusMessage;
use crate::status::group::WindowMovedOutOfGroupStatusMessage;

/// Window group-related status events.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GroupEvent {
    /// A window group was toggled.
    Toggled(GroupToggledStatusMessage),
    /// A window was moved into a group.
    MovedInto(WindowMovedIntoGroupStatusMessage),
    /// A window was moved out of a group.
    MovedOut(WindowMovedOutOfGroupStatusMessage),
    /// The ignore-group-lock state changed.
    IgnoreLockChanged(IgnoreGroupLockStateChangedStatusMessage),
    /// The lock-groups state changed.
    LockChanged(LockGroupsStateChangedStatusMessage),
}

impl TypedMessage for GroupEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::GroupEvent");
}
