use serde::Deserialize;
use serde::Serialize;

/// Parsed output from the LLM during a ReAct loop iteration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LlmResponse {
    /// The LLM requests a tool call.
    ToolCall {
        /// Name of the tool to invoke.
        tool: String,
        /// JSON-encoded arguments for the tool.
        arguments: serde_json::Value,
    },
    /// The LLM has reached a final answer.
    FinalAnswer {
        /// The final response text for the user.
        answer: String,
    },
}
