//! Re-export of the shared MCP registry for the launcher host.
//!
//! Plugins send `mcp.register.tool` and `mcp.register.resource` messages to
//! publish their capabilities. The `MessageHandler` implementations live in the
//! `smearor_model_mcp` crate so the registry and its trait impls stay in the
//! same crate.

pub use smearor_model_mcp::McpRegistry;
