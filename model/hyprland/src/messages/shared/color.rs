use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// An 8-bit RGBA color, matching `hyprland::ctl::Color`.
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HyprlandColor {
    /// The red channel.
    pub red: u8,
    /// The green channel.
    pub green: u8,
    /// The blue channel.
    pub blue: u8,
    /// The alpha channel.
    pub alpha: u8,
}

impl TypedMessage for HyprlandColor {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandColor");
}
