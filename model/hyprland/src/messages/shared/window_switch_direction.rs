use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Direction for switching between windows in the stack.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandWindowSwitchDirection {
    #[default]
    Back,
    Forward,
}

impl TypedMessage for HyprlandWindowSwitchDirection {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowSwitchDirection");
}
