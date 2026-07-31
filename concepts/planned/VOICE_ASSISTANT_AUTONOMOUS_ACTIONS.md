# Concept: Voice Assistant Autonomous Actions (Proactive Agent)

This document describes the concept for **Autonomous Actions** in the Voice Assistant Service. The assistant periodically "wakes up" on its own — without user
interaction or wake word — to execute tasks stored in `SemanticMemory`. The LLM autonomously plans and schedules trigger events using its existing tool set
(weather lookup, memory, entity states, etc.). Tasks include time-based reminders, environment-triggered actions (e.g. lights on at sunset), and periodic
routines (e.g. start internet radio during the day).

---

## 1. Goal & Motivation

### Current State

The Voice Assistant operates in a **reactive** model:

1. User activates the pipeline (button, MCP tool, wake word, or text input).
2. STT transcribes audio.
3. The ReAct loop executes tools and produces a final answer.
4. The assistant returns to `Idle` or `Standby`.

The assistant never acts on its own. It has no concept of time, schedules, or pending tasks. All actions require explicit user initiation.

### Problem

- **No proactive behavior**: The assistant cannot remind the user of an appointment at 18:00.
- **No time-based automation**: The assistant cannot turn on lights at a specific time.
- **No periodic routines**: The assistant cannot start internet radio at a set time.
- **No task persistence**: Even if the user says "remind me at 18:00", the assistant has no mechanism to schedule and execute that reminder later.

### Required Capabilities

| Capability           | Example                                          | Solution                                                              |
|----------------------|--------------------------------------------------|-----------------------------------------------------------------------|
| Time-based trigger   | "Remind me of my appointment at 18:00"           | `TriggerType::Absolute` at 18:00                                      |
| LLM-planned trigger  | "Turn on lights at sunset"                       | LLM looks up sunset time via weather tool, creates `Absolute` trigger |
| Periodic trigger     | "Play internet radio during the day"             | `TriggerType::Periodic` with daytime condition                        |
| Task persistence     | Tasks survive restarts                           | Stored as `AutonomousTask` in `SemanticMemory`                        |
| Autonomous execution | LLM wakes up and runs ReAct loop                 | `AutonomousScheduler` spawns `SubmitText` commands                    |
| Task management      | Create, list, delete tasks                       | MCP tools for CRUD operations                                         |
| User notification    | "Dein Termin um 18:00 Uhr beginnt in 15 Minuten" | TTS speaks the final answer                                           |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                       Voice Assistant Service                            │
│                                                                          │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │  Autonomous      │    │  Semantic        │    │  Existing        │   │
│  │  Scheduler       │    │  Memory          │    │  Pipeline        │   │
│  │                  │    │                  │    │                  │   │
│  │  tick (60s)      │───▶│  recall tasks    │    │  SubmitText      │   │
│  │  evaluate        │    │  store tasks     │    │  → ReAct loop    │   │
│  │  fire due tasks  │───▶│  delete tasks    │    │  → TTS answer    │   │
│  │                  │    │                  │    │                  │   │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘   │
│                                  │                     │               │
│                                  │                     │               │
│                          ┌───────▼──────────┐    ┌────▼──────────┐     │
│                          │  Task Store      │    │  State:        │     │
│                          │  (SQLite + RAM)  │    │  Autonomous →  │     │
│                          │  category:       │    │  ThinkingLlm → │     │
│                          │  autonomous_task │    │  Idle           │     │
│                          └──────────────────┘    └────────────────┘     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User says: "Schalte die Lichter bei Sonnenuntergang ein."
2. ReAct loop uses weather tool to look up today's sunset time (e.g. 21:43).
3. ReAct loop stores an AutonomousTask in SemanticMemory:
   - trigger: Absolute { time: "21:43", date: None }
   - prompt: "Schalte alle Lichter ein."
   - category: autonomous_task
4. AutonomousScheduler ticks every 60 seconds.
5. Scheduler recalls all autonomous_task facts from SemanticMemory.
6. For each task, scheduler evaluates the trigger condition:
   a. Absolute(18:00): compare current time to target time.
   b. Periodic(every 2h, 08:00–20:00): check interval and time window.
7. If a trigger fires:
   a. Scheduler sends VoiceCommandMessage::submit_text(task.prompt).
   b. Service transitions to Autonomous state.
   c. ReAct loop runs with the stored prompt.
   d. TTS speaks the result (if enabled).
   e. Service returns to Idle or Standby.
8. One-shot tasks are deleted after execution.
   Recurring tasks remain and fire again on next trigger.
