use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Output backend type for the output-create command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandOutputBackend {
    #[default]
    Wayland,
    X11,
    Headless,
    Auto,
}

impl TypedMessage for HyprlandOutputBackend {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandOutputBackend");
}
