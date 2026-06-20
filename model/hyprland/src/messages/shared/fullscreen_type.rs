use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Fullscreen mode for toggle fullscreen commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandFullscreenType {
    Real,
    Maximize,
    #[default]
    NoParam,
}

impl TypedMessage for HyprlandFullscreenType {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandFullscreenType");
}