```

### LLM-Directed Trigger Planning

The LLM plans triggers autonomously using its existing tool set. There are no specialized trigger types for sun events, weather conditions, or entity states.
Instead, the LLM uses the ReAct loop to gather information and then creates a
`TriggerType::Absolute` or `TriggerType::Periodic` task:

- "Lights on at sunset" → LLM calls weather tool to get sunset time → creates `Absolute` trigger
- "Reminder when temperature drops below 18°C" → LLM creates a `Periodic` trigger that checks temperature every 30 minutes
- "Turn on fan when I get home" → LLM creates a `Periodic` trigger that checks presence entity state every 5 minutes

This puts the intelligence in the LLM rather than in the scheduler. The scheduler remains a simple time-based evaluator. The LLM can compose complex conditions
by combining tool calls with trigger creation.

---

## 3. Task Data Model

Tasks are stored as `AutonomousTask` structs. They are serialized to JSON and stored in `SemanticMemory` with category `autonomous_task`. The `StoredFact.key`
field holds the task ID, and `StoredFact.value` holds the JSON-serialized task.

### `TriggerType`

```rust
/// Defines when an autonomous task should fire.
/// The LLM chooses the appropriate trigger type and parameters based on
/// the user's request and information gathered from tools (weather,
/// entity states, memory, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerType {
    /// Fires at a specific wall-clock time.
    /// Format: "HH:MM" (24-hour, local timezone).
    /// Example: "18:00" fires at 18:00 local time.
    /// The LLM can look up sun event times, weather data, or other
    /// contextual information via tools and then create an Absolute
    /// trigger at the resolved time.
    Absolute {
        /// Time in "HH:MM" format (24-hour, local timezone).
        time: String,
        /// Optional date in "YYYY-MM-DD" format. If None, fires every day.
        date: Option<String>,
    },
    /// Fires periodically within a time window.
    /// Example: every 120 minutes between 08:00 and 20:00.
    /// The LLM can use this for condition-based checks by creating a
    /// Periodic trigger with a short interval and a prompt that includes
    /// a condition (e.g. "If temperature > 25°C, turn on fan.").
    Periodic {
        /// Interval between firings in minutes.
        interval_minutes: u32,
        /// Start of the active window in "HH:MM" format. If None, no lower bound.
        window_start: Option<String>,
        /// End of the active window in "HH:MM" format. If None, no upper bound.
        window_end: Option<String>,
    },
    /// Fires once when the voice assistant service starts.
    /// The scheduler evaluates Initial tasks on the first tick after
    /// the scheduler is started (either on startup if `auto_enable = true`
    /// or when enabled via MCP tool). Initial tasks are always treated
    /// as one-shot: they are deleted after firing.
    /// Example use cases: greeting the user, reading out today's
    /// weather forecast, summarizing appointments for the day.
    Initial,
}
```

### `AutonomousTask`

```rust
/// A task that the voice assistant executes autonomously when its trigger fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousTask {
    /// Unique identifier (UUID).
    pub id: String,
    /// The prompt text fed into the ReAct loop when the task fires.
    /// Example: "Schalte alle Lichter ein."
    pub prompt: String,
    /// When the task should fire.
    pub trigger: TriggerType,
    /// Whether the task fires once and is deleted, or fires repeatedly.
    pub one_shot: bool,
    /// Human-readable description for listing and logging.
    pub description: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 timestamp of the last firing, if any.
    pub last_fired: Option<String>,
    /// Minimum interval between firings in minutes (debounce).
    /// Prevents the scheduler from firing the same task twice in one tick.
    pub min_firing_interval_minutes: u32,
}
```

### Storage in SemanticMemory

Tasks are stored via `SemanticMemory::store`:

```rust
semantic_memory.store(
key: & task.id,
value: & serde_json::to_string( & task) ?,
category: FactCategory::Fact,  // Reuse existing category; task JSON is in value
)
```

The scheduler recalls tasks by querying `SemanticMemory::recall("autonomous task reminder schedule", 50)` and filtering for deserializable `AutonomousTask`
entries. Alternatively, a dedicated `recall_by_category` method can be added to `SemanticMemory` to filter by a new `FactCategory::AutonomousTask` variant.

---

## 4. Autonomous Scheduler Module

### New File: `services/voice_assistant/src/autonomous.rs`

This module contains the `AutonomousScheduler`, trigger evaluation logic, and task management functions.

### `AutonomousScheduler`

```rust
/// Errors that can occur during autonomous scheduling.
#[derive(Debug, thiserror::Error)]
pub enum AutonomousError {
    /// Failed to evaluate a trigger condition.
    #[error("Trigger evaluation failed: {0}")]
    TriggerEvaluation(String),
    /// Failed to store or recall a task from semantic memory.
    #[error("Memory operation failed: {0}")]
    Memory(String),
    /// Failed to parse a task from stored JSON.
    #[error("Task deserialization failed: {0}")]
    Deserialization(String),
}

