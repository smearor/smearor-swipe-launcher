use async_channel::Sender;
use serde_json::Value;
use tokio::sync::oneshot;

use crate::CommandResponseWrapper;
use crate::McpCommand;
use crate::McpError;
use crate::ReadResourceParams;
use crate::jsonrpc::JSONRPC_INTERNAL_ERROR;
use crate::jsonrpc::JSONRPC_INVALID_PARAMS;
use crate::jsonrpc::JsonRpcResponse;
use crate::resources::ReadResourceOutput;
use crate::resources::ReadResourceResult;
use crate::resources::ResourceContent;
use crate::resources::ResourceDefinition;

/// Resolver that bridges resource definitions with the rust-mcp-sdk schema types.
/// Provides SDK-facing operations for reading resources and generating JSON-RPC responses.
pub struct ResourceResolver<'a> {
    resources: &'a [ResourceDefinition],
}

impl<'a> ResourceResolver<'a> {
    /// Create a new resolver over a slice of resource definitions.
    pub fn new(resources: &'a [ResourceDefinition]) -> Self {
        Self { resources }
    }

    /// Read a resource by sending the request to the launcher core.
    pub(crate) async fn read_resource(sender: Sender<McpCommand>, uri: String, mime_type: String) -> Result<ReadResourceOutput, McpError> {
        let (response_tx, response_rx) = oneshot::channel::<Result<String, String>>();
        sender
            .try_send(
                CommandResponseWrapper::builder()
                    .params(ReadResourceParams::builder().uri(uri).build())
                    .response(response_tx)
                    .build()
                    .into(),
            )
            .map_err(|e| McpError::ChannelSend(e.to_string()))?;
        match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
            Ok(Ok(Ok(contents))) => Ok(ReadResourceOutput { contents, mime_type }),
            Ok(Ok(Err(e))) => Err(McpError::LauncherError(e)),
            Ok(Err(_)) => Err(McpError::ChannelClosed),
            Err(_) => Err(McpError::Timeout),
        }
    }

    /// Read a core resource by URI and return a `ReadResourceOutput` for the SDK
    /// ServerHandler. Returns Err(McpError) on failure.
    pub async fn read_sdk(&self, sender: Sender<McpCommand>, uri: &str) -> Result<ReadResourceOutput, McpError> {
        let Some(resource) = self.resources.iter().find(|r| r.uri == uri) else {
            return Err(McpError::ResourceNotFound(uri.to_string()));
        };
        let mime_type = resource.mime_type.clone();
        match (resource.handler)(sender, uri.to_string()).await {
            Ok(contents) => Ok(ReadResourceOutput { contents, mime_type }),
            Err(e) => Err(e),
        }
    }

    /// Read a resource by URI and return a JSON-RPC response.
    pub async fn read_response(&self, sender: Sender<McpCommand>, id: Option<Value>, uri: String) -> JsonRpcResponse {
        let Some(resource) = self.resources.iter().find(|r| uri == r.uri) else {
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
            Err(e) => JsonRpcResponse::error(id, JSONRPC_INTERNAL_ERROR, e.to_string(), None),
        }
    }
}
