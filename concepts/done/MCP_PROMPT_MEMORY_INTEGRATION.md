# Concept: Prompt-Memory Integration for the Voice Assistant

This document describes the concept for integrating MCP Prompts with the Voice Assistant's **short-term memory** (EntityStore, ConversationHistory) and
**long-term memory** (SemanticMemory). Prompts evolve from static instruction templates into **interface logic** that dynamically queries memory systems before
injection, enabling context filtering, prompt-driven memory maintenance, and pronoun resolution.

---

## 1. Goal & Motivation

### Current State

Prompts, tools, and resources are currently independent of the memory systems:

- **Prompts** are registered by plugins and listed in the context message as static JSON entries. The LLM can request a prompt by name, and the plugin returns a
  fixed instruction template.
- **SemanticMemory** is queried generically in `build_context_message` via `build_long_term_summary(user_text)` — the user's raw speech is the recall query,
  regardless of which prompts are active.
- **EntityStore** is injected as a flat list of device states, with no prompt-driven filtering.
- **ConversationHistory** is injected as recent messages, with no prompt-driven pronoun resolution.

### Problem

The LLM receives memory context that is **not tailored to the active prompts**. For example:

1. The `system_health_check` prompt instructs the LLM to read CPU and memory resources, but the SemanticMemory recall still uses the user's raw speech ("Wie ist
   der Systemstatus?") as the embedding query — missing facts like "user_threshold_cpu_80" that would be found with a targeted query like "CPU temperature
   threshold preference".
2. The `mpris_control_guide` prompt lists available players, but the EntityStore is injected as all devices — including non-media devices — adding noise.
3. When the user says "Mach ihn aus", there is no prompt-driven mechanism to resolve "ihn" via ConversationHistory or EntityStore before the LLM selects a tool.

### Required Capabilities

| Capability                       | Example                                                                         | Solution                                                       |
|----------------------------------|---------------------------------------------------------------------------------|----------------------------------------------------------------|
| Prompt-driven context filtering  | `system_health_check` → recall only CPU/temperature facts                       | `requires_memory` flag + targeted recall query                 |
| Prompt-driven memory maintenance | `memory_cleanup_assistant` → analyze entity history, summarize habits           | Prompt handler with memory write access                        |
| Pronoun resolution via memory    | "Mach ihn aus" → resolve "ihn" from EntityStore                                 | `resolve_entity_intent` prompt + EntityStore query             |
| Dynamic context enrichment       | `weather_context_guide` → inject user's weather preferences from SemanticMemory | `requires_memory` + semantic recall with prompt-specific query |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                     Voice Assistant Service                          │
│                                                                      │
│  ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐ │
│  │ Prompt Catalog  │    │  Memory Systems  │    │ Context Builder │ │
│  │                 │    │                  │    │                 │ │
│  │ PromptCatalog   │───▶│  SemanticMemory  │───▶│ build_context   │ │
│  │ Entry           │    │  EntityStore     │    │ _message()      │ │
│  │ +requires_memory│    │  ConvHistory     │    │                 │ │
│  │ +memory_query   │    │                  │    │ For each prompt │ │
│  │ +entity_filter  │    └──────────────────┘    │ with            │ │
│  │                 │                            │ requires_memory:│ │
│  └─────────────────┘                            │ 1. Build query  │ │
│                                                 │ 2. Query memory │ │
│                                                 │ 3. Inject result│ │
│                                                 └─────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User says: "Wie ist der Systemstatus?"

1. build_context_message("Wie ist der Systemstatus?")
2. Iterate prompt_catalog:
   - system_health_check: requires_memory=true, memory_type=Semantic, query="CPU temperature threshold preference"
   - mpris_control_guide: requires_memory=false
   - weather_context_guide: requires_memory=true, memory_type=Semantic, query="weather location preference"
3. For each requires_memory prompt:
   - SemanticMemory.recall(prompt.memory_query, top_n) → facts
   - EntityStore filtered by prompt.entity_filter (optional)
4. Inject into context message:
   - "Prompt 'system_health_check' memory context: user_threshold_cpu_80 = 80°C"
   - "Prompt 'weather_context_guide' memory context: weather_location = Konstanz"