/// The autonomous scheduler runs a periodic tick loop, evaluates task triggers,
/// and fires due tasks by sending SubmitText commands to the service.
pub struct AutonomousScheduler {
    /// Interval between scheduler ticks in seconds.
    tick_interval_seconds: u64,
    /// Whether the scheduler is enabled.
    enabled: Arc<Mutex<bool>>,
    /// Stop signal for the scheduler loop.
    stop_sender: Option<tokio::sync::oneshot::Sender<()>>,
}
```

### Scheduler Loop

The scheduler runs as a background tokio task and ticks every 60 seconds:

```rust
impl AutonomousScheduler {
    /// Starts the scheduler loop.
    /// Returns a stop sender for shutting down the scheduler.
    pub fn start(
        &mut self,
        command_sender: tokio::sync::mpsc::UnboundedSender<VoiceCommandMessage>,
        semantic_memory: SharedSemanticMemory,
    ) -> tokio::sync::oneshot::Sender<()> {
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let tick_interval = Duration::from_secs(self.tick_interval_seconds);
        let enabled = self.enabled.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_interval);
            let mut initial_fired = false;
            loop {
                tokio::select! {
                    _ = stop_rx => {
                        debug!("Autonomous scheduler: stopped");
                        return;
                    }
                    _ = interval.tick() => {
                        if !*enabled.lock().unwrap_or_else(|e| e.into_inner()) {
                            continue;
                        }
                        Self::tick(&command_sender, &semantic_memory, &mut initial_fired).await;
                    }
                }
            }
        });
        self.stop_sender = Some(stop_tx);
        stop_tx
    }
}
```

### Tick Evaluation

For each tick, the scheduler:

1. Recalls all `autonomous_task` facts from `SemanticMemory`.
2. Deserializes each fact value into `AutonomousTask`.
3. Evaluates the trigger condition against the current time.
4. Fires due tasks by sending `VoiceCommandMessage::submit_text(task.prompt)`.
5. Updates `last_fired` and deletes one-shot tasks.

```rust
async fn tick(
    command_sender: &tokio::sync::mpsc::UnboundedSender<VoiceCommandMessage>,
    semantic_memory: &SharedSemanticMemory,
    initial_fired: &mut bool,
) {
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_time = now.format("%H:%M").to_string();

    // Recall all tasks
    let tasks = match Self::recall_tasks(semantic_memory) {
        Ok(tasks) => tasks,
        Err(error) => {
            debug!("Autonomous scheduler: failed to recall tasks: {error}");
            return;
        }
    };

    for task in tasks {
        // Initial tasks only fire on the first tick after scheduler start
        if matches!(task.trigger, TriggerType::Initial) && *initial_fired {
            continue;
        }
        if Self::should_fire(&task, &current_time, &today) {
            debug!("Autonomous scheduler: firing task '{}' ({})", task.description, task.id);
            let _ = command_sender.send(VoiceCommandMessage::submit_text(&task.prompt));

            // Update last_fired or delete one-shot tasks
            Self::after_fire(semantic_memory, &task).await;
        }
    }

    // Mark Initial tasks as fired after the first tick
    *initial_fired = true;
}
```

### Trigger Evaluation

```rust
/// Evaluates whether a task's trigger condition is met.
fn should_fire(
    task: &AutonomousTask,
    current_time: &str,
    today: &str,
) -> bool {
    // Debounce: skip if fired within the last min_firing_interval_minutes
    if let Some(last_fired) = &task.last_fired {
        if let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_fired) {
            let elapsed = chrono::Local::now().signed_duration_since(last.with_timezone(&chrono::Local));
            if elapsed.num_minutes() < task.min_firing_interval_minutes as i64 {
                return false;
            }
        }
    }

    match &task.trigger {
        TriggerType::Absolute { time, date } => {
            if let Some(target_date) = date {
                // One-time event: match both date and time
                target_date == today && Self::time_matches(current_time, time, 1)
            } else {
                // Recurring daily: match time only
                Self::time_matches(current_time, time, 1)
            }
        }
        TriggerType::Initial => {
            // Always fires on the first tick after scheduler start.
            // The scheduler sets a flag on first tick to fire Initial tasks.
            // See `fire_initial_tasks` in the scheduler startup logic.
            true
        }
        TriggerType::Periodic { interval_minutes, window_start, window_end } => {
            // Check if current time is within the active window
            if let Some(start) = window_start {
                if current_time < start.as_str() {
                    return false;
                }
            }
            if let Some(end) = window_end {
                if current_time > end.as_str() {
                    return false;
                }
            }
            // Check if enough time has passed since last firing
            if let Some(last_fired) = &task.last_fired {
                if let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_fired) {
                    let elapsed = chrono::Local::now().signed_duration_since(last.with_timezone(&chrono::Local));
                    return elapsed.num_minutes() >= *interval_minutes as i64;
                }
            }
            // No last_fired: fire if within window
            true
        }
    }
}
```

### Time Matching Helper

The scheduler ticks every 60 seconds. To avoid missing a trigger when the tick doesn't align exactly with the target time, `time_matches` checks if the current
time is within ±`tolerance_minutes` of the target:

```rust
/// Checks if current_time is within tolerance_minutes of target_time.
/// Both times are in "HH:MM" format.
fn time_matches(current: &str, target: &str, tolerance_minutes: i32) -> bool {
    let current_minutes = Self::parse_time_to_minutes(current);
    let target_minutes = Self::parse_time_to_minutes(target);
    let diff = (current_minutes - target_minutes).abs();
    diff <= tolerance_minutes as i32
}
```

---

## 5. New State: `Autonomous`

Add a new variant to `AssistantState` in `model/voice_assistant/src/messages/state.rs`:

```rust
/// The assistant is executing an autonomously triggered task.
/// This state indicates that no user interaction initiated the pipeline;
/// the scheduler fired a stored task.
Autonomous,
```

### State Transitions

```
Idle ──(scheduler fires task)──▶ Autonomous
Autonomous ──(react loop starts)──▶ ThinkingLlm
ThinkingLlm ──(react loop completes)──▶ Idle (or Standby if wake word enabled)
Autonomous ──(error)──▶ Error ──▶ Idle
```

The `Autonomous` state is transient — it transitions to `ThinkingLlm` as soon as the ReAct loop begins. Its primary purpose is to signal the widget that this
pipeline run was scheduler-initiated, not user-initiated.

---

## 6. New Command Action

Add a new variant to `VoiceCommandAction` in `model/voice_assistant/src/messages/command.rs`:

```rust
/// An autonomous task fired by the scheduler. The text field contains
/// the task prompt. This is similar to SubmitText but carries the
/// semantic origin "autonomous" so the service can set the Autonomous state.
AutonomousTrigger,
```

### Command Handling in `service.rs`

```rust
VoiceCommandAction::AutonomousTrigger => {
let mut active = service_active.lock().unwrap_or_else( | e | e.into_inner());
if * active {
debug ! ("Voice Assistant: already active, ignoring autonomous trigger");
continue;
}
* active = true;
drop(active);

// Set Autonomous state to signal scheduler origin
Self::set_state( & service_state, AssistantState::Autonomous, & service_status_sender, & service_transcript, & service_answer).await;

Self::run_text_pipeline(
& message.text,
& service_config,
& service_state,
& service_llm,
& service_worker,
& service_entity_store,
& service_semantic_memory,
& service_conversation_history,
& service_tool_router,
& service_resource_router,
&service_prompt_router,
& service_training_mode,
& service_active_trace,
& service_training_history,
& service_transcript,
& service_answer,
& service_response_type,
& service_active,
& service_tts,
& service_pending,
&service_pending_resources,
& service_pending_prompts,
& service_tool_catalog,
& service_resource_catalog,
& service_prompt_catalog,
& service_core_context,
& service_meta,
& service_status_sender,
)
.await;
}
```

Add a constructor to `VoiceCommandMessage`:

```rust
pub fn autonomous_trigger(text: &str) -> Self {
    Self::new(VoiceCommandAction::AutonomousTrigger, text)
}
```

---

## 7. Integration into `VoiceAssistantService`

### 7.1 State Additions

In `services/voice_assistant/src/service.rs`, add:

```rust
pub struct VoiceAssistantService {
    // ... existing fields ...

