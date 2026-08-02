use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::status::system::ConfigReloadedStatusMessage;
use crate::status::system::KeyboardLayoutChangedStatusMessage;
use crate::status::system::ScreencastStatusMessage;

/// System-level status events.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SystemEvent {
    /// The keyboard layout changed.
    KeyboardLayoutChanged(KeyboardLayoutChangedStatusMessage),
    /// A screencast state changed.
    Screencast(ScreencastStatusMessage),
    /// The Hyprland config was reloaded.
    ConfigReloaded(ConfigReloadedStatusMessage),
}

impl TypedMessage for SystemEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SystemEvent");
}