5. LLM receives tailored context → better tool selection and responses
```

---

## 3. Prompt Registry Extension

### 3.1 `RegisterPromptMessage` (`model/mcp/src/lib.rs`)

The `RegisterPromptMessage` struct is extended with three new fields:

```rust
/// Message sent by a plugin to register a prompt with the MCP server.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RegisterPromptMessage {
    /// Unique prompt name exposed to MCP clients.
    pub name: stabby::string::String,
    /// Human-readable description of the prompt.
    pub description: stabby::string::String,
    /// JSON schema for the prompt's arguments. Empty object if no arguments.
    pub arguments_schema: stabby::string::String,
    /// Whether the voice assistant should query memory before injecting this prompt.
    /// When true, the assistant uses `memory_query` to recall relevant facts
    /// from SemanticMemory and/or filter EntityStore entries.
    pub requires_memory: stabby::bool,
    /// Natural language query string used for SemanticMemory.recall().
    /// Ignored when `requires_memory` is false.
    /// Example: "CPU temperature threshold preference" for system_health_check.
    pub memory_query: stabby::string::String,
    /// Comma-separated entity name filter for EntityStore.
    /// Only entities whose name contains one of the filter strings are injected.
    /// Empty string means no filtering (all entities injected).
    /// Example: "cpu,memory,battery,temperature" for system_health_check.
    pub entity_filter: stabby::string::String,
}

impl RegisterPromptMessage {
    /// Create a new prompt registration message without memory requirements.
    pub fn new(name: &str, description: &str, arguments_schema: &str) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments_schema: arguments_schema.into(),
            requires_memory: false,
            memory_query: "".into(),
            entity_filter: "".into(),
        }
    }

    /// Create a new prompt registration message with memory requirements.
    pub fn with_memory(
        name: &str,
        description: &str,
        arguments_schema: &str,
        memory_query: &str,
        entity_filter: &str,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments_schema: arguments_schema.into(),
            requires_memory: true,
            memory_query: memory_query.into(),
            entity_filter: entity_filter.into(),
        }
    }
}
```

### 3.2 `RegisteredPrompt` (`model/mcp/src/registry.rs`)

The `RegisteredPrompt` struct in the `McpRegistry` is extended accordingly:

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
    /// Whether the voice assistant should query memory before injecting this prompt.
    pub requires_memory: bool,
    /// Natural language query for SemanticMemory.recall() when requires_memory is true.
    pub memory_query: String,
    /// Comma-separated entity name filter for EntityStore. Empty means no filtering.
    pub entity_filter: String,
}
```

The `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for McpRegistry` is updated to extract the new fields.

### 3.3 `PromptCatalogEntry` (`model/voice_assistant/src/messages/prompt_catalog.rs`)

```rust
/// A prompt entry in the voice assistant's prompt catalog.
#[derive(Debug, Clone)]
pub struct PromptCatalogEntry {
    /// Prompt name (e.g., "weather_summary").
    pub name: String,
    /// Human-readable description of the prompt.
    pub description: String,
    /// JSON schema for the prompt's arguments.
    pub arguments_schema: String,
    /// Whether the voice assistant should query memory before injecting this prompt.
    pub requires_memory: bool,
    /// Natural language query for SemanticMemory.recall() when requires_memory is true.
    pub memory_query: String,
    /// Comma-separated entity name filter for EntityStore. Empty means no filtering.
    pub entity_filter: String,
}
```

---

## 4. Context Injection

### 4.1 `build_context_message` Extension

The `build_context_message` function in `tool_catalog.rs` is extended to query memory for each prompt with `requires_memory = true`:

```rust
pub fn build_context_message(&self, user_text: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Tools, resources, prompts (existing).
    let selected_tools = self.select_tools_for_prompt(user_text);
    let tools_json = self.serialize_tools(&selected_tools);
    parts.push(format!("Available tools: {tools_json}"));

    let resources_json = self.serialize_resources();
    parts.push(format!("Available resources: {resources_json}"));

    let prompts_json = self.serialize_prompts();
    if prompts_json != "[]" {
        parts.push(format!("Available prompts: {prompts_json}"));
    }

    // Entity states (existing, but now optionally filtered by prompts).
    if self.config.inject_entity_states {
        let entity_summary = self.build_entity_summary();
        if !entity_summary.is_empty() {
            parts.push(format!("Known device states:\n{entity_summary}"));
        }
    }

    // Long-term facts (existing, user-text-based recall).
    if self.config.inject_long_term_facts {
        let long_term_summary = self.build_long_term_summary(user_text);
        if !long_term_summary.is_empty() {
            parts.push(format!("Known facts:\n{long_term_summary}"));
        }
    }

    // NEW: Prompt-driven memory context.
    let prompt_memory = self.build_prompt_memory_context();
    if !prompt_memory.is_empty() {
        parts.push(prompt_memory);
    }

    parts.join("\n")
}
```

