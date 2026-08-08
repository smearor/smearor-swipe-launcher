# Short-Term Memory for the Voice Assistant LLM

This document describes the concept for a **short-term memory** system that allows the Voice Assistant LLM to retain context across multiple voice commands. It
combines conversation history for pronoun resolution with an entity state store for device status persistence.

---

## 1. Problem Statement

### Current Behavior

Every `execute_react_loop` call starts with a fresh conversation:

```rust
// react.rs:72
let mut conversation = vec![
    LlamaChatMessage::new("user".to_string(), user_text.to_string())?,
];
```

There is no memory of previous commands. This leads to two failure modes:

1. **Pronoun resolution fails**: User says "Schalte den Ventilator ein", then "Mach ihn wieder aus". The LLM does not know what "ihn" refers to.
2. **Device state unknown**: User says "Ist der Ventilator an?". The LLM has no way to query the current state of devices it has previously controlled.

### Required Capabilities

| Capability          | Example                               | Memory Type                       |
|---------------------|---------------------------------------|-----------------------------------|
| Pronoun resolution  | "Mach **ihn** aus" → fan              | Conversation history              |
| Device state query  | "Läuft der **Ventilator**?"           | Entity state store                |
| Multi-step chaining | "Schalte alles im Büro aus"           | Entity state store + tool catalog |
| Context after reset | "Mach ihn aus" (after KV-cache reset) | Entity state store (persistent)   |

---

## 2. Architecture Overview

```
+---------------------+     +--------------------------+     +-----------------------+
| Voice Assistant     |---->| Conversation History     |---->| LLM System Prompt     |
| Service             |     | (in-memory, trimmed)     |     | (injected as context) |
|                     |     +--------------------------+     +-----------------------+
|                     |     +--------------------------+     +-----------------------+
|                     |---->| Entity State Store       |---->| MCP Resource          |
|                     |     | (in-memory, persistent)  |     | memory://entities     |
|                     |     +--------------------------+     +-----------------------+
|                     |              |  ^
|                     |              v  |
|                     |     +--------------------------+
|                     |---->| Tool Response Parser     |
|                     |     | (extracts state changes) |
|                     |     +--------------------------+
+---------------------+
```

Two independent memory layers:

1. **Conversation History** — stores recent user/assistant messages for pronoun resolution and multi-turn reasoning. Volatile: lost on session reset.
2. **Entity State Store** — stores device states (on/off, last action, timestamp). Persistent: survives session resets and is queryable via MCP resource.

---

## 3. Layer 1: Conversation History

### Data Structure

```rust
/// Recent conversation messages retained across pipeline runs.
/// Trimmed to a maximum number of messages to prevent context overflow.
pub type ConversationHistory = Arc<RwLock<Vec<LlamaChatMessage>>>;
```

### Service Integration

Add a new field to `VoiceAssistantService`:

```rust
pub conversation_history: ConversationHistory,
```

### ReAct Loop Adaptation

In `execute_react_loop`, load the history before starting and save it after completion:

```rust
pub async fn execute_react_loop(&self, user_text: &str) -> Result<String, AssistantError> {
    let system_prompt = self.build_system_prompt();

    // Load history and append new user message.
    let mut conversation = self
        .conversation_history
        .read()
        .map(|h| h.clone())
        .unwrap_or_default();

    conversation.push(
        LlamaChatMessage::new("user".to_string(), user_text.to_string())
            .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
    );

    // ... run ReAct loop (tool calls, iterations) ...

    // After final answer: append assistant message and save.
    conversation.push(
        LlamaChatMessage::new("assistant".to_string(), final_answer.clone())
            .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
    );

    // Trim to last N messages.
    const MAX_HISTORY_MESSAGES: usize = 10;
    if conversation.len() > MAX_HISTORY_MESSAGES {
        let start = conversation.len() - MAX_HISTORY_MESSAGES;
        conversation = conversation.split_off(start);
    }

    if let Ok(mut history) = self.conversation_history.write() {
        *history = conversation;
    }

    Ok(final_answer)
}
```

