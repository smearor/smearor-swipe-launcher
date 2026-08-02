use serde::Deserialize;
use serde::Serialize;

/// A learning insight extracted by the LLM during a ReAct iteration.
/// Automatically stored in semantic memory after the final answer is produced.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewInsight {
    /// Short key for the fact (e.g. "office_lights_layout").
    pub key: String,
    /// The fact content (e.g. "office_area has 3 lights: desk, window, kommode").
    pub value: String,
    /// Category: "fact", "preference", or "habit".
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "fact".to_string()
}

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
    /// The LLM requests an MCP resource read.
    ResourceRead {
        /// URI of the resource to read (e.g., "audio://volume").
        resource: String,
    },
    /// The LLM has reached a final answer.
    FinalAnswer {
        /// The final response text for the user.
        answer: String,
        /// Insights learned during this request, automatically stored in
        /// semantic memory. Empty when the LLM has nothing new to persist.
        #[serde(default)]
        new_insights: Vec<NewInsight>,
    },
    /// The LLM asks the user a clarifying question.
    Clarify {
        /// The clarifying question for the user.
        question: String,
    },
    /// The LLM has converted the final answer into TTS-ready text.
    TextToSpeechAnswer {
        /// TTS-ready text with all symbols written as spoken words.
        text: String,
    },
}
