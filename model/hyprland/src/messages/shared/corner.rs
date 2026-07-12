use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Corner position for move-to-corner commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandCorner {
    #[default]
    BottomLeft,
    BottomRight,
    TopRight,
    TopLeft,
}

impl TypedMessage for HyprlandCorner {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandCorner");
}