### Trimming Strategy

The trim limit of 10 messages is chosen based on:

- **System prompt**: ~2000 tokens (with 8000 char tool catalog)
- **10 messages**: ~500-1000 tokens (assuming ~50-100 tokens per message)
- **Generation budget**: ~256 tokens
- **Total**: ~2756-3256 tokens, well within `n_ctx = 8192`

The trim limit should be configurable via `VoiceAssistantServiceConfig`:

```rust
/// Maximum number of conversation messages to retain in short-term memory.
pub max_history_messages: usize,  // default: 10
```

### What Is Stored

Only user and assistant messages are stored. Tool results are not persisted in the history (they are transient within a single ReAct loop). This keeps the
history compact and focused on the dialog.

### Interaction with Persistent LLM Session

When combined with the persistent LLM worker thread (see `SPEAK_SWIPER_CONTEXT_OVERFLOW_CONCEPT.md`), the conversation history is passed to the worker on every
`generate()` call. The worker's delta-only decode mechanism ensures that only new messages are processed, while the KV cache retains the older messages. When
the worker auto-resets (context overflow or system prompt change), the full history is re-processed from scratch.

---

## 4. Layer 2: Entity State Store

### Data Model

```rust
/// Represents the state of a controllable entity (e.g., a smart home device).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityState {
    /// Human-readable name of the entity (e.g., "Ventilator").
    pub name: String,
    /// Current state (e.g., "on", "off", "open", "closed").
    pub state: String,
    /// Tool name that controls this entity (e.g., "button_shelly_fan_button").
    pub tool: String,
    /// Last action performed (e.g., "click", "longpress").
    pub last_action: String,
    /// ISO 8601 timestamp of the last state change.
    pub last_changed: String,
}

/// In-memory store of entity states, keyed by entity identifier.
pub type EntityStore = Arc<RwLock<HashMap<String, EntityState>>>;
```

### Service Integration

Add a new field to `VoiceAssistantService`:

```rust
pub entity_store: EntityStore,
```

### Automatic State Extraction

After each successful tool invocation, the service parses the tool name and arguments to extract state changes. The parsing logic uses the tool name prefix to
determine the entity type:

```rust
/// Extracts entity state from a tool call and its result.
fn extract_entity_state(tool_name: &str, arguments: &serde_json::Value) -> Option<EntityState> {
    // Button tools: "button_<plugin_id>"
    if let Some(plugin_id) = tool_name.strip_prefix("button_") {
        let action = arguments.get("action").and_then(|v| v.as_str()).unwrap_or("click");
        let state = match action {
            "click" => "on",
            "longpress" => "off",
            _ => return None,
        };
        return Some(EntityState {
            name: plugin_id.replace('_', " "),
            state: state.to_string(),
            tool: tool_name.to_string(),
            last_action: action.to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    // App launcher tools: "app_launcher_exec" / "app_launcher_terminate"
    if tool_name == "app_launcher_exec" {
        let desktop_file = arguments.get("desktop_file").and_then(|v| v.as_str())?;
        let app_name = std::path::Path::new(desktop_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        return Some(EntityState {
            name: app_name.to_string(),
            state: "running".to_string(),
            tool: tool_name.to_string(),
            last_action: "exec".to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }
    if tool_name == "app_launcher_terminate" {
        let desktop_file = arguments.get("desktop_file").and_then(|v| v.as_str())?;
        let app_name = std::path::Path::new(desktop_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        return Some(EntityState {
            name: app_name.to_string(),
            state: "stopped".to_string(),
            tool: tool_name.to_string(),
            last_action: "terminate".to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    None
}
```

### State Update After Tool Call

In the ReAct loop, after a successful tool invocation:

