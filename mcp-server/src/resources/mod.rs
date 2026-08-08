//! MCP resource definitions and invocation helpers.

mod core;
mod creator;
mod definition;
mod into_sdk_resource;
mod read_resource_output;
mod registered;
mod resource_content;
mod result;

use crate::CommandResponseWrapper;
use crate::McpCommand;
use crate::ReadResourceParams;
use crate::jsonrpc::JSONRPC_INTERNAL_ERROR;
use crate::jsonrpc::JSONRPC_INVALID_PARAMS;
use crate::jsonrpc::JsonRpcResponse;
use async_channel::Sender;
use serde_json::Value;
use tokio::sync::oneshot;

pub use definition::ResourceDefinition;
pub use definition::ResourceFuture;
pub use definition::ResourceHandler;
pub use into_sdk_resource::IntoSdkResource;
pub use into_sdk_resource::SdkResourceFields;
pub use read_resource_output::ReadResourceOutput;
pub use resource_content::ReadResourceResult;
pub use resource_content::ResourceContent;
pub use result::ResourceResult;

/// Read a resource by sending the request to the launcher core.
pub(crate) async fn read_resource(sender: Sender<McpCommand>, uri: String, mime_type: String) -> Result<ReadResourceOutput, String> {
    let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
    sender
        .try_send(
            CommandResponseWrapper::builder()
                .params(ReadResourceParams::builder().uri(uri).build())
                .response(response_tx)
                .build()
                .into(),
        )
        .map_err(|e| format!("Failed to send resource read command: {}", e))?;
    match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
        Ok(Ok(Ok(contents))) => Ok(ReadResourceOutput { contents, mime_type }),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(_)) => Err("Launcher core dropped the response channel".to_string()),
        Err(_) => Err("Resource read timed out".to_string()),
    }
}

/// Read a core resource by URI and return a `ReadResourceOutput` for the SDK
/// ServerHandler. Returns Err(message) on failure.
pub async fn read_resource_sdk(resources: &[ResourceDefinition], sender: Sender<McpCommand>, uri: &str) -> Result<ReadResourceOutput, String> {
    let Some(resource) = resources.iter().find(|r| r.uri == uri) else {
        return Err(format!("Resource {} not found", uri));
    };
    let mime_type = resource.mime_type.clone();
    match (resource.handler)(sender, uri.to_string()).await {
        Ok(contents) => Ok(ReadResourceOutput { contents, mime_type }),
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
            serde_json::to_value(ReadResourceResult {
                contents: vec![ResourceContent {
                    uri,
                    mime_type: resource.mime_type.clone(),
                    text: contents,
                }],
            })
            .unwrap_or_default(),
        ),
        Err(message) => JsonRpcResponse::error(id, JSONRPC_INTERNAL_ERROR, message, None),
    }
}
