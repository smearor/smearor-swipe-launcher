pub mod compositor;
pub mod ctl;
pub mod state;
pub mod system;
pub mod toggle;
pub mod window;
pub mod workspace;

use crate::service::HyprlandService;
use smearor_hyprland_model::HyprlandMcpTools;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        let arguments = message.0.arguments.to_string();
        trace!("Hyprland Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match HyprlandMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(error) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(error, &correlation_id)));
                return;
            }
        };
        match tool {
            HyprlandMcpTools::WindowCenter
            | HyprlandMcpTools::WindowChangeGroupActive
            | HyprlandMcpTools::WindowChangeSplitRatio
            | HyprlandMcpTools::WindowClose
            | HyprlandMcpTools::WindowCycle
            | HyprlandMcpTools::WindowExec
            | HyprlandMcpTools::WindowFocusCurrentOrLast
            | HyprlandMcpTools::WindowFocusMaster
            | HyprlandMcpTools::WindowFocusMonitor
            | HyprlandMcpTools::WindowFocusUrgentOrLast
            | HyprlandMcpTools::WindowFocusWindow
            | HyprlandMcpTools::WindowKillActive
            | HyprlandMcpTools::WindowMoveActive
            | HyprlandMcpTools::WindowMoveCursor
            | HyprlandMcpTools::WindowMoveCursorToCorner
            | HyprlandMcpTools::WindowMoveFocus
            | HyprlandMcpTools::WindowMoveIntoGroup
            | HyprlandMcpTools::WindowMoveWindow
            | HyprlandMcpTools::WindowMoveWindowPixel
            | HyprlandMcpTools::WindowResizeActive
            | HyprlandMcpTools::WindowResizeWindowPixel
            | HyprlandMcpTools::WindowSwap
            | HyprlandMcpTools::WindowSwapWithMaster => {
                self.handle_window_tool(tool, &arguments, &correlation_id, &broadcaster);
            }

            HyprlandMcpTools::MoveWindowToWorkspace
            | HyprlandMcpTools::WorkspaceMoveCurrentToMonitor
            | HyprlandMcpTools::WorkspaceMoveFocusedWindow
            | HyprlandMcpTools::WorkspaceMoveFocusedWindowSilent
            | HyprlandMcpTools::WorkspaceMoveToWorkspaceSilent
            | HyprlandMcpTools::WorkspaceRename
            | HyprlandMcpTools::WorkspaceSwapActive
            | HyprlandMcpTools::WorkspaceToggleSpecial
            | HyprlandMcpTools::WorkspaceOption => {
                self.handle_workspace_tool(tool, &arguments, &correlation_id, &broadcaster);
            }

            HyprlandMcpTools::ToggleFloating
            | HyprlandMcpTools::ToggleFullscreen
            | HyprlandMcpTools::ToggleDpms
            | HyprlandMcpTools::ToggleFakeFullscreen
            | HyprlandMcpTools::ToggleGroup
            | HyprlandMcpTools::ToggleOpaque
            | HyprlandMcpTools::TogglePin
            | HyprlandMcpTools::TogglePseudo
            | HyprlandMcpTools::ToggleSplit => {
                self.handle_toggle_tool(tool, &arguments, &correlation_id, &broadcaster);
            }

            HyprlandMcpTools::SystemAddMaster
            | HyprlandMcpTools::SystemBringActiveToTop
            | HyprlandMcpTools::SystemCustom
            | HyprlandMcpTools::SystemExit
            | HyprlandMcpTools::SystemForceRendererReload
            | HyprlandMcpTools::SystemGlobal
            | HyprlandMcpTools::SystemLockGroups
            | HyprlandMcpTools::SystemMoveOutOfGroup
            | HyprlandMcpTools::SystemOrientation
            | HyprlandMcpTools::SystemPass
            | HyprlandMcpTools::SystemRemoveMaster
            | HyprlandMcpTools::SystemSetCursor => {
                self.handle_system_tool(tool, &arguments, &correlation_id, &broadcaster);
            }

            HyprlandMcpTools::CtlKill
            | HyprlandMcpTools::CtlNotify
            | HyprlandMcpTools::CtlOutputCreate
            | HyprlandMcpTools::CtlOutputRemove
            | HyprlandMcpTools::CtlPluginLoad
            | HyprlandMcpTools::CtlPluginUnload
            | HyprlandMcpTools::CtlReload
            | HyprlandMcpTools::CtlSetCursor
            | HyprlandMcpTools::CtlSetError
            | HyprlandMcpTools::CtlSetProp
            | HyprlandMcpTools::CtlSwitchXkbLayout => {
                self.handle_ctl_tool(tool, &arguments, &correlation_id, &broadcaster);
            }

            HyprlandMcpTools::SwitchWorkspace | HyprlandMcpTools::CompositorCreateWorkspace | HyprlandMcpTools::CompositorSwitchWorkspace => {
                self.handle_compositor_tool(tool, &arguments, &correlation_id, &broadcaster);
            }

            HyprlandMcpTools::RefreshState => {
                self.handle_refresh_state_tool(&correlation_id, &broadcaster);
            }
        }
    }
}