```rust
let tool_result = self .invoke_tool(tool_name, & arguments).await?;

// Update entity store if this was a state-changing tool.
if let Some( mut entity_state) = Self::extract_entity_state(tool_name, & arguments) {
// Try to get a better name from the tool description.
if let Ok(catalog) = self.tool_catalog.read() {
if let Some(entry) = catalog.iter().find( | t | t.name == tool_name) {
entity_state.name = entry.description.clone();
}
}
let key = tool_name.to_string();
if let Ok( mut store) = self.entity_store.write() {
store.insert(key, entity_state);
}
}
```

### MCP Resource: `memory://entities`

The entity store is exposed as an MCP resource, allowing the LLM to query it via the standard resource invocation mechanism.

**Registration** (in `register_mcp_capabilities`):

```rust
let entity_resource = RegisterResourceMessage::new(
"memory://entities",
"Entity States",
"Current states of all controllable entities (devices, applications). \
     Use this to check if a device is on or off before performing actions.",
"application/json",
);
broadcaster.broadcast_message_to_topic(entity_resource);
```

**Resource Handler** (in `MessageHandler<InvokeResourceMessage>`):

```rust
"memory://entities" => {
let store = self.entity_store.read().unwrap_or_else( | e | e.into_inner());
let json = serde_json::json !({
"entities": store.values().collect::< Vec < _ > > (),
});
InvokeResourceResponse::success( & message.0.correlation_id, & json.to_string())
}
```

### MCP Tool: `memory_query`

For targeted queries, a dedicated MCP tool allows the LLM to query a specific entity:

**Registration:**

```rust
let memory_query_tool = RegisterToolMessage::new(
"memory_query",
"Query the current state of a controllable entity (device or application). \
     Returns the state, last action, and timestamp. \
     Use this to check if a device is on or off.",
r#"{ "type": "object", "properties": { "entity": { "type": "string", "description": "Entity name or tool name to query (e.g., 'fan', 'button_shelly_fan_button')" } }, "required": ["entity"] }"#,
);
broadcaster.broadcast_message_to_topic(memory_query_tool);
```

**Tool Handler:**

```rust
"memory_query" => {
let args: serde_json::Value = serde_json::from_str( & message.0.arguments.to_string())
.unwrap_or(serde_json::Value::Null);
let query = args.get("entity").and_then( | v | v.as_str()).unwrap_or("");

let store = self.entity_store.read().unwrap_or_else( | e | e.into_inner());

// Match by tool name (exact) or by entity name (case-insensitive substring).
let result = store
.iter()
.find( | (key, state) | {
* key == query
| | state.name.to_lowercase().contains( & query.to_lowercase())
})
.map( | (_, state)| serde_json::json ! ({
"name": state.name,
"state": state.state,
"tool": state.tool,
"last_action": state.last_action,
"last_changed": state.last_changed,
}));

let response = match result {
Some(entity) => InvokeToolResponse::success(
&message.0.correlation_id,
& entity.to_string(),
),
None => InvokeToolResponse::success(
& message.0.correlation_id,
"Entity not found in memory",
),
};
broadcaster.broadcast_message_to_topic(response);
}
```

---

## 5. System Prompt Injection

### Entity Context in System Prompt

To give the LLM immediate awareness of device states without requiring a tool call, a summary of known entities is injected into the system prompt:

```rust
pub fn build_system_prompt(&self) -> String {
    // ... existing tool catalog serialization ...

    // Inject entity states.
    let entity_summary = self.build_entity_summary();
    let entity_section = if entity_summary.is_empty() {
        String::new()
    } else {
        format!("\nKnown device states:\n{entity_summary}")
    };

    let template = self.config.system_prompt.as_deref().unwrap_or(DEFAULT_PROMPT);
    template
        .replace("{tools}", &serialized)
        .replace("{entities}", &entity_section)
}
```

### Entity Summary Builder

```rust
fn build_entity_summary(&self) -> String {
    let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
    if store.is_empty() {
        return String::new();
    }
    store
        .values()
        .map(|state| format!("- {}: {}", state.name, state.state))
        .collect::<Vec<_>>()
        .join("\n")
}
```

### Example System Prompt

