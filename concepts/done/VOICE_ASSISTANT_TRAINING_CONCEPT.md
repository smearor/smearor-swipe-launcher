# Concept: Voice Assistant Training Mode (Evaluation Harness)

This document describes the concept for an **Evaluation Harness** in the Voice Assistant Service. It enables training-mode recording of the ReAct loop, exposes
start/stop/query MCP tools, and supports offline analysis and fine-tuning of LLM behavior.

---

## 1. Goal & Motivation

### Current State

The Voice Assistant operates in a **ReAct loop** (Reasoning + Acting):

1. User prompt is transcribed.
2. LLM produces a reasoning step (`thought`) and selects an `action`.
3. The action is executed (tool or resource read).
4. The `observation` is fed back into the loop.
5. The loop terminates with a `final_answer` or `clarify`.

This trace is currently **not recorded**. The only observable output is the final answer in `voice_assistant://status`. There is no way to:

- Inspect the chain of thought.
- Replay a session.
- Identify which tool call caused a failure.
- Export training data for supervised fine-tuning.

### Problem

Without a recorded trace, debugging and improving the LLM is hard:

- The user sees a wrong answer but cannot know **which** step failed.
- The LLM may hallucinate a tool name, pick the wrong parameters, or ignore a memory fact.
- There is no structured dataset to evaluate prompt or model changes.
- There is no switch to control whether a session should be recorded.

### Required Capabilities

| Capability          | Example                                       | Solution                                         |
|---------------------|-----------------------------------------------|--------------------------------------------------|
| Start recording     | "Start training mode"                         | `voice_assistant_training_start` tool            |
| Stop recording      | "End training mode"                           | `voice_assistant_training_end` tool              |
| Inspect trace       | "Show me the last ReAct trace"                | `voice_assistant_training_get` tool              |
| Capture ReAct steps | Thought, Action, Observation, Answer          | `TrainingTrace` data structure inside `react.rs` |
| Export / replay     | Generate a JSON trace for external evaluation | Query tool returns JSON-serializable traces      |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                     Voice Assistant Service                          │
│                                                                      │
│  ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐  │
│  │   MCP Tools     │    │  Training Mode   │    │  ReAct Loop     │  │
│  │                 │    │                  │    │                 │  │
│  │ training_start  │───▶│  training_mode   │───▶│  execute_react  │  │
│  │ training_end    │    │  active trace    │    │  trace.add()    │  │
│  │ training_get    │◀───│  trace history   │◀───│  final / error  │  │
│  └─────────────────┘    └──────────────────┘    └─────────────────┘  │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User (or harness) calls voice_assistant_training_start()
2. VoiceAssistantService sets training_mode = true
   and creates a fresh TrainingTrace slot for the next user prompt.
3. User says "Spiele meinen Lieblingssong ab"
4. execute_react_loop() records per iteration:
   - thought: raw LLM output
   - action: parsed tool/resource/answer/clarify
   - parameters: JSON arguments for tool/resource
   - observation: tool result, resource content, or error
   - answer: final_answer or clarify text
