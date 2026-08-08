//! MCP server for the Smearor Swipe Launcher.
//!
//! Exposes launcher control and state through the Model Context Protocol using
//! the `rust-mcp-sdk` and `rust-mcp-axum` crates for robust protocol handling
//! with Streamable HTTP and SSE transport support.

pub mod command;
mod error;
pub mod jsonrpc;
pub mod logs;
pub mod prompts;
pub mod resources;
pub mod server;
pub mod tools;

pub use crate::error::McpError;
pub use crate::logs::LogBuffer;
pub use crate::logs::LogBufferLayer;
pub use crate::logs::LogEntry;
pub use crate::logs::LogQueryResponse;
pub use crate::server::McpServer;
pub use crate::server::McpServerConfig;
pub use crate::server::McpServerState;
pub use crate::server::SwipeLauncherHandler;

pub use command::CloseAreaParams;
pub use command::CommandResponseWrapper;
pub use command::FocusAreaParams;
pub use command::GetAreaConfigParams;
pub use command::GetLogsParams;
pub use command::InstanceTypeParam;
pub use command::InvokePluginPromptParams;
pub use command::InvokePluginResourceParams;
pub use command::InvokePluginToolParams;
pub use command::InvokePromptParams;
pub use command::ListAllAreasParams;
pub use command::ListAreasParams;
pub use command::ListInstancesParams;
pub use command::LoadInstanceParams;
pub use command::McpCommand;
pub use command::McpCommandVariant;
pub use command::OpenAreaParams;
pub use command::OpenTransientAreaParams;
pub use command::ReadResourceParams;
pub use command::ReloadInstanceParams;
pub use command::SendMessageParams;
pub use command::SendMultipleMessagesParams;
pub use command::StartInstanceParams;
pub use command::StopInstanceParams;
pub use command::ToggleAreaParams;
pub use command::UnloadInstanceParams;
pub use command::WebServerStatusParams;
