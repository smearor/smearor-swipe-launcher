//! Shared registry for MCP tools and resources.
//!
//! The launcher host populates the registry by processing registration messages
//! from plugins. The MCP server reads from the same registry to expose dynamic
//! tools and resources to external clients.

use crate::RegisterResourceMessage;
use crate::RegisterToolMessage;
use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::sync::Arc;
use std::sync::Mutex;

/// Description of a tool registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub plugin_id: String,
}

/// Description of a resource registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub plugin_id: String,
}

/// Shared registry for MCP tools and resources.
#[derive(Clone)]
pub struct McpRegistry {
    inner: Arc<Mutex<McpRegistryInner>>,
}

#[derive(Default)]
struct McpRegistryInner {
    tools: Vec<RegisteredTool>,
    resources: Vec<RegisteredResource>,
}

impl McpRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(McpRegistryInner::default())),
        }
    }

    /// Register or replace a tool.
    pub fn register_tool(&self, tool: RegisteredTool) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.tools.retain(|t| t.name != tool.name);
        inner.tools.push(tool);
    }

    /// Register or replace a resource.
    pub fn register_resource(&self, resource: RegisteredResource) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.resources.retain(|r| r.uri != resource.uri);
        inner.resources.push(resource);
    }

    /// Return a snapshot of all registered tools.
    pub fn list_tools(&self) -> Vec<RegisteredTool> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.tools.clone()
    }

    /// Return a snapshot of all registered resources.
    pub fn list_resources(&self) -> Vec<RegisteredResource> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.resources.clone()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageHandler<FfiEnvelopePayload<RegisterToolMessage>> for McpRegistry {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterToolMessage>, sender_id: &str) {
        let schema = serde_json::from_str(&message.0.input_schema.to_string()).unwrap_or(serde_json::Value::Null);
        let tool = RegisteredTool {
            name: message.0.name.to_string(),
            description: message.0.description.to_string(),
            input_schema: schema,
            plugin_id: sender_id.to_string(),
        };
        self.register_tool(tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<RegisterResourceMessage>> for McpRegistry {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterResourceMessage>, sender_id: &str) {
        let resource = RegisteredResource {
            uri: message.0.uri.to_string(),
            name: message.0.name.to_string(),
            description: message.0.description.to_string(),
            mime_type: message.0.mime_type.to_string(),
            plugin_id: sender_id.to_string(),
        };
        self.register_resource(resource);
    }
}