### 4.2 `build_prompt_memory_context`

New method that iterates active prompts with `requires_memory = true` and queries memory:

```rust
/// Builds memory context for prompts that require memory access.
/// For each prompt with `requires_memory = true`:
/// - Queries SemanticMemory with the prompt's `memory_query`.
/// - Filters EntityStore entries by the prompt's `entity_filter`.
/// - Applies a token budget to prevent context overflow when many
///   memory-requiring prompts are active simultaneously.
/// Returns a formatted string with all prompt-memory context blocks.
/// Empty results (no matching facts or entities) produce no header line,
/// avoiding token waste.
fn build_prompt_memory_context(&self) -> String {
    let prompts = match self.prompt_catalog.read() {
        Ok(prompts) => prompts,
        Err(_) => return String::new(),
    };

    let memory_prompts: Vec<&PromptCatalogEntry> = prompts
        .iter()
        .filter(|p| p.requires_memory)
        .collect();

    if memory_prompts.is_empty() {
        return String::new();
    }

    // Token budget: limit the number of prompt-memory blocks to prevent
    // context overflow. When too many prompts are active, only the first
    // `max_prompt_memory_blocks` prompts (in registration order) are injected.
    // This is a simple, deterministic budgeting strategy. A future enhancement
    // could rank prompts by relevance to the current user_text via nucleo.
    const MAX_PROMPT_MEMORY_BLOCKS: usize = 5;
    const MAX_FACTS_PER_PROMPT: usize = 3;

    let mut blocks: Vec<String> = Vec::new();

    for prompt in memory_prompts.iter().take(MAX_PROMPT_MEMORY_BLOCKS) {
        // Semantic recall with prompt-specific query.
        // The `memory_query` string is static per prompt registration,
        // so the moka embedding cache in SemanticMemory guarantees a
        // cache hit after the first call — no repeated ONNX forward passes.
        if !prompt.memory_query.is_empty() {
            let facts = self
                .semantic_memory
                .write()
                .ok()
                .and_then(|mut memory| {
                    memory.recall(&prompt.memory_query, MAX_FACTS_PER_PROMPT).ok()
                })
                .unwrap_or_default();

            // Only emit a header when there are facts to show.
            // Empty results produce no output — no token waste.
            if !facts.is_empty() {
                let facts_summary = facts
                    .iter()
                    .map(|f| format!("- {} ({}): {}", f.key, f.category, f.value))
                    .collect::<Vec<_>>()
                    .join("\n");
                blocks.push(format!(
                    "Prompt '{}' recalled facts:\n{facts_summary}",
                    prompt.name
                ));
            }
        }

        // EntityStore filtering.
        // Only entities whose name contains one of the filter strings
        // (case-insensitive) are included. Empty filter = no filtering.
        // If no entities match, no header is emitted.
        if !prompt.entity_filter.is_empty() {
            let filters: Vec<&str> =
                prompt.entity_filter.split(',').map(str::trim).collect();
            let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
            let filtered: Vec<String> = store
                .values()
                .filter(|state| {
                    filters
                        .iter()
                        .any(|f| state.name.to_lowercase().contains(f))
                })
                .map(|state| format!("- {}: {}", state.name, state.state))
                .collect();

            // Only emit a header when there are matching entities.
            // No matches → no output, no token waste.
            if !filtered.is_empty() {
                blocks.push(format!(
                    "Prompt '{}' relevant entities:\n{}",
                    prompt.name,
                    filtered.join("\n")
                ));
            }
        }
    }

    if blocks.is_empty() {
        return String::new();
    }

    let mut result = format!("Prompt memory context:\n{}", blocks.join("\n"));

    // If we truncated prompts due to the budget, append a notice.
    if memory_prompts.len() > MAX_PROMPT_MEMORY_BLOCKS {
        result.push_str(&format!(
            "\n(Truncated: {} of {} memory prompts shown)",
            MAX_PROMPT_MEMORY_BLOCKS,
            memory_prompts.len()
        ));
    }

    result
}
```

### 4.3 Context Message Layout

After the extension, the context message has this structure:

