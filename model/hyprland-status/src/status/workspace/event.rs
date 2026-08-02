use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::status::workspace::ChangedSpecialStatusMessage;
use crate::status::workspace::FullscreenStateChangedStatusMessage;
use crate::status::workspace::SpecialRemovedStatusMessage;
use crate::status::workspace::SubMapChangedStatusMessage;
use crate::status::workspace::WorkspaceRenamedStatusMessage;

/// Workspace-related status events.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkspaceEvent {
    /// The fullscreen state changed.
    FullscreenStateChanged(FullscreenStateChangedStatusMessage),
    /// A workspace was renamed.
    Renamed(WorkspaceRenamedStatusMessage),
    /// A special workspace was removed.
    SpecialRemoved(SpecialRemovedStatusMessage),
    /// A special workspace was changed (opened/closed/toggled).
    ChangedSpecial(ChangedSpecialStatusMessage),
    /// The submap changed.
    SubMapChanged(SubMapChangedStatusMessage),
}

impl TypedMessage for WorkspaceEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceEvent");
}
