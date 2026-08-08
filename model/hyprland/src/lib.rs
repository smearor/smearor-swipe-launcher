#![recursion_limit = "512"]
#![allow(long_running_const_eval)]

mod mcp;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::requests::MoveWindowArgs;
pub use mcp::requests::SwitchWorkspaceArgs;
pub use mcp::requests::ToggleFloatingArgs;
pub use mcp::resources::HyprlandMcpResources;
pub use mcp::responses::ActiveWindowEntry;
pub use mcp::responses::HyprlandStateResponse;
pub use mcp::tools::HyprlandMcpTools;

pub use smearor_hyprland_command::*;
pub use smearor_hyprland_dispatch::*;
pub use smearor_hyprland_shared::*;
pub use smearor_hyprland_status::*;

/// Register all JSON converter implementations for Hyprland messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    smearor_hyprland_command::register_json_converters(context);
    smearor_hyprland_dispatch::register_json_converters(context);
    smearor_hyprland_status::register_json_converters(context);
}
