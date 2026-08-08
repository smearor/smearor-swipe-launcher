use serde::Serialize;

/// A single content item in a tool response.
#[derive(Serialize)]
pub struct ToolContent {
    /// The content type, always "text" for now.
    #[serde(rename = "type")]
    pub content_type: &'static str,
    /// The text payload.
    pub text: String,
}

impl ToolContent {
    /// Create a new text content item.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text",
            text: text.into(),
        }
    }
}

/// The result payload of a tool invocation, returned as the `result` field in a JSON-RPC response.
#[derive(Serialize)]
pub struct ToolResultPayload {
    /// Array of content items.
    pub content: Vec<ToolContent>,
    /// Whether the tool invocation resulted in an error.
    #[serde(rename = "isError")]
    pub is_error: bool,
}

impl ToolResultPayload {
    /// Create a successful tool result with a single text content item.
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: false,
        }
    }

    /// Create an error tool result with a single text content item.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(message)],
            is_error: true,
        }
    }
}