```
Available tools: [{"name": "sysinfo_refresh", ...}]
Available resources: [{"uri": "sysinfo://cpu", ...}]
Available prompts: [{"name": "system_health_check", "requires_memory": true, ...}]
Known device states:
- Ventilator: on
- Shelly Plug: off
Known facts:
- fan_preference (preference): User prefers fan on level 2 in summer
Prompt memory context:
Prompt 'system_health_check' recalled facts:
- cpu_temp_threshold (preference): User wants CPU temperature warning at 80°C
- memory_warning_threshold (preference): User wants memory warning at 90%
Prompt 'system_health_check' relevant entities:
- CPU: 45.5%
- Memory: 62%
```

---

## 5. Memory Query Strategies

### 5.1 SemanticMemory Recall

Each prompt with `requires_memory = true` and a non-empty `memory_query` triggers a `SemanticMemory.recall()` call with the prompt-specific query string. This
is **additive** to the existing user-text-based recall in `build_long_term_summary`.

| Prompt Name             | `memory_query`                           | Purpose                                        |
|-------------------------|------------------------------------------|------------------------------------------------|
| `system_health_check`   | `"CPU temperature threshold preference"` | Recall user's temperature warning thresholds   |
| `weather_context_guide` | `"weather location preference"`          | Recall user's preferred weather location       |
| `mpris_control_guide`   | `"music player preference"`              | Recall user's preferred media player           |
| `power_safety_guide`    | `"power action confirmation preference"` | Recall whether user wants confirmation prompts |
| `memory_guide`          | `"memory management preference"`         | Recall user's memory cleanup habits            |

### 5.2 EntityStore Filtering

Each prompt with a non-empty `entity_filter` restricts the EntityStore to entries whose `name` contains one of the comma-separated filter strings
(case-insensitive substring match).

| Prompt Name           | `entity_filter`                    | Purpose                          |
|-----------------------|------------------------------------|----------------------------------|
| `system_health_check` | `"cpu,memory,battery,temperature"` | Only show system health entities |
| `mpris_control_guide` | `"player,media,music"`             | Only show media-related entities |
| `power_action_guide`  | `"power,shutdown,reboot"`          | Only show power-related entities |

When `entity_filter` is empty, all entities are shown (current behavior).

### 5.3 ConversationHistory (Future Enhancement)

A future extension could add a `requires_conversation_history: bool` field to `PromptCatalogEntry`, allowing prompts like `resolve_entity_intent` to trigger a
scan of recent conversation messages for pronoun resolution. This is not part of the initial implementation.

---

## 6. Use Cases

### 6.1 Context Filtering (Dynamic Prompting)

**Scenario**: User asks "Wie ist der Systemstatus?" and the `system_health_check` prompt is registered with `requires_memory = true`.

**Flow**:

1. `build_context_message("Wie ist der Systemstatus?")` is called.
2. The prompt catalog is scanned for `requires_memory` prompts.
3. `system_health_check` has `memory_query = "CPU temperature threshold preference"`.
4. `SemanticMemory.recall("CPU temperature threshold preference", 3)` returns:
    - `cpu_temp_threshold (preference): User wants CPU temperature warning at 80°C`
5. `system_health_check` has `entity_filter = "cpu,memory,battery,temperature"`.
6. EntityStore is filtered to only CPU, memory, and battery entities.
7. The context message includes: "Prompt 'system_health_check' recalled facts: ..." and "Prompt 'system_health_check' relevant entities: ...".
8. The LLM sees the user's threshold preferences and current entity states, enabling a tailored health report.

### 6.2 Prompt-Driven Memory Maintenance

**Scenario**: A `memory_cleanup_assistant` prompt is registered with `requires_memory = true` and `memory_query = "entity state changes habit pattern"`.

**Flow**:

1. The LLM requests the `memory_cleanup_assistant` prompt via `{"prompt": "memory_cleanup_assistant"}`.
2. The plugin handler returns instructions: "Analyze the entity history from the last 24 hours. Summarize frequent state changes into a habit fact. Use
   memory_store to persist it."
3. The context message already includes recalled facts about entity habits, giving the LLM context on what habits are already stored.
4. The LLM uses `memory_store` to persist new habits it discovers.

### 6.3 Pronoun Resolution via Memory

**Scenario**: User says "Mach ihn aus" and a `resolve_entity_intent` prompt is registered with `requires_memory = true`,
`memory_query = "last active device entity"`, and `entity_filter = ""` (all entities).

**Flow**:

