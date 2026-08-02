# Registering MCP Tools

Plugins and services can expose MCP (Model Context Protocol) tools, resources, and prompts. These are used by the voice assistant and external AI clients to
interact with the launcher.

## Registering a Tool

Send a `RegisterToolMessage` during plugin initialization:

```rust
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

// In your plugin's start() or init handler:
self.broadcast_message(
    "mcp.register.tool",
    &RegisterToolMessage::new(
        "my_tool",
        "Does something useful",
        r#"{"type":"object","properties":{"param":{"type":"string"}}}"#,
    ),
);
```

## Handling Tool Invocations

Implement `MessageHandler` for `InvokeToolMessage`:

```rust
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for MyPlugin {
    fn handle_message(&self, msg: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let name = msg.name.to_string();
        let arguments = msg.arguments.to_string();
        let correlation_id = msg.correlation_id.to_string();

        let result = match name.as_str() {
            "my_tool" => {
                // Execute the tool
                Ok("Tool executed successfully".to_string())
            }
            _ => Err("Unknown tool".to_string()),
        };

        let response = match result {
            Ok(text) => InvokeToolResponse::success(&correlation_id, &text),
            Err(err) => InvokeToolResponse::error(&correlation_id, &err),
        };

        self.broadcast_message("mcp.tool.response", &response);
    }
}
```

## Registering a Resource

```rust
use smearor_model_mcp::RegisterResourceMessage;

self.broadcast_message(
    "mcp.register.resource",
    &RegisterResourceMessage::new(
        "my_resource",
        "My Resource",
        "text/plain",
        "Description of the resource",
    ),
);
```

Handle `InvokeResourceMessage` similarly to tool invocations.

## Registering a Prompt

```rust
use smearor_model_mcp::RegisterPromptMessage;

self.broadcast_message(
    "mcp.register.prompt",
    &RegisterPromptMessage::new(
        "my_prompt",
        "A helpful prompt template",
        "Description of the prompt",
    ),
);
```

Handle `InvokePromptMessage` by returning a list of `PromptMessage` entries.

## MCP Capabilities Registrator

For convenience, the `McpCapabilitiesRegistrator` trait provides a unified API:

```rust
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;

impl McpCapabilitiesRegistrator for MyPlugin {}
```

This trait provides helper methods for registering tools, resources, and prompts with less boilerplate.

## Routing

The host routes MCP invocations based on the `McpRegistry`:

1. `mcp.invoke.tool` → Look up tool name in registry → find owning plugin → route
2. `mcp.invoke.resource` → Look up URI in registry → find owning plugin → route
3. `mcp.invoke.prompt` → Look up prompt name in registry → find owning plugin → route

If no plugin owns the invocation, the host falls back to core MCP capabilities.

See [MCP Server and AI Integration](../features/mcp-server.md) for the feature overview.
