//! MCP server module: configuration, state, server lifecycle, and request handler.

mod config;
mod handler;
mod mcp_server;
mod state;

pub use config::McpServerConfig;
pub use handler::SwipeLauncherHandler;
pub use mcp_server::McpServer;
pub use state::McpServerState;