1. `build_context_message("Mach ihn aus")` is called.
2. `resolve_entity_intent` triggers `SemanticMemory.recall("last active device entity", 3)`.
3. EntityStore is injected with all entities (no filter).
4. The LLM sees: "Prompt 'resolve_entity_intent' recalled facts: last_active_device = Ventilator" and "Known device states: Ventilator: on".
5. The LLM resolves "ihn" → Ventilator and calls the appropriate tool.

---

## 7. Plugin Registration Updates

### 7.1 Prompts Without Memory (Backward Compatible)

Existing prompts that do not require memory continue to use `RegisterPromptMessage::new()`:

```rust
let prompt = RegisterPromptMessage::new(
"app_launch_guide",
"System message with app search and launch pipeline instructions.",
r#"{ "type": "object", "properties": {} }"#,
);
broadcaster.broadcast_message_to_topic(prompt);
```

### 7.2 Prompts With Memory

Prompts that benefit from memory context use `RegisterPromptMessage::with_memory()`:

```rust
let health_prompt = RegisterPromptMessage::with_memory(
"system_health_check",
"Returns a structured system health diagnostic guide.",
r#"{ "type": "object", "properties": {} }"#,
"CPU temperature threshold preference",
"cpu,memory,battery,temperature",
);
broadcaster.broadcast_message_to_topic(health_prompt);

let weather_prompt = RegisterPromptMessage::with_memory(
"weather_context_guide",
"Returns instructions for resolving weather locations.",
r#"{ "type": "object", "properties": {} }"#,
"weather location preference",
"",
);
broadcaster.broadcast_message_to_topic(weather_prompt);
```

### 7.3 Affected Prompts

| Prompt Name                | Service           | `requires_memory` | `memory_query`                           | `entity_filter`                    |
|----------------------------|-------------------|-------------------|------------------------------------------|------------------------------------|
| `system_health_check`      | sysinfo           | `true`            | `"CPU temperature threshold preference"` | `"cpu,memory,battery,temperature"` |
| `weather_context_guide`    | weather           | `true`            | `"weather location preference"`          | `""`                               |
| `mpris_control_guide`      | mpris             | `true`            | `"music player preference"`              | `"player,media,music"`             |
| `power_safety_guide`       | power             | `true`            | `"power action confirmation preference"` | `""`                               |
| `memory_guide`             | voice_assistant   | `true`            | `"memory management preference"`         | `""`                               |
| `launcher_overview`        | mcp-server (core) | `false`           | —                                        | —                                  |
| `area_control_help`        | mcp-server (core) | `false`           | —                                        | —                                  |
| `broker_message_guide`     | mcp-server (core) | `false`           | —                                        | —                                  |
| `tool_shortcut_guide`      | mcp-server (core) | `false`           | —                                        | —                                  |
| `voice_assistant_status`   | voice_assistant   | `false`           | —                                        | —                                  |
| `weather_query_guide`      | weather           | `false`           | —                                        | —                                  |
| `app_launch_guide`         | app-launcher      | `false`           | —                                        | —                                  |
| `terminal_command_guide`   | terminal_command  | `false`           | —                                        | —                                  |
| `terminal_lifecycle_guide` | terminal_command  | `false`           | —                                        | —                                  |
| `power_action_guide`       | power             | `false`           | —                                        | —                                  |

---

## 8. Handler Updates

### 8.1 `on_prompt_registered` (`services/voice_assistant/src/tool_catalog.rs`)

The method signature is extended to accept the new fields:

```rust
pub fn on_prompt_registered(
    &self,
    name: String,
    description: String,
    arguments_schema: String,
    requires_memory: bool,
    memory_query: String,
    entity_filter: String,
) {
    let entry = PromptCatalogEntry {
        name,
        description,
        arguments_schema,
        requires_memory,
        memory_query,
        entity_filter,
    };
    if let Ok(mut catalog) = self.prompt_catalog.write() {
        catalog.push(entry);
    }
    debug!(
        "Voice Assistant: prompt catalog updated, {} entries",
        self.prompt_catalog.read().map(|c| c.len()).unwrap_or(0)
    );
}
```

### 8.2 `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>>` for `VoiceAssistantService`

The handler extracts the new fields from the incoming message:

```rust
impl MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterPromptMessage>, _sender_id: &str) {
        let name = message.0.name.to_string();
        let description = message.0.description.to_string();
        let arguments_schema = message.0.arguments_schema.to_string();
        let requires_memory = message.0.requires_memory;
        let memory_query = message.0.memory_query.to_string();
        let entity_filter = message.0.entity_filter.to_string();
        debug!(
            "Voice Assistant: Prompt registered: {} (requires_memory={})",
            name, requires_memory
        );
        self.on_prompt_registered(
            name,
            description,
            arguments_schema,
            requires_memory,
            memory_query,
            entity_filter,
        );
    }
}
```