    /// Whether autonomous scheduling is enabled.
    autonomous_enabled: Arc<Mutex<bool>>,
    /// The autonomous scheduler handle, active when enabled.
    autonomous_scheduler: Option<AutonomousSchedulerHandle>,
    /// Autonomous scheduler configuration.
    autonomous_config: AutonomousConfig,
}
```

### 7.2 Scheduler Handle

```rust
/// Handle for the running autonomous scheduler.
struct AutonomousSchedulerHandle {
    /// Stop sender to terminate the scheduler loop.
    stop_sender: tokio::sync::oneshot::Sender<()>,
}
```

### 7.3 Initialization

During `VoiceAssistantService::new()`, after semantic memory is initialized:

```rust
// Initialize autonomous scheduler
let autonomous_config = AutonomousConfig::default ();
if autonomous_config.auto_enable {
let scheduler = AutonomousScheduler::new(autonomous_config.clone());
let stop_tx = scheduler.start(
service.command_sender.clone().unwrap(),
service.semantic_memory.clone(),
);
service.autonomous_scheduler = Some(AutonomousSchedulerHandle { stop_sender: stop_tx });
* service.autonomous_enabled.lock().unwrap() = true;
}
```

---

## 8. Configuration

### Additions to `VoiceAssistantServiceConfig`

In `services/voice_assistant/src/config.rs`:

```rust
/// Configuration for autonomous actions.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AutonomousConfig {
    /// Whether autonomous scheduling is enabled on startup.
    pub auto_enable: bool,
    /// Scheduler tick interval in seconds.
    pub tick_interval_seconds: u64,
    /// Whether to speak autonomous task results via TTS.
    pub tts_enabled: bool,
    /// Minimum firing interval in minutes (debounce for all tasks).
    pub min_firing_interval_minutes: u32,
}

impl Default for AutonomousConfig {
    fn default() -> Self {
        Self {
            auto_enable: true,
            tick_interval_seconds: 60,
            tts_enabled: true,
            min_firing_interval_minutes: 1,
        }
    }
}
```

Add to `VoiceAssistantServiceConfig`:

```rust
pub struct VoiceAssistantServiceConfig {
    // ... existing fields ...

