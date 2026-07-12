use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Workspace layout options for the workspace-toggle command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandWorkspaceOptions {
    #[default]
    AllPseudo,
    AllFloat,
}

impl TypedMessage for HyprlandWorkspaceOptions {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWorkspaceOptions");
}