### 8.3 `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>>` for `McpRegistry`

The registry handler is updated to store the new fields in `RegisteredPrompt`:

```rust
impl MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for McpRegistry {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterPromptMessage>, sender_id: &str) {
        let schema = serde_json::from_str(&message.0.arguments_schema.to_string())
            .unwrap_or(serde_json::Value::Null);
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
```

---

## 9. `serialize_prompts` Extension

The `serialize_prompts` function remains unchanged — the `requires_memory`, `memory_query`, and `entity_filter` fields are **internal** to the voice assistant
and are **not** serialized into the LLM-visible prompt list. They only affect the `build_prompt_memory_context` output:

```rust
fn serialize_prompts(&self) -> String {
    let prompts = self.prompt_catalog.read().unwrap_or_else(|e| e.into_inner());
    let prompts_json: Vec<serde_json::Value> = prompts
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "description": p.description,
                "arguments_schema": serde_json::from_str::<serde_json::Value>(&p.arguments_schema)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect();
    serde_json::to_string(&prompts_json).unwrap_or_default()
}
```

---

## 10. Crate Structure

| Crate                       | Path                        | Responsibility                                                                                   |
|-----------------------------|-----------------------------|--------------------------------------------------------------------------------------------------|
| **Model (MCP)**             | `model/mcp/`                | `RegisterPromptMessage` extension with `requires_memory`, `memory_query`, `entity_filter` fields |
| **Model (MCP Registry)**    | `model/mcp/src/registry.rs` | `RegisteredPrompt` extension, `MessageHandler` update                                            |
| **Model (Voice Assistant)** | `model/voice_assistant/`    | `PromptCatalogEntry` extension with new fields                                                   |
| **Voice Assistant Service** | `services/voice_assistant/` | `on_prompt_registered` update, `build_prompt_memory_context` method, `MessageHandler` update     |
| **Service Plugins**         | `services/*/`               | Update `RegisterPromptMessage::new` → `with_memory` for prompts that benefit from memory context |

---

## 11. Roadmap

### Phase 1: Model Crate Extensions (`model/mcp`, `model/voice_assistant`)

**Goal:** Extend all shared types with the `requires_memory`, `memory_query`, and `entity_filter` fields.

**Order:**

1. Add `requires_memory: stabby::bool`, `memory_query: stabby::string::String`, and `entity_filter: stabby::string::String` to `RegisterPromptMessage` in
   `model/mcp/src/lib.rs`.
2. Add `with_memory()` constructor to `RegisterPromptMessage`.
3. Update `new()` constructor to set defaults (`requires_memory: false`, empty strings).
4. Add the same fields to `RegisteredPrompt` in `model/mcp/src/registry.rs`.
5. Update `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for McpRegistry` to extract the new fields.
6. Add the same fields to `PromptCatalogEntry` in `model/voice_assistant/src/messages/prompt_catalog.rs`.
7. Run `cargo check` for both model crates.

**Exit criteria:**

- Both model crates compile without warnings.
- All new fields have English rustdoc documentation.
- `#[stabby::stabby]` is present on all FFI-relevant types.

---

### Phase 2: Voice Assistant Service Integration

**Goal:** Update the voice assistant to accept and store the new prompt fields, and implement `build_prompt_memory_context`.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Update `on_prompt_registered` in `tool_catalog.rs` to accept the three new parameters.
2. Update `MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for VoiceAssistantService` to extract and pass the new fields.
3. Implement `build_prompt_memory_context` in `tool_catalog.rs`.
4. Extend `build_context_message` to call `build_prompt_memory_context` and append the result.
5. Run `cargo check` for the voice assistant service crate.

**Exit criteria:**

- The voice assistant service compiles without warnings.
- `build_context_message` includes prompt-driven memory context when prompts with `requires_memory = true` are registered.
- Prompts without `requires_memory` do not trigger memory queries (no performance regression).

---

### Phase 3: Service Plugin Updates

**Goal:** Update service plugins to use `RegisterPromptMessage::with_memory()` for prompts that benefit from memory context.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. **sysinfo service**: Update `system_health_check` prompt registration to use `with_memory()` with `memory_query = "CPU temperature threshold preference"` and
   `entity_filter = "cpu,memory,battery,temperature"`.