    /// Autonomous actions configuration.
    pub autonomous: AutonomousConfig,
}
```

### Enabling / Disabling via Config File

Autonomous actions can be turned on or off via the config file. The `auto_enable` field controls whether the scheduler starts on service startup:

- **`auto_enable = true`**: The `AutonomousScheduler` is started during `VoiceAssistantService::new()`. Tasks are evaluated and fired automatically.
- **`auto_enable = false`**: The scheduler is not started. No autonomous tasks fire. The MCP tools (`autonomous_create`, `autonomous_list`, `autonomous_delete`)
  remain available — tasks can still be created and stored — but they will not execute until the scheduler is enabled (either by changing the config and
  restarting, or at runtime via the `voice_assistant_autonomous_enable` MCP tool).

This allows the user to disable autonomous actions globally without deleting stored tasks. For example, a user who only wants the voice assistant to act on
explicit commands can set:

```toml
[autonomous]
auto_enable = false
```

Tasks created while the scheduler is disabled remain in `SemanticMemory` and will fire once the scheduler is enabled (either via config restart or runtime MCP
tool).

### Example `config.toml` Section

```toml
[[services]]
name = "voice-assistant"
type = "voice_assistant"

[autonomous]
auto_enable = true
tick_interval_seconds = 60
tts_enabled = true
min_firing_interval_minutes = 1
```

### Disabled Example

```toml
[[services]]
name = "voice-assistant"
type = "voice_assistant"

[autonomous]
auto_enable = false
```

---

## 9. MCP Tools

All tools are registered in `services/voice_assistant/src/mcp.rs` as `RegisterToolMessage` broadcasts and handled in the `InvokeToolMessage` handler.

### 9.1 `voice_assistant_autonomous_create`

**Description:** Creates a new autonomous task that the assistant will execute when its trigger fires.

**Input schema:**

```json
{
  "type": "object",
  "properties": {
    "prompt": {
      "type": "string",
      "description": "The prompt text fed into the ReAct loop when the task fires. Example: 'Schalte alle Lichter ein.'"
    },
    "description": {
      "type": "string",
      "description": "Human-readable description of the task."
    },
    "trigger_type": {
      "type": "string",
      "enum": [
        "absolute",
        "periodic",
        "initial"
      ],
      "description": "Type of trigger. Use 'absolute' for time-based triggers (the LLM should look up sun event times, weather data, etc. via tools and pass the resolved time). Use 'periodic' for recurring checks within a time window. Use 'initial' for tasks that fire once when the voice assistant starts."
    },
    "trigger_params": {
      "type": "object",
      "description": "Trigger-specific parameters. For 'absolute': {time: 'HH:MM', date: 'YYYY-MM-DD'}. For 'periodic': {interval_minutes: 120, window_start: '08:00', window_end: '20:00'}. For 'initial': {} (no parameters needed)."
    },
    "one_shot": {
      "type": "boolean",
      "description": "If true, the task fires once and is deleted. If false, it fires repeatedly."
    }
  },
  "required": [
    "prompt",
    "description",
    "trigger_type",
    "trigger_params"
  ]
}
```

**Behavior:**

- Constructs an `AutonomousTask` with a UUID.
- Serializes the task and stores it in `SemanticMemory` with key = task ID.
- Returns `{"status": "ok", "task_id": "uuid"}`.

### 9.2 `voice_assistant_autonomous_list`

**Description:** Lists all stored autonomous tasks.

**Input schema:**

```json
{
  "type": "object",
  "properties": {}
}
```

**Output schema:**

```json
{
  "type": "object",
  "properties": {
    "tasks": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string"
          },
          "prompt": {
            "type": "string"
          },
          "description": {
            "type": "string"
          },
          "trigger_type": {
            "type": "string"
          },
          "trigger_params": {
            "type": "object"
          },
          "one_shot": {
            "type": "boolean"
          },
          "last_fired": {
            "type": "string"
          },
          "created_at": {
            "type": "string"
          }
        }
      }
    }
  }
}
```

**Behavior:**

- Recalls all `autonomous_task` facts from `SemanticMemory`.
- Deserializes each into `AutonomousTask`.
- Returns the list as JSON.

### 9.3 `voice_assistant_autonomous_delete`

**Description:** Deletes an autonomous task by its ID.

**Input schema:**

```json
{
  "type": "object",
  "properties": {
    "task_id": {
      "type": "string",
      "description": "The UUID of the task to delete."
    }
  },
  "required": [
    "task_id"
  ]
}
```

**Behavior:**

- Removes the task from `SemanticMemory` by key (task ID).
- Returns `{"status": "ok", "task_id": "uuid"}`.

### 9.4 `voice_assistant_autonomous_enable`

**Description:** Enables the autonomous scheduler. The assistant will start evaluating task triggers.

**Input schema:**

```json
{
  "type": "object",
  "properties": {}
}
```

**Behavior:**

- Sets `autonomous_enabled` to `true`.
- Starts the scheduler loop if not already running.
- Returns `{"status": "ok"}`.

### 9.5 `voice_assistant_autonomous_disable`

**Description:** Disables the autonomous scheduler. No tasks will fire until re-enabled.

**Input schema:**

```json
{
  "type": "object",
  "properties": {}
}
```

**Behavior:**

- Sets `autonomous_enabled` to `false`.
- Stops the scheduler loop.
- Returns `{"status": "ok"}`.

---

## 10. LLM-Directed Task Creation

### How the LLM Creates Tasks

When the user says "Schalte die Lichter bei Sonnenuntergang ein", the ReAct loop processes this as a normal user prompt. The LLM has access to the
`voice_assistant_autonomous_create` tool (registered via MCP) and all existing tools (weather, memory, entity states, etc.). The LLM's reasoning chain would be:

```
Thought: The user wants the lights turned on at sunset. This is a future action, not an immediate one.
         I need to find today's sunset time first.