5. Loop ends with final answer or error
6. VoiceAssistantService closes the trace and stores it
7. User calls voice_assistant_training_get() to retrieve the trace
```

---

## 3. Training Trace Data Model

The trace data is stored in the `services/voice_assistant` crate. It stays internal; no model crate is needed because the data is not shared across plugins.

### `ReActStep`

```rust
#[derive(Debug, Clone)]
pub struct ReActStep {
    /// Iteration number inside the ReAct loop.
    pub iteration: usize,
    /// Raw LLM output before parsing.
    pub thought: String,
    /// Parsed action kind, e.g. "tool:voice_assistant_recall_memory".
    pub action: String,
    /// Raw JSON or URI argument for the action.
    pub parameters: String,
    /// Result of the tool/resource execution, or error text.
    pub observation: String,
    /// Final answer or clarify text produced in this step.
    pub answer: Option<String>,
    /// Timestamp of this step.
    pub timestamp: std::time::Instant,
}
```

### `TrainingTrace`

```rust
#[derive(Debug, Clone)]
pub struct TrainingTrace {
    /// User input that started this trace.
    pub user_text: String,
    /// When the trace started.
    pub start_time: std::time::Instant,
    /// When the trace ended, if it has ended.
    pub end_time: Option<std::time::Instant>,
    /// All recorded steps.
    pub steps: Vec<ReActStep>,
    /// Whether the trace ended with a final answer.
    pub success: Option<bool>,
    /// Optional label supplied by the caller, e.g. for dataset grouping.
    pub label: Option<String>,
}
```

---

## 4. MCP Tools

All three tools are registered in `services/voice_assistant/src/mcp.rs` as `RegisterToolMessage` broadcasts and handled in the `InvokeToolMessage` handler.

### 4.1 `voice_assistant_training_start`

**Description:** Enables training mode for the Voice Assistant. The next user interaction will be recorded as a training trace.

**Input schema:**

```json
{
  "type": "object",
  "properties": {
    "label": {
      "type": "string",
      "description": "Optional label for the training trace (e.g. 'favorite_song_test')."
    }
  }
}
```

**Behavior:**

- Sets `training_mode` to `true`.
- Prepares a `TrainingTrace` slot for the next `execute_react_loop` call.
- Returns `{"status": "ok"}`.

### 4.2 `voice_assistant_training_end`

**Description:** Disables training mode and finalizes the current trace.

**Input schema:**

```json
{
  "type": "object",
  "properties": {}
}
```

**Behavior:**

- Sets `training_mode` to `false`.
- Closes the currently active trace with `end_time` and `success`.
- Returns `{"status": "ok", "trace_id": "trace-2026-07-15-09-50-12-abc123"}`.

### 4.3 Trace ID Management

Each trace receives a stable, globally unique identifier. The recommended format is:

```
trace-{timestamp}-{uuid}
```

Example:

- `trace-2026-07-15T09:50:12Z-7f3a9c2e`

The identifier is generated in `voice_assistant_training_end` when the trace is finalized and stored in `TrainingTrace.id`. The `training_history` uses a
`BTreeMap<String, TrainingTrace>` keyed by `trace_id` so that lookups are stable and the ordering of insertion is preserved.

For in-memory storage, this is sufficient. If persistence for `training_history` is added later (e.g. JSONL on disk), the `trace_id` is used as the file key and
is the same across restarts. This ensures that `voice_assistant_training_get` with `trace_id` returns the same trace after a service restart.

### 4.3 `voice_assistant_training_get`

**Description:** Returns the last N recorded training traces, optionally filtered by label or user text substring.

**Input schema:**

```json
{
  "type": "object",
  "properties": {
    "limit": {
      "type": "integer",
      "description": "Maximum number of traces to return. Default: 1."
    },
    "label": {
      "type": "string",
      "description": "Optional label to filter traces."
    },
    "query": {
      "type": "string",
      "description": "Optional substring to search in user_text."
    }
  }
}
```

**Output schema:**

```json
{
  "type": "object",
  "properties": {
    "traces": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "user_text": { "type": "string" },
          "label": { "type": "string" },
          "success": { "type": "boolean" },
          "steps": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "iteration": { "type": "integer" },
                "thought": { "type": "string" },
                "action": { "type": "string" },
                "parameters": { "type": "string" },
                "observation": { "type": "string" },
                "answer": { "type": "string" }
              }
            }
          }
        }
      }
    }
  }
}
```

---

## 5. Integration into the ReAct Loop

The ReAct loop in `services/voice_assistant/src/react.rs` is instrumented as follows:

### 5.1 Start Trace

Before the loop starts, the service checks `training_mode`:

```rust
if * training_mode.lock().unwrap() {
let mut trace = current_trace.lock().unwrap();
trace.user_text = user_text.to_string();
trace.start_time = std::time::Instant::now();
trace.steps.clear();
trace.success = None;
trace.end_time = None;
}
```

### 5.2 Record Each Iteration

For each ReAct iteration:

```rust
let mut step = ReActStep {
iteration: loop_index,
thought: llm_output.clone(),
action: action.to_string(),
parameters: params.to_string(),
observation: String::new(),
answer: None,
timestamp: std::time::Instant::now(),
};

// Execute action, capture observation
step.observation = execute_action(action, params).await.unwrap_or_else( | e| e.to_string());

if let ActionResult::FinalAnswer(answer) = result {
step.answer = Some(answer.clone());
}