2. **weather service**: Update `weather_context_guide` prompt registration to use `with_memory()` with `memory_query = "weather location preference"`.
3. **mpris service**: Update `mpris_control_guide` prompt registration to use `with_memory()` with `memory_query = "music player preference"` and
   `entity_filter = "player,media,music"`.
4. **power service**: Update `power_safety_guide` prompt registration to use `with_memory()` with `memory_query = "power action confirmation preference"`.
5. **voice assistant service**: Update `memory_guide` prompt registration to use `with_memory()` with `memory_query = "memory management preference"`.
6. Run `cargo check` for all affected service crates.

**Exit criteria:**

- All service plugins compile without warnings.
- Prompts that do not need memory continue to use `RegisterPromptMessage::new()` (backward compatible).
- At least 5 prompts are registered with `requires_memory = true`.

---

### Phase 4: Build, Test & Verify

**Goal:** Full build and verification.

**Dependencies:** All previous phases must be complete.

**Order:**

1. Run `cargo build` for the entire workspace.
2. Run `cargo test` for the model crates.
3. Manually verify that the context message includes prompt-driven memory context when memory-requiring prompts are registered.
4. Verify that prompts without `requires_memory` do not produce memory context blocks.
5. Verify that empty `memory_query` or empty `entity_filter` are handled gracefully.

**Exit criteria:**

- Full workspace build succeeds without errors.
- Context message includes prompt-memory blocks only for prompts with `requires_memory = true`.
- No panics or crashes when SemanticMemory or EntityStore are empty.

---

## 12. Configuration

No new configuration fields are required. The existing `inject_long_term_facts` and `inject_entity_states` flags control the existing memory injection. The
prompt-driven memory context is always injected when prompts with `requires_memory = true` are registered, as it is a prompt-level concern, not a global config
toggle.

If a global toggle is desired in the future, a `inject_prompt_memory_context: bool` field can be added to `VoiceAssistantServiceConfig` with a default of
`true`.

---

## 13. Performance Considerations

| Operation                            | Cost                                                        | Mitigation                                                   |
|--------------------------------------|-------------------------------------------------------------|--------------------------------------------------------------|
| `SemanticMemory.recall()` per prompt | ~5-10 ms first call (ONNX forward pass + cosine similarity) | Embedding cached by moka on subsequent calls                 |
| `EntityStore` filtering per prompt   | < 1 ms (in-memory HashMap scan)                             | Negligible                                                   |
| Additional context tokens            | ~50-100 tokens per prompt-memory block                      | Budgeted to `MAX_PROMPT_MEMORY_BLOCKS` (5) → max ~500 tokens |

### 13.1 Embedding Cache Guarantee

The `memory_query` string is **static per prompt registration** — it is defined once in `RegisterPromptMessage::with_memory()` and never changes at runtime.
This means the moka embedding cache in `SemanticMemory` provides a **cache hit guarantee** after the first call for each prompt:

- First command with prompt `system_health_check`: `recall("CPU temperature threshold preference")` → ONNX forward pass (~5-10 ms) → embedding cached.
- Second command: same `memory_query` → **cache hit** → cosine similarity only (~1-2 ms).
- All subsequent commands: **cache hit** → ~1-2 ms per prompt.

With 5 memory-requiring prompts, the first command costs ~25-50 ms total, and all subsequent commands cost ~5-10 ms total — negligible compared to LLM inference
time.

### 13.2 Token Budgeting

When many prompts with `requires_memory = true` are active simultaneously, the prompt-memory context block could grow unbounded. The
`build_prompt_memory_context` method applies a **token budget** via two constants:

- `MAX_PROMPT_MEMORY_BLOCKS = 5`: At most 5 prompts contribute memory context blocks. Additional prompts are truncated with a notice.
- `MAX_FACTS_PER_PROMPT = 3`: At most 3 facts per prompt are recalled from SemanticMemory.

**Worst-case token estimate:**

| Component                                   | Tokens   |
|---------------------------------------------|----------|
| 5 prompts × 3 facts × ~15 tokens/fact       | ~225     |
| 5 prompts × ~5 entities × ~10 tokens/entity | ~250     |
| Headers and formatting                      | ~50      |
| Truncation notice (if applicable)           | ~15      |
| **Total worst case**                        | **~540** |

