use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Direction for cycling through windows or workspaces.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandCycleDirection {
    #[default]
    Next,
    Previous,
}

impl TypedMessage for HyprlandCycleDirection {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandCycleDirection");
}
