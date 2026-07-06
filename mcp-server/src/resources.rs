//! MCP resource definitions and invocation helpers.

use crate::McpCommand;
use crate::jsonrpc::JSONRPC_INTERNAL_ERROR;
use crate::jsonrpc::JSONRPC_INVALID_PARAMS;
use crate::jsonrpc::JsonRpcResponse;
use async_channel::Sender;
use serde_json::Value;
use tokio::sync::oneshot;

/// Resource handler signature.
pub type ResourceHandler = Box<dyn Fn(Sender<McpCommand>, String) -> ResourceFuture + Send + Sync>;

/// Future returned by a resource handler.
pub type ResourceFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>;

/// Built-in resource definition exposed by the MCP server.
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub handler: ResourceHandler,
}

/// Build the list of core resources available from the MVP.
pub fn core_resources() -> Vec<ResourceDefinition> {
    vec![ResourceDefinition {
        uri: "area://list".to_string(),
        name: "area_list".to_string(),
        description: "List of all configured areas with status and position.".to_string(),
        mime_type: "application/json".to_string(),
        handler: Box::new(|sender, _uri| Box::pin(async move { read_resource(sender, "area://list".to_string()).await })),
    }]
}

/// Read a resource by sending the request to the launcher core.
async fn read_resource(sender: Sender<McpCommand>, uri: String) -> Result<String, String> {
    let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
    sender
        .try_send(McpCommand::ReadResource { uri, response: response_tx })
        .map_err(|e| format!("Failed to send resource read command: {}", e))?;
    match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Launcher core dropped the response channel".to_string()),
        Err(_) => Err("Resource read timed out".to_string()),
    }
}

/// Read a core resource by URI and return (contents, mime_type) for the SDK
/// ServerHandler. Returns Err(message) on failure.
pub async fn read_resource_sdk(resources: &[ResourceDefinition], sender: Sender<McpCommand>, uri: &str) -> Result<(String, String), String> {
    let Some(resource) = resources.iter().find(|r| r.uri == uri) else {
        return Err(format!("Resource {} not found", uri));
    };
    let mime_type = resource.mime_type.clone();
    match (resource.handler)(sender, uri.to_string()).await {
        Ok(contents) => Ok((contents, mime_type)),
        Err(message) => Err(message),
    }
}

/// Read a resource by URI and return a JSON-RPC response.
pub async fn read_resource_response(resources: &[ResourceDefinition], sender: Sender<McpCommand>, id: Option<Value>, uri: String) -> JsonRpcResponse {
    let Some(resource) = resources.iter().find(|r| uri == r.uri) else {
        return JsonRpcResponse::error(id, JSONRPC_INVALID_PARAMS, format!("Resource {} not found", uri), None);
    };

    match (resource.handler)(sender, uri.clone()).await {
        Ok(contents) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": resource.mime_type,
                    "text": contents
                }]
            }),
        ),
        Err(message) => JsonRpcResponse::error(id, JSONRPC_INTERNAL_ERROR, message, None),
    }
}
