use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::toggle::ToggleDpmsDispatchMessageStabby;
use crate::dispatch::toggle::ToggleFakeFullscreenDispatchMessage;
use crate::dispatch::toggle::ToggleFloatingDispatchMessage;
use crate::dispatch::toggle::ToggleFullscreenDispatchMessage;
use crate::dispatch::toggle::ToggleGroupDispatchMessage;
use crate::dispatch::toggle::ToggleOpaqueDispatchMessage;
use crate::dispatch::toggle::TogglePinDispatchMessage;
use crate::dispatch::toggle::TogglePseudoDispatchMessage;
use crate::dispatch::toggle::ToggleSplitDispatchMessage;

/// Kind for toggle-related dispatch commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToggleDispatchKind {
    #[default]
    ToggleDpms,
    ToggleFakeFullscreen,
    ToggleFloating,
    ToggleFullscreen,
    ToggleGroup,
    ToggleOpaque,
    TogglePin,
    TogglePseudo,
    ToggleSplit,
}

/// Toggle-related dispatch options.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToggleDispatchOps {
    pub toggle_dpms: stabby::option::Option<ToggleDpmsDispatchMessageStabby>,
    pub toggle_fake_fullscreen: stabby::option::Option<ToggleFakeFullscreenDispatchMessage>,
    pub toggle_floating: stabby::option::Option<ToggleFloatingDispatchMessage>,
    pub toggle_fullscreen: stabby::option::Option<ToggleFullscreenDispatchMessage>,
    pub toggle_group: stabby::option::Option<ToggleGroupDispatchMessage>,
    pub toggle_opaque: stabby::option::Option<ToggleOpaqueDispatchMessage>,
    pub toggle_pin: stabby::option::Option<TogglePinDispatchMessage>,
    pub toggle_pseudo: stabby::option::Option<TogglePseudoDispatchMessage>,
    pub toggle_split: stabby::option::Option<ToggleSplitDispatchMessage>,
}

impl TypedMessage for ToggleDispatchKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleDispatchKind");
}