training_trace.lock().unwrap().steps.push(step);
```

### 5.3 Finalize Trace

At loop exit:

```rust
if * training_mode.lock().unwrap() {
let mut trace = current_trace.lock().unwrap();
trace.end_time = Some(std::time::Instant::now());
trace.success = Some(final_answer.is_some());
// Move trace to history
training_history.lock().unwrap().push(trace.clone());
}
```

---

## 6. State Additions in `VoiceAssistantService`

In `services/voice_assistant/src/service/loaded_service.rs` add:

```rust
/// Whether training mode is active.
training_mode: Arc<Mutex<bool>>,
/// Currently active trace being recorded.
active_trace: Arc<Mutex<TrainingTrace>>,
/// History of completed traces.
training_history: Arc<Mutex<Vec<TrainingTrace>>>,
```

These fields are cloned into the ReAct loop and the tool handlers just like the other service state.

---

## 7. File Additions

- `services/voice_assistant/src/training.rs` - new module for `ReActStep` and `TrainingTrace`
- `services/voice_assistant/src/lib.rs` - register `pub mod training;`
- `services/voice_assistant/src/service/loaded_service.rs` - add state fields and clone them into the pipeline
- `services/voice_assistant/src/react.rs` - instrument the loop
- `services/voice_assistant/src/mcp.rs` - register and handle the three MCP tools

---

## 8. Security & Privacy

- Training traces are kept in memory only. No persistence is implemented by default.
- Trace data includes tool results and may contain sensitive information (e.g. search queries, system state).
- If future features export traces to disk, they must require explicit user opt-in.

---

## 9. Semantic Memory Integration for Traces

Training traces should not be lost after the session ends. Successful, user-approved traces are persisted as **workflows** in `SemanticMemory` and optionally as
`Entity` snapshots in `EntityStore`. This allows the Voice Assistant to retrieve proven problem-solving paths before starting a new ReAct loop.

### 9.1 Workflow Storage

When a trace ends with `success = true`, the service optionally calls:

```rust
semantic_memory.store(
key: "workflow_{label}",
value: json!(trace),
category: "user_approved_workflow",
tags: vec![trace.label, "workflow", "training"],
)
```

Example:

- `key: "workflow_play_favorite_song"`
- `value: { "user_text": "Spiele meinen Lieblingssong ab", "steps": [...] }`
- `category: "user_approved_workflow"`

### 9.2 Retrieval Before ReAct Loop

Before `execute_react_loop` starts, the service checks:

```rust
if let Ok(facts) = semantic_memory.recall(user_text, 3) {
for fact in facts.iter().filter( | f | f.category == "user_approved_workflow") {
// Inject workflow into context or short-circuit the loop
}
}
```

If a matching workflow exists, the assistant can either:

- Inject the workflow steps as context into the system prompt.
- Execute the same action sequence directly if confidence is high.
- Ask the user: "Ich habe das letzte Mal so gemacht: [steps]. Soll ich das wiederholen?"

### 9.3 Trace Diff Analysis

`voice_assistant_training_get` supports querying two traces by `trace_id` for comparison. This enables prompt or model A/B testing:

**Input:**

```json
{
  "trace_a": 0,
  "trace_b": 1,
  "compare": true
}
```

**Output:**

```json
{
  "trace_a": { ... },
  "trace_b": { ... },
  "comparison": {
    "steps_a": 3,
    "steps_b": 2,
    "a_successful": true,
    "b_successful": true,
    "difference_in_words": 120,
    "efficiency_winner": "b"
  }
}
```

A future harness can ask the LLM: "Compare trace A and trace B. Is the new trace more efficient?" and use the structured diff as context.

---

## 10. Memory Distillation (Automatisches Lernen)

Successful traces contain repeated patterns of user behavior. A background process can distill these patterns into compact **policy rules** or **user habits**
stored in `SemanticMemory`. The user no longer has to explicitly state preferences.

### 10.1 Pattern Detection

The distillation task runs periodically or after `training_end` and analyzes the last N successful traces with the same label. It looks for repeated action
sequences:

```rust
for trace in successful_traces {
let action_sequence = trace.steps.iter()
.map( | s | & s.action)
.collect::< Vec < _ > >();
pattern_counter.increment(action_sequence);
}
```

Example sequence:

- `mpris_control_guide` prompt
- `mpris_play` tool
- `mpris_volume` tool

### 10.2 Habit Creation

If a sequence appears `>= 5` times consecutively and the trace ends successfully, the system creates a `SemanticMemory` fact:

```rust
semantic_memory.store(
key: "user_habit_mpris_loud_start",
value: "After mpris_play, the user raises the volume. Start with a higher volume.",
category: "user_habit",
tags: vec!["mpris", "volume", "habit"],
)
```

### 10.3 Habit Application

The `mpris_play` tool handler can recall `user_habit_mpris_loud_start` and apply the preference automatically:

```rust
if let Ok(habits) = semantic_memory.recall("mpris play volume habit", 1) {
if habits.iter().any( |h | h.category == "user_habit") {
mpris_volume.set(80);
}
}
```

#### Habit Override Priority

Explicit user intent must always override learned habits. The override hierarchy is:

1. **Explicit user request** (e.g. "setze Lautstärke auf 20") — highest priority, disables habit for this invocation.
2. **Context from current conversation** (e.g. user just said "leise").
3. **Learned `user_habit` from SemanticMemory** — applied only when no explicit request is present.
4. **Default behavior** — lowest priority.

Before applying a habit, the tool handler checks whether the current user input or the tool arguments contain an explicit value. If they do, the habit is
skipped for this invocation but remains in memory for future calls.

### 10.4 Habit Maintenance

Habits are not permanent. If a distilled habit leads to a failed trace, the system:

- Decreases the confidence counter.
- Removes the habit if it fails consistently.
- Stores a new habit if the pattern changes.

---

## 11. Future Extensions

- **Export to JSONL** for supervised fine-tuning of the LLM.
- **Trace replay** to reproduce an exact session.
- **Annotation** in `voice_assistant_training_end` to mark the trace as good/bad.
- **Evaluation Harness** that compares a baseline and new model on the same set of traces.
- **Distillation scheduler** that runs in a background task and emits habit updates.
