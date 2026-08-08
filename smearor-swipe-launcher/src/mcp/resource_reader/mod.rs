mod area_buttons_handler;
mod area_list_handler;
mod area_plugins_handler;
mod area_state_handler;
mod common;
mod plugin_list_handler;
mod registry;

pub use registry::McpResourceHandler;
pub use registry::McpResourceHandlerRegistry;

use crate::host::LauncherHost;

/// Read an MCP resource by URI using the default handler registry.
pub fn read_mcp_resource(host: &LauncherHost, uri: &str) -> Result<String, String> {
    let registry = McpResourceHandlerRegistry::default();
    registry.read(host, uri)
}