This is well within the context window budget and significantly smaller than the unfiltered tool catalog (~1000 tokens without nucleo).

**Future enhancement**: Instead of taking the first 5 prompts in registration order, a nucleo-based relevance ranking could select the top 5 prompts most
relevant to the current `user_text`. This would ensure the most contextually important prompts are prioritized when truncation is necessary.

---

## 14. Security Considerations

- **No sensitive data in memory queries**: The `memory_query` string is set by the plugin developer at registration time. It must not contain user-specific
  sensitive data.
- **Entity filtering is read-only**: The `entity_filter` only restricts which entities are shown — it does not modify the EntityStore.
- **No LLM-visible memory queries**: The `memory_query` and `entity_filter` fields are internal configuration and are not serialized into the LLM-visible prompt
  list in `serialize_prompts`.
- **Memory recall is read-only**: `SemanticMemory.recall()` does not modify stored facts. It only updates `last_accessed` and `access_count` (touch).

---

## 15. Interaction with Other Concepts

### With MCP Prompt Concept (`MCP_PROMPT_CONCEPT.md`)

This concept extends the existing MCP Prompt framework. The `RegisterPromptMessage` struct gains new fields, but the MCP protocol endpoints (`prompts/list`,
`prompts/get`) are not affected — the new fields are internal to the voice assistant and do not appear in the MCP client-facing schema.

### With Long-Term Memory Concept (`SPEAK_SWIPER_LONG_MEMORY_CONCEPT.md`)

The `SemanticMemory.recall()` method is reused as-is. The prompt-driven recall is **additive** to the existing user-text-based recall in
`build_long_term_summary`. Both run independently in `build_context_message`.

### With Short-Term Memory Concept (`SPEAK_SWIPER_SHORT_MEMORY_CONCEPT.md`)

The `EntityStore` filtering reuses the existing `entity_store: Arc<RwLock<HashMap<String, EntityState>>>`. No changes to the EntityStore data structure are
required. The filtering is a read-only scan at context-build time.

### With nucleo Tool Router

The nucleo tool router is unaffected. Tool selection and prompt-driven memory context operate independently in `build_context_message`. The nucleo router
selects tools based on the user's raw speech, while prompt-memory context enriches the LLM's understanding of the user's preferences and entity states.

---

## 16. Testing Strategy

### Unit Tests

- **`RegisterPromptMessage::new()`**: Verify `requires_memory` defaults to `false` and `memory_query`/`entity_filter` default to empty strings.
- **`RegisterPromptMessage::with_memory()`**: Verify all fields are set correctly.
- **`PromptCatalogEntry` construction**: Verify the new fields are stored and readable.
- **`build_prompt_memory_context` with no memory prompts**: Verify empty string output.
- **`build_prompt_memory_context` with memory prompts but empty SemanticMemory**: Verify empty string output (graceful degradation).
- **`build_prompt_memory_context` with entity_filter**: Verify only matching entities are included.
- **`build_prompt_memory_context` with entity_filter matching no entities**: Verify no header line is emitted (no "Prompt 'XYZ' relevant entities:" with empty
  content).
- **`build_prompt_memory_context` with empty SemanticMemory and empty EntityStore**: Verify empty string output (no headers, no token waste).
- **`build_prompt_memory_context` token budgeting**: Register 10 memory-requiring prompts, verify only `MAX_PROMPT_MEMORY_BLOCKS` (5) are injected and a
  truncation notice is appended.
- **`build_prompt_memory_context` caching**: Call `build_prompt_memory_context` twice with the same prompts, verify the second call is faster (embedding cache
  hit).

### Integration Tests

- **End-to-end prompt registration**: Register a prompt with `with_memory()`, verify it appears in the prompt catalog with `requires_memory = true`.
- **Context message with memory prompt**: Register a memory-requiring prompt, store a matching fact in SemanticMemory, call `build_context_message`, verify the
  fact appears in the "Prompt memory context" section.
- **Context message without memory prompt**: Register only non-memory prompts, call `build_context_message`, verify no "Prompt memory context" section is
  present.
- **Entity filtering**: Register a prompt with `entity_filter = "cpu,memory"`, populate EntityStore with mixed entities, verify only CPU/memory entities appear
  in the prompt-memory context.

---

*Concept for integrating MCP Prompts with the Voice Assistant's short-term and long-term memory systems — enabling prompts to act as interface logic that
dynamically queries memory before injection, evolving the AI from a reactive command-receiver to a proactive assistant.*