```
Du bist ein Smart-Home-Assistent. Verfügbare Tools: [{"name": "button_shelly_fan_button", ...}].
Known device states:
- Ventilator: on
- Schreibtischlampe: off
Antworte in JSON. Tool-Aufruf: {"tool": "<name>", "arguments": {...}}. Finale Antwort: {"final_answer": "<text>"}.
```

### Prompt Template Placeholder

The system prompt template gains a new `{entities}` placeholder:

```toml
[voice_assistant]
system_prompt = "Du bist ein Smart-Home-Assistent. Verfügbare Tools: {tools}.{entities} Antworte in JSON. ..."
```

If `{entities}` is not present in the template, the entity summary is not injected (backward compatible).

---

## 6. Configuration

New fields in `VoiceAssistantServiceConfig`:

```rust
/// Maximum number of conversation messages to retain in short-term memory.
pub max_history_messages: usize,  // default: 10

/// Whether to inject entity states into the system prompt.
pub inject_entity_states: bool,   // default: true
```

**TOML usage:**

```toml
[voice_assistant]
max_history_messages = 10
inject_entity_states = true
```

---

## 7. Interaction with Persistent LLM Session

When the persistent LLM worker (see `SPEAK_SWIPER_CONTEXT_OVERFLOW_CONCEPT.md`) is implemented, the memory layers interact as follows:

### Conversation History

- The conversation history is passed to `worker.generate()` on every call.
- The worker's delta-only decode processes only new messages.
- On auto-reset (context overflow), the full history is re-processed.
- After reset, the trimmed history (last 10 messages) is re-injected, maintaining continuity.

### Entity State Store

- The entity store is **independent** of the LLM session.
- It survives session resets completely.
- The entity summary is re-injected into the system prompt on every `build_system_prompt` call.
- After a reset, the system prompt (including entity states) is re-processed as part of the fresh session.

### Combined Flow

```
Command 1: "Schalte den Ventilator ein"
  → System prompt: {tools} + {entities: empty}
  → LLM calls: button_shelly_fan_button(action=click)
  → Entity store updated: fan → on
  → Conversation history: [user: "Schalte...", assistant: "Ventilator eingeschaltet"]

Command 2: "Mach ihn wieder aus"
  → System prompt: {tools} + {entities: "Ventilator: on"}
  → Conversation history loaded: [user: "Schalte...", assistant: "Ventilator eingeschaltet"]
  → New user message appended: "Mach ihn wieder aus"
  → LLM resolves "ihn" → Ventilator (from history + entity states)
  → LLM calls: button_shelly_fan_button(action=longpress)
  → Entity store updated: fan → off
  → Conversation history: [..., user: "Mach ihn...", assistant: "Ventilator ausgeschaltet"]

Command 3 (after session reset): "Ist der Ventilator an?"
  → System prompt: {tools} + {entities: "Ventilator: off"}
  → Conversation history: trimmed to last 10 messages
  → LLM answers directly: "Nein, der Ventilator ist aus." (no tool call needed)
```

---

## 8. Entity State Inference

### Button Tools

Button tools follow the naming convention `button_<plugin_id>`. The action parameter determines the state:

| Action       | Inferred State |
|--------------|----------------|
| `click`      | `on`           |
| `longpress`  | `off`          |
| `swipe_up`   | `increasing`   |
| `swipe_down` | `decreasing`   |

### App Launcher Tools

| Tool                     | Inferred State |
|--------------------------|----------------|
| `app_launcher_exec`      | `running`      |
| `app_launcher_terminate` | `stopped`      |

### Extensibility

The `extract_entity_state` function can be extended for additional tool types:

- `audio_set_volume` → entity: "volume", state: the volume value
- `mpris_play` / `mpris_pause` → entity: "media", state: "playing" / "paused"
- `network_toggle_vpn` → entity: "vpn", state: "active" / "inactive"

Each new tool type adds a matching branch in `extract_entity_state`.

---

## 9. Token Budget Analysis

### Conversation History

With `max_history_messages = 10` and an average of 50 tokens per message:

- 10 messages × 50 tokens = 500 tokens
- This is ~6% of `n_ctx = 8192`

