use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Identifies a workspace without special workspace support.
#[repr(stabby)]
#[stabby::stabby]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWorkspaceIdentifier {
    Previous,
    Empty,
    Id(i32),
    Relative(i32),
    RelativeMonitor(i32),
    RelativeMonitorIncludingEmpty(i32),
    RelativeOpen(i32),
    Name(stabby::string::String),
}

impl Default for HyprlandWorkspaceIdentifier {
    fn default() -> Self {
        HyprlandWorkspaceIdentifier::Previous()
    }
}

impl TypedMessage for HyprlandWorkspaceIdentifier {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWorkspaceIdentifier");
}
