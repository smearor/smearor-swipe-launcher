//! Shared registry for MCP tools and resources.
//!
//! The launcher host populates the registry by processing registration messages
//! from plugins. The MCP server reads from the same registry to expose dynamic
//! tools and resources to external clients.

use crate::RegisterPromptMessage;
use crate::RegisterResourceMessage;
use crate::RegisterToolMessage;
use crate::RegisteredPrompt;
use crate::RegisteredResource;
use crate::RegisteredTool;
use crate::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::sync::Arc;
use std::sync::Mutex;

/// Shared registry for MCP tools and resources.
#[derive(Clone)]
pub struct McpRegistry {
    inner: Arc<Mutex<McpRegistryInner>>,
}

#[derive(Default)]
struct McpRegistryInner {
    tools: Vec<RegisteredTool>,
    resources: Vec<RegisteredResource>,
    prompts: Vec<RegisteredPrompt>,
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

    /// Register or replace a prompt.
    pub fn register_prompt(&self, prompt: RegisteredPrompt) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.prompts.retain(|p| p.name != prompt.name);
        inner.prompts.push(prompt);
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

    /// Return a snapshot of all registered prompts.
    pub fn list_prompts(&self) -> Vec<RegisteredPrompt> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.prompts.clone()
    }

    /// Remove all tools registered by a specific instance.
    pub fn remove_tools_by_instance(&self, instance_id: &str) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let before = inner.tools.len();
        inner.tools.retain(|t| !t.plugin_id.starts_with(&format!("{}:", instance_id)));
        let removed = before - inner.tools.len();
        if removed > 0 {
            eprintln!("Removed {} MCP tools from instance '{}'", removed, instance_id);
        }
    }

    /// Remove all resources registered by a specific instance.
    pub fn remove_resources_by_instance(&self, instance_id: &str) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.resources.retain(|r| !r.plugin_id.starts_with(&format!("{}:", instance_id)));
    }

    /// Remove all prompts registered by a specific instance.
    pub fn remove_prompts_by_instance(&self, instance_id: &str) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.prompts.retain(|p| !p.plugin_id.starts_with(&format!("{}:", instance_id)));
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
        let title = message.0.title.as_ref().map(|t| t.to_string());
        let annotations: Option<ToolAnnotations> = message.0.annotations.as_ref().and_then(|a| serde_json::from_str(&a.to_string()).ok());
        let tool = RegisteredTool {
            name: message.0.name.to_string(),
            description: message.0.description.to_string(),
            input_schema: schema,
            plugin_id: sender_id.to_string(),
            title,
            annotations,
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

impl MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for McpRegistry {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterPromptMessage>, sender_id: &str) {
        let schema = serde_json::from_str(&message.0.arguments_schema.to_string()).unwrap_or(serde_json::Value::Null);
        let prompt = RegisteredPrompt {
            name: message.0.name.to_string(),
            description: message.0.description.to_string(),
            arguments_schema: schema,
            plugin_id: sender_id.to_string(),
            requires_memory: message.0.requires_memory,
            memory_query: message.0.memory_query.to_string(),
            entity_filter: message.0.entity_filter.to_string(),
        };
        self.register_prompt(prompt);
    }
}
