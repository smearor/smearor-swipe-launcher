#![recursion_limit = "512"]
#![allow(long_running_const_eval)]

mod mcp;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::args::compositor::*;
pub use mcp::args::ctl::*;
pub use mcp::args::system::*;
pub use mcp::args::toggle::*;
pub use mcp::args::types::McpColor;
pub use mcp::args::types::McpCorner;
pub use mcp::args::types::McpCycleDirection;
pub use mcp::args::types::McpDirection;
pub use mcp::args::types::McpFocusMasterParam;
pub use mcp::args::types::McpFullscreenType;
pub use mcp::args::types::McpLockType;
pub use mcp::args::types::McpMonitorIdentifier;
pub use mcp::args::types::McpMonitorIdentifierKind;
pub use mcp::args::types::McpNotifyIcon;
pub use mcp::args::types::McpOutputBackend;
pub use mcp::args::types::McpPosition;
pub use mcp::args::types::McpPositionKind;
pub use mcp::args::types::McpPropType;
pub use mcp::args::types::McpPropTypeKind;
pub use mcp::args::types::McpSwapWithMasterParam;
pub use mcp::args::types::McpSwitchXkbLayoutCmd;
pub use mcp::args::types::McpSwitchXkbLayoutCmdKind;
pub use mcp::args::types::McpWindowIdentifier;
pub use mcp::args::types::McpWindowMove;
pub use mcp::args::types::McpWindowMoveKind;
pub use mcp::args::types::McpWindowSwitchDirection;
pub use mcp::args::types::McpWorkspaceIdentifier;
pub use mcp::args::types::McpWorkspaceIdentifierKind;
pub use mcp::args::types::McpWorkspaceIdentifierWithSpecial;
pub use mcp::args::types::McpWorkspaceOptions;
pub use mcp::args::window::*;
pub use mcp::args::workspace::*;
pub use mcp::prompts::HyprlandMcpPrompts;
pub use mcp::requests::MoveWindowArgs;
pub use mcp::requests::SwitchWorkspaceArgs;
pub use mcp::requests::ToggleFloatingArgs;
pub use mcp::resources::HyprlandMcpResources;
pub use mcp::responses::ActiveWindowEntry;
pub use mcp::responses::GroupStatusEvent;
pub use mcp::responses::GroupStatusResponse;
pub use mcp::responses::HyprlandStateResponse;
pub use mcp::responses::KeywordGetResponse;
pub use mcp::responses::LayerStatusEvent;
pub use mcp::responses::LayerStatusResponse;
pub use mcp::responses::MonitorEntry;
pub use mcp::responses::MonitorsResponse;
pub use mcp::responses::SystemStatusEvent;
pub use mcp::responses::SystemStatusResponse;
pub use mcp::responses::VersionResponse;
pub use mcp::responses::WindowEntry;
pub use mcp::responses::WindowStatusEvent;
pub use mcp::responses::WindowStatusResponse;
pub use mcp::responses::WindowsResponse;
pub use mcp::responses::WorkspaceEntry;
pub use mcp::responses::WorkspaceSnapshotResponse;
pub use mcp::responses::WorkspaceStatusEvent;
pub use mcp::responses::WorkspaceStatusResponse;
pub use mcp::responses::WorkspacesResponse;
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
