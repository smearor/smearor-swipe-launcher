# Concept: MCP Prompts for Smearor Swipe Launcher

This document describes the concept for extending the MCP server with **Prompts** — the third capability alongside Tools and Resources. Prompts allow service
and widget plugins to provide reusable, parameterized instruction templates that external AI clients (e.g., Claude Desktop, Cursor) and the internal Voice
Assistant can retrieve and inject into LLM conversations.

---

## 1. Goal & Motivation

The MCP protocol defines three capability types: **Tools** (callable functions), **Resources** (readable state), and **Prompts** (parameterized instruction
templates). The Smearor Swipe Launcher currently implements Tools and Resources. Adding Prompts completes the MCP capability triad and enables:

* **Reusable instruction templates:** Services can expose pre-defined prompts that guide AI clients through complex multi-step operations (e.g., "Set up a new
  wallpaper theme", "Configure a terminal command").
* **Context-rich templates:** Prompts can include dynamic context from the plugin's current state, such as available players, configured locations, or running
  commands.
* **Voice Assistant integration:** The Voice Assistant can discover and use prompts to improve its system prompt or provide task-specific instructions to the
  LLM.
* **External client discovery:** MCP clients like Claude Desktop can list and use prompts to interact with the launcher more effectively.

---

## 2. Architecture

Prompts follow the same plugin-driven registration pattern as Tools and Resources. Each plugin (service or widget) registers its prompts via the message broker.
The `McpRegistry` stores prompt definitions, and the MCP server exposes them through the standard MCP `prompts/list` and `prompts/get` endpoints.

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP CLIENT                              │
│  (e.g., Claude Desktop, Cursor, Voice Assistant)            │
└─────────────────────┬───────────────────────┬───────────────┘
                      │ JSON-RPC / MCP over Streamable HTTP + SSE │
                      ▼                         ▼
┌──────────────────────────────────────────────────┐  ┌────────────────────────┐
│   MCP Server (rust-mcp-sdk + axum)               │  │  Prompt Registry        │
│  ┌────────────────────────────────────────────┐  │  │  (McpRegistry +         │
│  │   prompts/list, prompts/get                │  │  │   Plugin handlers)      │
│  └────────────────────────────────────────────┘  │  └────────────────────────┘
│  ┌────────────────────────────────────────────┐  │
│  │   Notifications: prompts/list_changed      │  │
│  └────────────────────────────────────────────┘  │
└─────────────────────┬────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                 Smearor Swipe Launcher Core                  │
│  ┌─────────────────────┐    ┌─────────────────────────────┐│
│  │   McpRegistry       │    │   Central Message Broker      ││
│  │   (tools, resources,│    │   (publish/subscribe)         ││
│  │    prompts)         │    │                               ││
│  └─────────────────────┘    └─────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## 3. MCP Protocol Integration

### 3.1 Server Capabilities

The MCP server already advertises `prompts` in its `ServerCapabilities`:

```rust
capabilities: ServerCapabilities {
prompts: Some(rust_mcp_sdk::schema::ServerCapabilitiesPrompts {
list_changed: Some(true),
}),
..
}
```

The `handle_list_prompts_request` currently returns an empty list. This concept implements the full prompt lifecycle.

### 3.2 MCP Endpoints

| Method         | Description                                           |
|----------------|-------------------------------------------------------|
| `prompts/list` | Returns all registered prompts (core + plugin).       |
| `prompts/get`  | Returns the resolved prompt content for a given name. |

### 3.3 Notifications

| Notification                         | Trigger                          | Payload |
|--------------------------------------|----------------------------------|---------|
| `notifications/prompts/list_changed` | A plugin registers a new prompt. | `{}`    |

---

## 4. Model Crate (`model/mcp`)

### 4.1 New Message Types

Three new message types are added alongside `RegisterToolMessage` and `RegisterResourceMessage`:

#### RegisterPromptMessage

Sent by a plugin to register a prompt with the MCP server.

```rust
/// Message sent by a plugin to register a prompt with the MCP server.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RegisterPromptMessage {
    /// Unique prompt name exposed to MCP clients.
    pub name: stabby::string::String,
    /// Human-readable description of the prompt.
    pub description: stabby::string::String,
    /// Optional JSON schema for the prompt's arguments.
    /// If empty, the prompt takes no arguments.
    pub arguments_schema: stabby::string::String,
}

impl RegisterPromptMessage {
    /// Create a new prompt registration message.
    pub fn new(name: &str, description: &str, arguments_schema: &str) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments_schema: arguments_schema.into(),
        }
    }
}
```

#### InvokePromptMessage

Sent by the host to a plugin to resolve a prompt with arguments.

```rust
/// Request sent by the host to a plugin to resolve a registered prompt.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokePromptMessage {
    /// Prompt name as registered by the plugin.
    pub name: stabby::string::String,
    /// Correlation ID used to match the response.
    pub correlation_id: stabby::string::String,
    /// JSON-encoded arguments for the prompt, matching the arguments schema.
    pub arguments: stabby::string::String,
}

impl InvokePromptMessage {
    /// Create a new prompt invocation request.
    pub fn new(name: &str, correlation_id: &str, arguments: &str) -> Self {
        Self {
            name: name.into(),
            correlation_id: correlation_id.into(),
            arguments: arguments.into(),
        }
    }
}
```

#### InvokePromptResponse

Returned by the plugin after resolving a prompt. The content is a list of MCP message roles (system, user, assistant) that the client injects into the
conversation.

```rust
/// A single message within a resolved prompt.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct PromptMessage {
    /// Role of the message: "system", "user", or "assistant".
    pub role: stabby::string::String,
    /// Content of the message.
    pub content: stabby::string::String,
}

/// Response returned by a plugin after resolving a registered prompt.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct InvokePromptResponse {
    /// Correlation ID matching the request.
    pub correlation_id: stabby::string::String,
    /// Resolved prompt messages. Empty on error.
    pub messages: stabby::vec::Vec<PromptMessage>,
    /// Error message. Empty when the resolution succeeded.
    pub error: stabby::string::String,
}

impl InvokePromptResponse {
    /// Create a successful prompt response.
    pub fn success(correlation_id: &str, messages: Vec<PromptMessage>) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            messages: messages.into(),
            error: "".into(),
        }
    }

    /// Create an error prompt response.
    pub fn error(correlation_id: &str, error: &str) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            messages: stabby::vec::Vec::new(),
            error: error.into(),
        }
    }
}
```

### 4.2 New Topics

```rust
pub const TOPIC_MCP_REGISTER_PROMPT: &str = "mcp.register.prompt";
pub const TOPIC_MCP_INVOKE_PROMPT: &str = "mcp.invoke.prompt";
pub const TOPIC_MCP_PROMPT_RESPONSE: &str = "mcp.prompt.response";
```

### 4.3 Registry Extension

The `McpRegistry` in `model/mcp/src/registry.rs` is extended with a `RegisteredPrompt` struct and corresponding storage:

```rust
/// Description of a prompt registered by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredPrompt {
    /// Unique prompt name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON schema for the prompt's arguments. Empty object if no arguments.
    pub arguments_schema: serde_json::Value,
    /// ID of the plugin that registered the prompt.
    pub plugin_id: String,
}
```

The `McpRegistryInner` struct gains a `prompts: Vec<RegisteredPrompt>` field. The registry exposes:

- `register_prompt(&self, prompt: RegisteredPrompt)` — Register or replace a prompt by name.
- `list_prompts(&self) -> Vec<RegisteredPrompt>` — Return a snapshot of all registered prompts.
- `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>>` — Auto-registers prompts from broker messages.

---

## 5. MCP Server (`mcp-server`)

### 5.1 McpCommand Extension

A new command variant is added to `McpCommand`:

```rust
/// Invoke a prompt registered by a plugin.
InvokePluginPrompt {
name: String,
plugin_id: String,
correlation_id: String,
arguments: serde_json::Value,
response: oneshot::Sender<Result<String, String> >,
},
```

### 5.2 Handler Implementation

The `SwipeLauncherHandler` is extended:

#### `handle_list_prompts_request`

Returns core prompts (from `mcp-server/src/prompts.rs`) plus all plugin-registered prompts from the `McpRegistry`.

```rust
async fn handle_list_prompts_request(
    &self,
    _params: Option<PaginatedRequestParams>,
    _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
) -> Result<ListPromptsResult, RpcError> {
    let state = self.state.clone();
    let mut sdk_prompts: Vec<Prompt> = state
        .prompts
        .iter()
        .map(|p| /* convert core prompt to SDK Prompt */)
        .collect();

    for plugin_prompt in state.plugin_registry.list_prompts() {
        sdk_prompts.push(Prompt {
            name: plugin_prompt.name.clone(),
            description: Some(plugin_prompt.description.clone()),
            arguments: /* parse arguments_schema into Vec<PromptArgument> */,
            annotations: None,
            meta: None,
            title: None,
        });
    }

    Ok(ListPromptsResult {
        prompts: sdk_prompts,
        next_cursor: None,
        meta: None,
    })
}
```

#### `handle_get_prompt_request`

Resolves a prompt by name. If the prompt is a plugin prompt, the host sends an `InvokePromptMessage` to the owning plugin and waits for the
`InvokePromptResponse` (with the same 10-second timeout as tools/resources).

```rust
async fn handle_get_prompt_request(
    &self,
    params: GetPromptRequestParams,
    _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
) -> Result<GetPromptResult, RpcError> {
    let state = self.state.clone();
    let name = params.name.clone();
    let arguments = params.arguments.clone();

    // Check plugin registry
    if let Some(plugin_prompt) = state.plugin_registry.list_prompts().into_iter().find(|p| p.name == name) {
        let correlation_id = state.correlation_counter.fetch_add(1, Ordering::Relaxed).to_string();
        let (response_tx, response_rx) = oneshot::channel();
        let _ = state.command_sender.try_send(McpCommand::InvokePluginPrompt {
            name: plugin_prompt.name.clone(),
            plugin_id: plugin_prompt.plugin_id.clone(),
            correlation_id,
            arguments: arguments.map(|m| serde_json::Value::Object(m)).unwrap_or(serde_json::Value::Null),
            response: response_tx,
        });

        match tokio::time::timeout(Duration::from_secs(10), response_rx).await {
            Ok(Ok(Ok(json))) => {
                // Parse JSON response into GetPromptResult
                return Ok(serde_json::from_str(&json).unwrap_or(GetPromptResult {
                    description: None,
                    messages: vec![],
                    meta: None,
                }));
            }
            Ok(Ok(Err(msg))) => return Err(RpcError::internal_error().with_message(msg)),
            Ok(Err(_)) => return Err(RpcError::internal_error().with_message("Prompt invocation dropped")),
            Err(_) => return Err(RpcError::internal_error().with_message("Prompt invocation timed out")),
        }
    }

    // Core prompt
    match prompts::get_prompt_sdk(&state.prompts, &name, &arguments) {
        Ok(result) => Ok(result),
        Err(msg) => Err(RpcError::internal_error().with_message(msg)),
    }
}
```

### 5.3 Core Prompts (`mcp-server/src/prompts.rs`)

A new module `prompts.rs` defines built-in prompts provided by the launcher core:

| Prompt Name            | Description                                                     | Arguments         |
|------------------------|-----------------------------------------------------------------|-------------------|
| `launcher_overview`    | Returns a system message describing the launcher and all areas. | –                 |
| `area_control_help`    | Returns instructions for controlling a specific area.           | `area_id: string` |
| `broker_message_guide` | Returns a guide for using `send_message` with the broker.       | –                 |

### 5.4 McpServerState Extension

```rust
pub struct McpServerState {
    pub command_sender: Sender<McpCommand>,
    pub tools: Vec<tools::ToolDefinition>,
    pub resources: Vec<resources::ResourceDefinition>,
    pub prompts: Vec<prompts::PromptDefinition>,
    pub plugin_registry: McpRegistry,
    pub correlation_counter: AtomicU64,
}
```

---

## 6. Plugin Integration

### 6.1 Registration Flow

Plugins register prompts during `register_mcp_capabilities()` by broadcasting a `RegisterPromptMessage` to the topic `mcp.register.prompt`:

```rust
pub fn register_mcp_capabilities(&self) {
    let broadcaster = self.get_broadcaster();

    // Existing tool and resource registrations...

    let prompt = RegisterPromptMessage::new(
        "weather_query_guide",
        "Returns a system prompt with weather query instructions and the configured location.",
        r#"{ "type": "object", "properties": { "include_forecast": { "type": "boolean", "description": "Whether to include forecast instructions" } } }"#,
    );
    broadcaster.broadcast_message_to_topic(prompt);
}
```

### 6.2 Invocation Handler

Plugins implement `MessageHandler<FfiEnvelopePayload<InvokePromptMessage>>` to resolve prompts:

```rust
impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        let arguments = message.0.arguments.to_string();

        let messages = match name.as_str() {
            "weather_query_guide" => {
                let config = self.config.clone();
                let include_forecast = serde_json::from_str::<serde_json::Value>(&arguments)
                    .ok()
                    .and_then(|a| a.get("include_forecast").and_then(|v| v.as_bool()))
                    .unwrap_or(true);

                let mut content = format!(
                    "You can query weather using the resource 'weather://current' \
                     (configured location: {}, {}) or by appending ?lat=<lat>&lon=<lon> for custom coordinates.",
                    config.latitude, config.longitude
                );

                if include_forecast {
                    content.push_str(" Use the 'weather_get_forecast' tool for forecasts at arbitrary coordinates.");
                }

                vec![PromptMessage {
                    role: "system".into(),
                    content: content.into(),
                }]
            }
            _ => {
                let response = InvokePromptResponse::error(&correlation_id, &format!("Unknown prompt: {name}"));
                self.send_response(response, sender_id);
                return;
            }
        };

        let response = InvokePromptResponse::success(&correlation_id, messages);
        self.send_response(response, sender_id);
    }
}
```

### 6.3 Response Routing

The host's `McpResponseTracker` (already used for tool and resource responses) is extended to handle `TOPIC_MCP_PROMPT_RESPONSE`. The `InvokePromptResponse` is
serialized to JSON and returned to the MCP server via the `oneshot` channel.

---

## 7. Voice Assistant Integration

### 7.1 Prompt Catalog

The Voice Assistant extends its catalog to track registered prompts:

```rust
pub struct PromptCatalogEntry {
    pub name: String,
    pub description: String,
    pub arguments_schema: serde_json::Value,
}
```

The `on_prompt_registered` method stores incoming `RegisterPromptMessage` entries, analogous to `on_tool_registered` and `on_resource_registered`.

### 7.2 Context Injection

In `build_context_message`, the available prompts are listed alongside tools and resources:

```rust
let prompts_json = self .serialize_prompts();
parts.push(format!("Available prompts: {prompts_json}"));
```

### 7.3 System Prompt Enhancement

The system prompt is extended with a rule for prompt usage:

```
- PROMPT USAGE: If the user's request matches a registered prompt, you may retrieve it using {"prompt": "<name>", "arguments": {<args>}}. Use prompts to obtain task-specific instructions or context before proceeding with tool calls.
```

### 7.4 Output Format Extension

A fifth output structure is added to the system prompt:

```
5. To retrieve a prompt template:
{"prompt": "<name>", "arguments": {<arguments_object>}}
```

---

## 8. Example Prompts

### 8.1 Weather Service

| Prompt Name           | Description                                                       | Arguments                   |
|-----------------------|-------------------------------------------------------------------|-----------------------------|
| `weather_query_guide` | System message with weather query instructions and location info. | `include_forecast: boolean` |

### 8.2 MPRIS Service

| Prompt Name           | Description                                                             | Arguments |
|-----------------------|-------------------------------------------------------------------------|-----------|
| `mpris_control_guide` | System message listing available players and control tool instructions. | –         |

### 8.3 Terminal Command Service

| Prompt Name              | Description                                                 | Arguments |
|--------------------------|-------------------------------------------------------------|-----------|
| `terminal_command_guide` | Lists configured terminal commands and launch instructions. | –         |

### 8.4 App Launcher Service

| Prompt Name        | Description                                                      | Arguments |
|--------------------|------------------------------------------------------------------|-----------|
| `app_launch_guide` | System message with app search and launch pipeline instructions. | –         |

### 8.5 Power Service

| Prompt Name          | Description                                            | Arguments |
|----------------------|--------------------------------------------------------|-----------|
| `power_action_guide` | Lists available power actions and safety instructions. | –         |

### 8.6 Voice Assistant

| Prompt Name              | Description                                                       | Arguments |
|--------------------------|-------------------------------------------------------------------|-----------|
| `voice_assistant_status` | Returns the current assistant status and configured capabilities. | –         |
| `memory_guide`           | Instructions for using the memory tools (store, recall, forget).  | –         |

---

## 9. Crate Structure

| Crate               | Path                        | Responsibility                                                                                                                          |
|---------------------|-----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------|
| **Model**           | `model/mcp/`                | `RegisterPromptMessage`, `InvokePromptMessage`, `InvokePromptResponse`, `PromptMessage`, topics, `RegisteredPrompt`, registry extension |
| **MCP Server**      | `mcp-server/`               | `prompts.rs` module, `PromptDefinition`, handler implementation, `McpCommand::InvokePluginPrompt`                                       |
| **Voice Assistant** | `services/voice_assistant/` | Prompt catalog, context injection, system prompt extension                                                                              |
| **Service Plugins** | `services/*/`               | Prompt registration and `InvokePromptMessage` handler per service                                                                       |
| **Widget Plugins**  | `plugins/*/`                | Optional prompt registration and handler per widget                                                                                     |

---

## 10. Roadmap

### Phase 1: Foundation — Model Crate (`model/mcp`)

**Goal:** Define all shared message types, topics, and registry extensions for prompts.

**Order:**

1. Add `RegisterPromptMessage`, `InvokePromptMessage`, `InvokePromptResponse`, and `PromptMessage` structs to `model/mcp/src/lib.rs`.
2. Add `#[stabby::stabby(no_opt)]` to all FFI-relevant types.
3. Implement `TypedMessage`, `MessageTopic`, and `SharedMessage` for each new message type.
4. Add new topic constants: `TOPIC_MCP_REGISTER_PROMPT`, `TOPIC_MCP_INVOKE_PROMPT`, `TOPIC_MCP_PROMPT_RESPONSE`.
5. Add `RegisteredPrompt` struct to `model/mcp/src/registry.rs`.
6. Extend `McpRegistryInner` with `prompts: Vec<RegisteredPrompt>`.
7. Implement `register_prompt`, `list_prompts`, and `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>>` for `McpRegistry`.
8. Re-export new types in `model/mcp/src/lib.rs`.
9. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message type.

---

### Phase 2: MCP Server — Prompt Handling (`mcp-server`)

**Goal:** Expose prompts through the MCP protocol endpoints.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create `mcp-server/src/prompts.rs` with `PromptDefinition` and `core_prompts()`.
2. Implement core prompts: `launcher_overview`, `area_control_help`, `broker_message_guide`.
3. Add `prompts: Vec<PromptDefinition>` to `McpServerState`.
4. Add `McpCommand::InvokePluginPrompt` variant.
5. Extend `send_command_and_wait` to handle the new command variant.
6. Implement `handle_list_prompts_request` to merge core and plugin prompts.
7. Implement `handle_get_prompt_request` to resolve core and plugin prompts.
8. Add `notifications/prompts/list_changed` notification when a plugin registers a prompt.
9. Add unit tests for prompt serialization and core prompt resolution.

**Exit criteria:**

- `prompts/list` returns all core and plugin-registered prompts.
- `prompts/get` resolves core prompts correctly.
- Plugin prompt invocation works with a 10-second timeout.
- `notifications/prompts/list_changed` is broadcast when a prompt is registered.

---

### Phase 3: Host Integration — Response Routing

**Goal:** Route `InvokePromptMessage` to the correct plugin and track responses.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Extend `McpResponseTracker` in `host/mod.rs` to handle `TOPIC_MCP_PROMPT_RESPONSE`.
2. Intercept `TOPIC_MCP_REGISTER_PROMPT` in `route_message` and insert into `McpRegistry`.
3. Intercept `TOPIC_MCP_INVOKE_PROMPT` and route to the target plugin instance.
4. Serialize `InvokePromptResponse` to JSON and return via `oneshot` channel.

**Exit criteria:**

- Plugin prompt registration is automatically picked up by the host.
- Plugin prompt invocation returns the resolved messages within the timeout.
- Unknown prompts return a clear error message.

---

### Phase 4: Voice Assistant Integration

**Goal:** Integrate prompts into the Voice Assistant's context and system prompt.

**Dependencies:** Phase 1, Phase 2, and Phase 3 must be complete.

**Order:**

1. Add `PromptCatalogEntry` struct and `prompt_catalog` field to `VoiceAssistantService`.
2. Implement `on_prompt_registered` to store incoming prompt registrations.
3. Add `serialize_prompts` method to format the prompt catalog as JSON.
4. Extend `build_context_message` to include available prompts.
5. Extend `build_system_prompt` with the prompt usage rule and the fifth output format.
6. Add handling for `{"prompt": "<name>", "arguments": {<args>}}` responses from the LLM.
7. When the LLM requests a prompt, retrieve it via `InvokePromptMessage` and inject the response as a system message into the next context.

**Exit criteria:**

- The Voice Assistant lists available prompts in the context message.
- The LLM can request a prompt by name and receive its content.
- Prompt content is injected as a system message in the next turn.

---

### Phase 5: Service Plugin Prompts

**Goal:** Implement prompts in service plugins.

**Dependencies:** Phase 1, Phase 2, Phase 3, and Phase 4 must be complete.

**Order:**

1. **Weather Service:** Register `weather_query_guide` prompt. Implement `InvokePromptMessage` handler.
2. **MPRIS Service:** Register `mpris_control_guide` prompt. Implement handler.
3. **Terminal Command Service:** Register `terminal_command_guide` prompt. Implement handler.
4. **App Launcher Service:** Register `app_launch_guide` prompt. Implement handler.
5. **Power Service:** Register `power_action_guide` prompt. Implement handler.
6. **Voice Assistant Service:** Register `voice_assistant_status` and `memory_guide` prompts. Implement handlers.
7. Add unit tests for each plugin's prompt handler.

**Exit criteria:**

- Each service exposes at least one prompt.
- All prompts resolve correctly when invoked.
- Prompts include relevant dynamic context (e.g., configured location, available players).

---

### Phase 6: Tests & Documentation

**Goal:** Comprehensive test coverage and documentation.

**Dependencies:** All previous phases must be complete.

**Order:**

1. Add integration test: register a prompt, list prompts via MCP, get prompt via MCP.
2. Add integration test: plugin prompt invocation with arguments.
3. Add integration test: timeout handling for unresponsive plugins.
4. Add integration test: `notifications/prompts/list_changed` notification.
5. Update `MCP_SERVER_CONCEPT.md` with prompt implementation status.
6. Update `ROADMAP.md` with completed prompt phases.

**Exit criteria:**

- All integration tests pass.
- Documentation is updated and reflects the current implementation.

---

## 11. Dependencies

* `rust-mcp-sdk` — `Prompt`, `PromptArgument`, `GetPromptResult`, `GetPromptRequestParams`, `ListPromptsResult` schema types.
* `stabby` — ABI-stable FFI types for `RegisterPromptMessage`, `InvokePromptMessage`, `InvokePromptResponse`, `PromptMessage`.
* `serde` / `serde_json` — Serialization of prompt arguments and responses.
* `tokio` — Async runtime and timeouts for prompt invocation.
* `async-channel` — `McpCommand` channel extension.
* Existing `McpRegistry` and `McpResponseTracker` infrastructure.

---

## 12. Naming Conventions

* Prompt names use `snake_case` and are service-prefixed when appropriate (e.g., `weather_query_guide`, `mpris_control_guide`).
* Core prompts (launcher-level) use no service prefix (e.g., `launcher_overview`, `area_control_help`).
* The naming convention is not enforced by the registry — each plugin chooses its prompt names.

---

## 13. Security Considerations

* Prompts are read-only instruction templates. They do not execute actions directly.
* Prompt content is generated by the plugin and returned as text. Plugins must not include sensitive data (e.g., passwords, API keys) in prompt content.
* The optional bearer token authentication (`McpServerConfig::auth_token`) applies to all MCP endpoints, including `prompts/list` and `prompts/get`.
* Prompt invocation uses the same 10-second timeout as tools and resources to prevent hanging clients.

---

*Concept for extending the Smearor Swipe Launcher MCP server with Prompts — the third MCP capability alongside Tools and Resources — enabling plugins to provide
parameterized instruction templates for AI clients and the Voice Assistant.*
