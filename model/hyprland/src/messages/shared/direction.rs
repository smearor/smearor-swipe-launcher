use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Direction for movement and focus commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandDirection {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

impl TypedMessage for HyprlandDirection {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandDirection");
}
