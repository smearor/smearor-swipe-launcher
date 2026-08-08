use schemars::JsonSchema;
use schemars::schema_for;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::CommandResponseWrapper;
use crate::McpCommand;
use crate::McpCommandVariant;
use crate::tools::definition::ToolDefinition;
use crate::tools::handler::ToolFuture;
use crate::tools::handler::ToolHandler;
use crate::tools::invocation::ToolInvocation;
use crate::tools::result::ToolResult;
use async_channel::Sender;
use tokio::sync::oneshot;

/// Trait that lets a params struct generate its own `ToolDefinition`.
pub trait ToolDefinitionCreator: JsonSchema + DeserializeOwned + McpCommandVariant + Send + 'static {
    /// The MCP tool name (e.g. "open_area").
    fn tool_name() -> &'static str;
    /// The human-readable description shown to the LLM.
    fn tool_description() -> &'static str;

    /// Create the full `ToolDefinition` with schema, name, description and handler.
    fn create_tool_definition() -> ToolDefinition {
        ToolDefinition {
            name: Self::tool_name().to_string(),
            description: Self::tool_description().to_string(),
            input_schema: schema_for!(Self).to_value(),
            handler: Box::new(|invocation| make_tool_handler::<Self>(invocation)),
        }
    }
}

/// Parse tool parameters from a JSON value into a typed struct.
fn parse_params<T: DeserializeOwned>(params: Option<&Value>) -> Result<T, String> {
    let value = params.cloned().unwrap_or(serde_json::Value::Object(Default::default()));
    serde_json::from_value(value).map_err(|e| format!("Invalid parameters: {e}"))
}

/// Send a typed params struct as a command and wait for the response.
async fn send_params_and_wait<T: McpCommandVariant + Send + 'static>(sender: Sender<McpCommand>, params: T) -> ToolResult {
    send_command_and_wait(sender, |response| CommandResponseWrapper::builder().params(params).response(response).build().into()).await
}

/// Build a tool handler future from parsed parameters.
fn make_tool_handler<T: DeserializeOwned + McpCommandVariant + Send + 'static>(invocation: ToolInvocation) -> ToolFuture {
    let ToolInvocation { sender, params } = invocation;
    let params = match parse_params::<T>(params) {
        Ok(p) => p,
        Err(e) => return Box::pin(async move { Err(e) }),
    };
    Box::pin(async move { send_params_and_wait(sender, params).await })
}

/// Send a command and wait for the launcher core to respond.
/// The closure receives the real response sender and constructs the full `McpCommand`.
async fn send_command_and_wait<F>(sender: Sender<McpCommand>, build_command: F) -> ToolResult
where
    F: FnOnce(oneshot::Sender<Result<String, String>>) -> McpCommand,
{
    let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
    let command = build_command(response_tx);

    sender
        .try_send(command)
        .map_err(|e| format!("Failed to send command to launcher core: {}", e))?;

    match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
        Ok(Ok(Ok(result))) => Ok(Value::String(result)),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(_)) => Err("Launcher core dropped the response channel".to_string()),
        Err(_) => Err("Tool invocation timed out".to_string()),
    }
}
