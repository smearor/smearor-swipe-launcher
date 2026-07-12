use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Parameter for the swap-with-master dispatch command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandSwapWithMasterParam {
    #[default]
    Master,
    Child,
    Auto,
}

impl TypedMessage for HyprlandSwapWithMasterParam {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandSwapWithMasterParam");
}
