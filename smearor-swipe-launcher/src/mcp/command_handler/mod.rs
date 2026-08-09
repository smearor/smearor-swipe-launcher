mod area;
mod common;
mod instance;
mod messaging;
mod plugin;
mod plugin_prompt;
mod plugin_resource;
mod plugin_tool;
mod resource;
mod web;

use crate::host::LauncherHost;
use smearor_mcp_server::McpCommand;
use tracing::debug;

pub use plugin::process_plugin_command;

pub async fn process_mcp_command(host: LauncherHost, command: McpCommand) {
    debug!(
        "process_mcp_command: ServiceManager ptr={:p} count={}",
        host.service_manager.as_ref(),
        host.service_manager.services.len()
    );
    match command {
        McpCommand::OpenArea(..)
        | McpCommand::OpenTransientArea(..)
        | McpCommand::CloseArea(..)
        | McpCommand::FocusArea(..)
        | McpCommand::ListAreas(..)
        | McpCommand::ListAllAreas(..)
        | McpCommand::ToggleArea(..)
        | McpCommand::GetAreaConfig(..) => area::handle_area_command(&host, command),

        McpCommand::SendMessage(..) | McpCommand::SendMultipleMessages(..) => messaging::handle_messaging_command(&host, command),

        McpCommand::ReadResource(..) => resource::handle_resource_command(&host, command),

        McpCommand::LoadInstance(..)
        | McpCommand::StartInstance(..)
        | McpCommand::StopInstance(..)
        | McpCommand::UnloadInstance(..)
        | McpCommand::ReloadInstance(..)
        | McpCommand::ListInstances(..) => instance::handle_instance_command(&host, command),

        McpCommand::WebServerStatus(..) => web::handle_web_command(&host, command),

        McpCommand::GetLogs(..) => {
            debug!("GetLogs command received in command handler — should be handled directly by MCP server");
        }

        McpCommand::InvokePrompt(..) => {
            debug!("InvokePrompt command received in command handler — should be handled directly by MCP server");
        }

        McpCommand::ReadResourceTool(..) => {
            debug!("ReadResourceTool command received in command handler — should be handled directly by MCP server");
        }

        _ => {
            debug!("process_mcp_command received plugin command, ignoring (handled by process_plugin_command)");
        }
    }
}
