use smearor_mcp_server::McpCommand;
use smearor_mcp_server::McpServer;
use smearor_mcp_server::McpServerConfig;

use crate::LauncherHost;
use crate::config::services::ServicesConfig;

/// Handles returned by `start_mcp_server` for use in the main loop and shutdown.
pub struct McpServerHandles {
    pub server: Option<McpServer>,
    pub command_receiver: async_channel::Receiver<McpCommand>,
}

/// Start the MCP server and register the command sender on the host.
pub fn start_mcp_server(host: &LauncherHost, services_config: &ServicesConfig) -> McpServerHandles {
    let mcp_config = McpServerConfig {
        bind_address: services_config.mcp.bind_address.clone(),
        port: services_config.mcp.port,
        auth_token: services_config.mcp.auth_token.clone(),
    };
    let (mcp_command_sender, command_receiver) = async_channel::unbounded::<McpCommand>();
    let mut server = McpServer::new(mcp_config, host.mcp_registry.clone(), mcp_command_sender.clone());
    host.set_mcp_command_sender(mcp_command_sender);
    server.start();

    McpServerHandles {
        server: Some(server),
        command_receiver,
    }
}
