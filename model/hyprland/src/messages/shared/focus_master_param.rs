use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Parameter for the focus-master dispatch command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandFocusMasterParam {
    #[default]
    Master,
    Auto,
}

impl TypedMessage for HyprlandFocusMasterParam {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandFocusMasterParam");
}