Action: resource:weather://forecast
Observation: {"daily": {"sunset": ["2026-07-15T21:43"]}}
Thought: Sunset is at 21:43 today. I should create an autonomous task with an Absolute trigger at 21:43.
Action: tool:voice_assistant_autonomous_create
Parameters: {
  "prompt": "Schalte alle Lichter ein.",
  "description": "Lichter bei Sonnenuntergang einschalten",
  "trigger_type": "absolute",
  "trigger_params": {"time": "21:43", "date": null},
  "one_shot": false
}
Observation: {"status": "ok", "task_id": "abc-123"}
Final Answer: Ich habe eine Aufgabe erstellt: Die Lichter werden jeden Tag um 21:43 Uhr eingeschaltet (heutiger Sonnenuntergang).
```

### LLM-Planned Condition-Based Triggers

The LLM can create `Periodic` triggers with conditional prompts. The prompt itself contains the condition, and the ReAct loop evaluates it when the task fires:

```
User: "Schalte den Ventilator ein, wenn die Temperatur über 25°C steigt."
LLM: Creates AutonomousTask {
    prompt: "Prüfe die aktuelle Temperatur. Wenn sie über 25°C liegt, schalte den Ventilator ein. Wenn nicht, tue nichts.",
    trigger: Periodic {
        interval_minutes: 30,
        window_start: Some("08:00"),
        window_end: Some("22:00"),
    },
    one_shot: false,
}
```

When the periodic trigger fires, the ReAct loop processes the prompt, checks the temperature via a tool, and either turns on the fan or does nothing.

### System Prompt Extension

The system prompt should include a brief instruction about autonomous task creation:

```
If the user asks for a time-based, recurring, or delayed action, use the
voice_assistant_autonomous_create tool to schedule it. Do not attempt to
execute the action immediately if the user specifies a future time or
recurring condition. To resolve relative times (e.g. "at sunset"), use
available tools (weather, memory, etc.) to look up the concrete time first,
then create an Absolute trigger with the resolved time.
```

---

## 11. Task Lifecycle

### Creation

1. User speaks or types a request involving a future or recurring action.
2. LLM calls `voice_assistant_autonomous_create` tool.
3. Task is stored in `SemanticMemory` (SQLite + embedding).
4. Task survives process restarts (persisted in SQLite).

### Firing

1. Scheduler tick evaluates all tasks.
2. If trigger condition is met:
   a. Scheduler sends `VoiceCommandMessage::autonomous_trigger(task.prompt)`. b. Service sets `Autonomous` state, then runs the ReAct loop. c. ReAct loop
   executes tools (e.g. `button_press` to toggle lights). d. TTS speaks the result (if `tts_enabled`).
3. Scheduler updates `last_fired` timestamp.

### Deletion

- **One-shot tasks**: Deleted from `SemanticMemory` after first firing.
- **Recurring tasks**: Remain in memory. Deleted only via `voice_assistant_autonomous_delete` tool.
- **Manual deletion**: User can say "Lösche die Aufgabe, die Lichter bei Sonnenuntergang einzuschalten" and the LLM calls `voice_assistant_autonomous_delete`.

### Update

To modify a task, the LLM deletes the old task and creates a new one. No dedicated update tool is needed.

---

## 12. Dependencies

### New Crate Dependencies

No new crate dependencies are needed. The scheduler uses `tokio` (already present), `chrono` (already present), `serde` (already present), and `uuid` (already
present).

---

## 13. File Additions

| File                                            | Purpose                                                                                                                                          |
|-------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| `services/voice_assistant/src/autonomous.rs`    | `AutonomousScheduler`, `AutonomousTask`, `TriggerType`, `AutonomousConfig`, `AutonomousError`                                                    |
| `services/voice_assistant/src/lib.rs`           | Add `pub(crate) mod autonomous;`                                                                                                                 |
| `services/voice_assistant/src/service.rs`       | Add `autonomous_enabled`, `autonomous_scheduler`, `autonomous_config` fields; handle `AutonomousTrigger` action; initialize scheduler in `new()` |
| `services/voice_assistant/src/config.rs`        | Add `AutonomousConfig` struct and field in `VoiceAssistantServiceConfig`                                                                         |
| `services/voice_assistant/src/mcp.rs`           | Register and handle the five autonomous MCP tools                                                                                                |
| `model/voice_assistant/src/messages/state.rs`   | Add `Autonomous` variant to `AssistantState`                                                                                                     |
| `model/voice_assistant/src/messages/command.rs` | Add `AutonomousTrigger` variant to `VoiceCommandAction`                                                                                          |

---

## 14. Security & Privacy

- **Autonomous actions are opt-in**: The scheduler is disabled by default if `autonomous.auto_enable = false`. The user must explicitly enable it via config or
  MCP tool.
- **Task persistence**: Tasks are stored in SQLite (`memory.db`). They survive restarts. The user can inspect and delete tasks at any time via MCP tools.
- **No elevated permissions**: Autonomous tasks use the same ReAct loop and tool set as user-initiated commands. No special privileges are granted.
- **TTS announcements**: When a task fires, the assistant may speak the result. This could be undesirable in certain situations (e.g. late at night). The
  `tts_enabled` config flag controls this. A future "quiet hours" feature could suppress TTS during specific time ranges.
- **Prompt injection risk**: Task prompts are stored as plain text. A malicious prompt stored in a task could instruct the LLM to perform unwanted actions.
  Mitigations:
    - Task prompts are user-created (via LLM tool call from user speech/text).
    - The ReAct loop enforces the same tool whitelist and safety checks for autonomous tasks as for user-initiated commands.
    - A future "task confirmation" feature could require user approval before a one-shot task fires.

---

## 15. Edge Cases & Error Handling

| Scenario                                                           | Handling                                                                                                                                         |
|--------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| Scheduler tick misses exact trigger time                           | `time_matches` uses ±1 minute tolerance; tick interval is 60 seconds                                                                             |
| Task fires while pipeline is active                                | Scheduler checks `active` flag; if active, skip (will retry next tick)                                                                           |
| Task deserialization fails (corrupted JSON)                        | Skip the task; log error; do not crash the scheduler                                                                                             |
| SemanticMemory is unavailable                                      | Scheduler logs error and skips tick; retries next tick                                                                                           |
| Task prompt is empty                                               | Scheduler skips the task; log warning                                                                                                            |
| Multiple tasks fire in the same tick                               | Tasks are processed sequentially; each sends a `SubmitText` command; the service processes them one at a time via the command channel            |
| Process restarts while a task is due                               | Task persists in SQLite; scheduler picks it up on next tick after restart; `last_fired` debounce prevents immediate re-fire if it fired recently |
| User deletes a task while scheduler is evaluating                  | `SemanticMemory` delete removes the entry; next tick won't find it                                                                               |
| Timezone changes                                                   | All times are local (`chrono::Local`)                                                                                                            |
| LLM creates task with stale time (e.g. sunset time from yesterday) | Task fires at the stale time; user can delete and recreate; a future enhancement could add task expiry                                           |
| Periodic condition prompt always evaluates to "do nothing"         | Task still fires periodically but the ReAct loop produces no action; user can delete the task                                                    |
| Initial task fires while pipeline is already active                | Scheduler checks `active` flag; if active, the Initial task is skipped and retried on the next tick; it is deleted once it fires successfully    |

---

## 16. Interaction with Existing Features

### Wake Word Mode

If wake word mode is enabled, the scheduler and wake word detector coexist:

- The scheduler sends `AutonomousTrigger` commands via the same `command_sender` channel.
- If the pipeline is busy (wake word triggered a pipeline), the scheduler's command waits in the channel queue.
- After an autonomous task completes, the service returns to `Standby` if wake word is enabled, or `Idle` otherwise.

### Training Mode

Autonomous task executions are recorded as training traces if training mode is active. This allows the user to evaluate how well the LLM performs autonomous
tasks without user interaction.

### Memory Distillation

Successful autonomous task executions can be distilled into habits, just like user-initiated traces. For example, if the LLM consistently creates a sunset
trigger at the right time, the distillation process can create a `user_habit` fact that optimizes future trigger creation.

### Conversation History

Autonomous task prompts are added to the conversation history, just like user-initiated text. This allows the user to refer to previous autonomous actions ("Was
hast du gerade gemacht?").

---

## 17. Example Scenarios

### Scenario 1: Sunset Lights (LLM-Planned Trigger)

```
User: "Schalte die Lichter bei Sonnenuntergang ein."
LLM: [calls weather://forecast resource to look up sunset time]
      [sunset today is 21:43]
      [creates AutonomousTask with Absolute trigger at 21:43]
LLM: "Ich habe eine Aufgabe erstellt. Die Lichter werden um 21:43 Uhr eingeschaltet (heutiger Sonnenuntergang)."

Note: The task fires daily at 21:43. The user can ask to update it
      when sunset time changes significantly. A future enhancement
      could let the LLM periodically refresh the trigger time.

[At 21:43, scheduler fires]
Scheduler: sends AutonomousTrigger("Schalte alle Lichter ein.")
Service: runs ReAct loop → calls button_press tool → lights turn on
TTS: "Ich habe die Lichter eingeschaltet."
```

### Scenario 2: Appointment Reminder

```
User: "Erinnere mich um 18:00 Uhr an meinen Termin."
LLM: Creates AutonomousTask {
    prompt: "Erinnere den Benutzer an seinen Termin um 18:00 Uhr.",
    trigger: Absolute { time: "18:00", date: None },
    one_shot: true,
}
LLM: "Ich werde dich um 18:00 Uhr an deinen Termin erinnern."

[At 18:00, scheduler fires]
Scheduler: sends AutonomousTrigger("Erinnere den Benutzer an seinen Termin um 18:00 Uhr.")
Service: runs ReAct loop → final answer
TTS: "Dein Termin um 18:00 Uhr beginnt jetzt."
[Task is deleted after firing]
```

### Scenario 3: Daytime Internet Radio

```
User: "Spiele tagsüber Internet Radio."
LLM: Creates AutonomousTask {
    prompt: "Starte Internet Radio.",
    trigger: Periodic {
        interval_minutes: 240,
        window_start: Some("08:00"),
        window_end: Some("20:00"),
    },
    one_shot: false,
}
LLM: "Ich werde tagsüber alle 4 Stunden Internet Radio starten."

[At 08:00, scheduler fires]
Scheduler: sends AutonomousTrigger("Starte Internet Radio.")
Service: runs ReAct loop → calls mpris_play or terminal_command tool
TTS: "Ich habe Internet Radio gestartet."
```

### Scenario 5: Startup Greeting (Initial Trigger)

```
User: "Wenn du startest, sag mir bitte guten Morgen und lies das Wetter vor."
LLM: Creates AutonomousTask {
    prompt: "Sag dem Benutzer guten Morgen und lies die heutige Wettervorhersage vor.",
    trigger: Initial,
    one_shot: true,
}
LLM: "Ich habe eine Aufgabe erstellt. Beim nächsten Start werde ich dir guten Morgen sagen und das Wetter vorlesen."

[Voice assistant starts, scheduler fires Initial tasks on first tick]
Scheduler: sends AutonomousTrigger("Sag dem Benutzer guten Morgen und lies die heutige Wettervorhersage vor.")
Service: runs ReAct loop → calls weather://forecast resource → TTS speaks greeting + forecast
[Task is deleted after firing]
```

### Scenario 4: Condition-Based Trigger (LLM-Planned)

```
User: "Schalte den Ventilator ein, wenn die Temperatur über 25°C steigt."
LLM: Creates AutonomousTask {
    prompt: "Prüfe die aktuelle Temperatur über die Wetterdaten. Wenn sie über 25°C liegt, schalte den Ventilator ein. Wenn nicht, tue nichts.",
    trigger: Periodic {
        interval_minutes: 30,
        window_start: Some("08:00"),
        window_end: Some("22:00"),
    },
    one_shot: false,
}
LLM: "Ich werde alle 30 Minuten die Temperatur prüfen und den Ventilator einschalten, wenn es wärmer als 25°C ist."

[Every 30 minutes between 08:00 and 22:00, scheduler fires]
Scheduler: sends AutonomousTrigger("Prüfe die aktuelle Temperatur...")
Service: runs ReAct loop → calls weather resource → checks temperature → may or may not call button_press
```

---

## 18. Future Extensions

- **Quiet hours**: Suppress TTS announcements during configurable time ranges (e.g. 22:00–07:00).
- **Task confirmation**: Require user confirmation before a one-shot task fires (e.g. "Dein Termin ist in 15 Minuten. Soll ich dich erinnern?").
- **Trigger refresh**: Let the LLM periodically update `Absolute` triggers (e.g. refresh sunset time daily) by recreating the task with a new time.
- **Task chaining**: Fire a sequence of tasks in order (e.g. "turn on lights, then start music, then set volume to 30").
- **Natural language time parsing**: Let the LLM parse "in 30 Minuten" or "morgen früh" into `TriggerType` automatically.
- **Calendar integration**: Pull appointments from a calendar service and create reminder tasks automatically.
- **Task templates**: Pre-defined task templates for common scenarios (e.g. "good morning routine" = turn on lights + start radio + read weather).
- **Geofencing**: Fire tasks when the user enters or leaves a location.
- **Task statistics**: Track how often each task fires, success rate, and average execution time.
- **Webhook triggers**: Fire tasks when an external webhook is received (e.g. smart home event, calendar notification).
- **Event-based triggers**: Extend `TriggerType` with entity-state-change triggers (e.g. "fire when presence sensor changes to 'home'").
