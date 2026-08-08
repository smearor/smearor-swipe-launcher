use serde::Serialize;

/// SDK-compatible text content block for a prompt message.
///
/// Matches the `ContentBlock::TextContent` variant expected by `rust_mcp_sdk`.
#[derive(Serialize)]
pub struct SdkPromptContent {
    /// Content type discriminator, always "text".
    #[serde(rename = "type")]
    pub content_type: &'static str,
    /// The prompt message text.
    pub text: String,
}

/// SDK-compatible prompt message for JSON serialization.
///
/// The SDK's `Role` enum only supports "user" and "assistant" — "system" must
/// be mapped to "user". This struct ensures the broker produces JSON that
/// `serde_json::from_str::<GetPromptResult>` can deserialize correctly.
#[derive(Serialize)]
pub struct SdkPromptMessage {
    /// Message role: "user" or "assistant".
    pub role: &'static str,
    /// Text content block.
    pub content: SdkPromptContent,
}

/// SDK-compatible prompt result for JSON serialization.
///
/// Matches `rust_mcp_sdk::schema::GetPromptResult` for deserialization after
/// the broker resolves a plugin prompt invocation.
#[derive(Serialize)]
pub struct SdkPromptResult {
    /// Resolved prompt messages.
    pub messages: Vec<SdkPromptMessage>,
}

impl SdkPromptMessage {
    /// Create an SDK-compatible prompt message from a model `PromptMessage`.
    ///
    /// Maps "system" role to "user" since the SDK's `Role` enum has no system variant.
    pub fn from_prompt_message(role: &str, content: &str) -> Self {
        let sdk_role = match role {
            "assistant" => "assistant",
            _ => "user",
        };
        Self {
            role: sdk_role,
            content: SdkPromptContent {
                content_type: "text",
                text: content.to_string(),
            },
        }
    }
}
