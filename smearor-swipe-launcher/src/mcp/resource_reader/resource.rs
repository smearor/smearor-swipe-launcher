pub use crate::mcp::resource_reader::registry::McpResourceHandlerRegistry;

use crate::host::LauncherHost;

/// Read an MCP resource by URI using the default handler registry.
pub fn read_mcp_resource(host: &LauncherHost, uri: &str) -> Result<String, String> {
    let registry = McpResourceHandlerRegistry::default();
    registry.read(host, uri)
}
