use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Identifies a window by address, class, title, or process ID.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWindowIdentifier {
    ProcessId(u32),
    Address(stabby::string::String),
    ClassRegularExpression(stabby::string::String),
    Title(stabby::string::String),
}

impl Default for HyprlandWindowIdentifier {
    fn default() -> Self {
        HyprlandWindowIdentifier::ProcessId(0)
    }
}

impl TypedMessage for HyprlandWindowIdentifier {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowIdentifier");
}
