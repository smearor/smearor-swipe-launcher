use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// The kind of workspace identifier, matching `hyprland::dispatch::WorkspaceIdentifierWithSpecial` variants.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandWorkspaceIdentifierKind {
    #[default]
    Previous,
    Empty,
    Id,
    Relative,
    RelativeMonitor,
    RelativeMonitorIncludingEmpty,
    RelativeOpen,
    Name,
    Special,
}

/// Identifies a workspace, including special workspaces.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct HyprlandWorkspaceIdentifierWithSpecial {
    /// The kind of identifier.
    pub kind: HyprlandWorkspaceIdentifierKind,
    /// Numeric value for Id/Relative variants.
    pub id: i32,
    /// Name value for the Name variant.
    pub name: stabby::option::Option<stabby::string::String>,
    /// Optional name for the Special variant.
    pub special_name: stabby::option::Option<stabby::string::String>,
}

impl TypedMessage for HyprlandWorkspaceIdentifierKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWorkspaceIdentifierKind");
}

impl TypedMessage for HyprlandWorkspaceIdentifierWithSpecial {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWorkspaceIdentifierWithSpecial");
}
