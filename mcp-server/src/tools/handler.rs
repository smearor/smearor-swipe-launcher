use crate::McpCommand;
use crate::tools::invocation::ToolInvocation;
use crate::tools::result::ToolResult;
use std::future::Future;
use std::pin::Pin;

/// Future returned by a tool handler.
pub type ToolFuture = Pin<Box<dyn Future<Output = ToolResult> + Send>>;

/// Tool handler signature.
pub type ToolHandler = Box<dyn Fn(ToolInvocation) -> ToolFuture + Send + Sync>;