### Entity States in System Prompt

With 10 entities and an average of 10 tokens per entity line:

- 10 entities × 10 tokens = 100 tokens
- This is ~1.2% of `n_ctx = 8192`

### Combined Memory Overhead

- Conversation history: ~500 tokens
- Entity states in prompt: ~100 tokens
- Total: ~600 tokens (~7.3% of context)
- Remaining for tool catalog + generation: ~7592 tokens

This is well within budget, even with the 8000-character tool catalog (~2000 tokens).

---

## 10. Testing Strategy

### Unit Tests

- **Conversation history trimming**: Verify only last N messages are retained.
- **Entity state extraction**: Test `extract_entity_state` for button tools, app launcher tools, and unknown tools.
- **Entity state query**: Test `memory_query` tool handler with exact match and fuzzy match.
- **Entity resource**: Test `memory://entities` resource returns correct JSON.
- **System prompt injection**: Verify `{entities}` placeholder is replaced correctly.

### Integration Tests

- **Multi-turn dialog**: "Schalte den Ventilator ein" → "Mach ihn aus" → verify second command resolves pronoun.
- **State persistence after reset**: Trigger session reset → "Ist der Ventilator an?" → verify correct answer from entity store.
- **Entity state update**: Call button tool → verify entity store updated → query `memory://entities`.
- **Empty entity store**: Verify system prompt does not include empty entity section.

### Manual Verification

- Issue 5 sequential commands and verify conversation history grows correctly.
- Check log output for entity state updates after tool calls.
- Verify `memory_query` tool appears in tool catalog.

---

## 11. Affected Files

| File                                                     | Change                                                                                                |
|----------------------------------------------------------|-------------------------------------------------------------------------------------------------------|
| `services/voice_assistant/src/config.rs`                 | Add `max_history_messages` and `inject_entity_states` fields.                                         |
| `services/voice_assistant/src/service/loaded_service.rs` | Add `conversation_history` and `entity_store` fields. Initialize in `new()`. Clone in pipeline spawn. |
| `services/voice_assistant/src/react.rs`                  | Load/save conversation history in `execute_react_loop`. Extract entity state after tool calls.        |
| `services/voice_assistant/src/mcp.rs`                    | Register `memory://entities` resource and `memory_query` tool. Add resource and tool handlers.        |
| `services/voice_assistant/src/tool_catalog.rs`           | Add `{entities}` placeholder replacement in `build_system_prompt`. Add `build_entity_summary` method. |
| `services/voice_assistant/Cargo.toml`                    | Add `chrono` dependency for timestamps (if not already present).                                      |

---

## 12. Dependencies

### New Crate Dependencies

- **`chrono`**: For ISO 8601 timestamps in entity states. Already used by other services (weather, clock).

### No New Model Crate

The entity state types are internal to the voice assistant service. They do not need to be shared via FFI, so no `model/` crate is required. If other services
need to query entity states in the future, the types can be moved to `model/voice_assistant/` with `#[stabby::stabby]` annotations.

---

## 13. Migration Path

The implementation is backward-compatible and can be done in three incremental steps:

### Step 1: Conversation History (Layer 1)

1. Add `conversation_history` field to `VoiceAssistantService`.
2. Load/save history in `execute_react_loop`.
3. Add `max_history_messages` config field.
4. Test: multi-turn dialog with pronoun resolution.

### Step 2: Entity State Store (Layer 2)

1. Add `entity_store` field to `VoiceAssistantService`.
2. Implement `extract_entity_state` function.
3. Call it after successful tool invocations in the ReAct loop.
4. Test: verify entity store updates after button tool calls.

### Step 3: MCP Integration

1. Register `memory://entities` resource and `memory_query` tool.
2. Add resource and tool handlers in `mcp.rs`.
3. Add `{entities}` placeholder to `build_system_prompt`.
4. Add `inject_entity_states` config field.
5. Test: LLM queries entity states via resource or tool.

Each step is independently functional and can be deployed separately.
