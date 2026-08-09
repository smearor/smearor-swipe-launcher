use hyprland::dispatch::WorkspaceOptions;
use smearor_hyprland_model::HyprlandWorkspaceOptions;

pub(crate) fn convert_workspace_options(opt: HyprlandWorkspaceOptions) -> WorkspaceOptions {
    match opt {
        HyprlandWorkspaceOptions::AllPseudo => WorkspaceOptions::AllPseudo,
        HyprlandWorkspaceOptions::AllFloat => WorkspaceOptions::AllFloat,
    }
}
