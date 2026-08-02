use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

/// A single step recorded during a ReAct loop iteration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReActStep {
    /// Iteration number inside the ReAct loop.
    pub iteration: usize,
    /// Raw LLM output before parsing.
    pub thought: String,
    /// Chain-of-thought reasoning extracted from `<|channel>thought` blocks.
    /// Only present for models that emit CoT tokens (e.g. Gemma 4 12B).
    pub cot: Option<String>,
    /// Parsed action kind, e.g. "tool:voice_assistant_recall_memory" or "resource:mpris://metadata".
    pub action: String,
    /// Raw JSON arguments or URI for the action.
    pub parameters: String,
    /// Result of the tool/resource execution, or error text.
    pub observation: String,
    /// Final answer or clarify text produced in this step, if any.
    pub answer: Option<String>,
    /// Timestamp of this step.
    pub timestamp: f64,
}

/// A complete training trace for one user interaction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TrainingTrace {
    /// Stable, globally unique identifier for this trace.
    pub id: String,
    /// User input that started this trace.
    pub user_text: String,
    /// When the trace started (epoch seconds).
    pub start_time: f64,
    /// When the trace ended, if it has ended (epoch seconds).
    pub end_time: Option<f64>,
    /// All recorded steps.
    pub steps: Vec<ReActStep>,
    /// Whether the trace ended with a final answer.
    pub success: Option<bool>,
    /// Optional label supplied by the caller for dataset grouping.
    pub label: Option<String>,
}

impl TrainingTrace {
    /// Creates a new trace with the given user text and label.
    /// The trace ID is generated immediately so it can be referenced
    /// during the active session before finalize is called.
    pub fn new(user_text: &str, label: Option<String>) -> Self {
        Self {
            id: generate_trace_id(),
            user_text: user_text.to_string(),
            start_time: current_epoch(),
            end_time: None,
            steps: Vec::new(),
            success: None,
            label,
        }
    }

    /// Records a single ReAct step.
    pub fn add_step(&mut self, iteration: usize, thought: &str, cot: Option<&str>, action: &str, parameters: &str, observation: &str, answer: Option<&str>) {
        self.steps.push(ReActStep {
            iteration,
            thought: thought.to_string(),
            cot: cot.map(|c| c.to_string()),
            action: action.to_string(),
            parameters: parameters.to_string(),
            observation: observation.to_string(),
            answer: answer.map(|a| a.to_string()),
            timestamp: current_epoch(),
        });
    }

    /// Finalizes the trace with a success flag.
    pub fn finalize(&mut self, success: bool) {
        self.end_time = Some(current_epoch());
        self.success = Some(success);
    }
}

/// In-memory store for training traces, keyed by trace ID.
pub type TrainingHistory = Arc<Mutex<BTreeMap<String, TrainingTrace>>>;

/// Creates a new empty training history.
pub fn new_training_history() -> TrainingHistory {
    Arc::new(Mutex::new(BTreeMap::new()))
}

/// Generates a stable trace ID: `trace-{timestamp}-{uuid_suffix}`.
fn generate_trace_id() -> String {
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H-%M-%SZ");
    let uuid_suffix = &uuid::Uuid::new_v4().to_string()[..8];
    format!("trace-{timestamp}-{uuid_suffix}")
}

/// Returns the current epoch time in seconds with sub-second precision.
fn current_epoch() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Retrieves traces from the history, optionally filtered by label or user text substring.
/// Returns at most `limit` traces, most recent first.
pub fn query_traces(history: &TrainingHistory, limit: usize, label_filter: Option<&str>, query_filter: Option<&str>) -> Vec<TrainingTrace> {
    let history = match history.lock() {
        Ok(history) => history,
        Err(error) => {
            debug!("Voice Assistant: training history lock poisoned: {error}");
            return Vec::new();
        }
    };

    let mut traces: Vec<TrainingTrace> = history
        .values()
        .filter(|trace| {
            if let Some(label) = label_filter {
                if trace.label.as_deref() != Some(label) {
                    return false;
                }
            }
            if let Some(query) = query_filter {
                if !trace.user_text.to_lowercase().contains(&query.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    traces.reverse();
    traces.truncate(limit);
    traces
}

/// Retrieves a single trace by ID from the history.
pub fn get_trace_by_id(history: &TrainingHistory, trace_id: &str) -> Option<TrainingTrace> {
    history.lock().ok()?.get(trace_id).cloned()
}

/// Retrieves the active (not-yet-finalized) trace by ID.
pub fn get_active_trace(active_trace: &Arc<Mutex<Option<TrainingTrace>>>, trace_id: &str) -> Option<TrainingTrace> {
    let guard = active_trace.lock().ok()?;
    guard.as_ref().filter(|t| t.id == trace_id).cloned()
}
