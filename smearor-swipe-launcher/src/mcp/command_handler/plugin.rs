use crate::mcp::McpResponseTracker;
use smearor_mcp_server::McpCommand;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use super::plugin_prompt::handle_plugin_prompt;
use super::plugin_resource::handle_plugin_resource;
use super::plugin_tool::handle_plugin_tool;

/// Process plugin tool/resource invocations on a tokio task so they don't
/// block the GLib main context. Only `Send` types are used here.
pub async fn process_plugin_command(broker_sender: UnboundedSender<FfiEnvelope>, response_tracker: McpResponseTracker, command: McpCommand) {
    match command {
        McpCommand::InvokePluginTool(cmd) => {
            handle_plugin_tool(&broker_sender, &response_tracker, cmd).await;
        }
        McpCommand::InvokePluginResource(cmd) => {
            handle_plugin_resource(&broker_sender, &response_tracker, cmd).await;
        }
        McpCommand::InvokePluginPrompt(cmd) => {
            handle_plugin_prompt(&broker_sender, &response_tracker, cmd).await;
        }
        _ => {
            debug!("process_plugin_command received non-plugin command, ignoring");
        }
    }
}
