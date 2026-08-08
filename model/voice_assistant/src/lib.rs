mod mcp;
mod messages;
mod paths;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use paths::xdg_config_path;
pub use paths::xdg_models_dir;

pub use mcp::prompts::VoiceAssistantMcpPrompts;
pub use mcp::requests::MemoryForgetArgs;
pub use mcp::requests::MemoryListArgs;
pub use mcp::requests::MemoryQueryArgs;
pub use mcp::requests::MemoryRecallArgs;
pub use mcp::requests::MemoryStoreArgs;
pub use mcp::requests::MemoryStoreBatchArgs;
pub use mcp::requests::ResourceDiscoveryGuideArgs;
pub use mcp::requests::VoiceAssistantSaveSystemPromptArgs;
pub use mcp::requests::VoiceAssistantSetMaxTokensArgs;
pub use mcp::requests::VoiceAssistantSetRollingWindowArgs;
pub use mcp::requests::VoiceAssistantSetSystemPromptArgs;
pub use mcp::requests::VoiceAssistantSetThresholdArgs;
pub use mcp::requests::VoiceAssistantSetWakeWordModelArgs;
pub use mcp::requests::VoiceAssistantSpeakArgs;
pub use mcp::requests::VoiceAssistantSubmitTextArgs;
pub use mcp::requests::VoiceAssistantSwitchModelArgs;
pub use mcp::requests::VoiceAssistantTrainingGetArgs;
pub use mcp::requests::VoiceAssistantTrainingStartArgs;
pub use mcp::resources::VoiceAssistantMcpResources;
pub use mcp::tools::VoiceAssistantMcpTools;
pub use messages::command::VoiceCommandAction;
pub use messages::command::VoiceCommandMessage;
pub use messages::command::VoiceCommandMessageStabby;
pub use messages::llm_response::LlmResponse;
pub use messages::llm_response::NewInsight;
pub use messages::prompt_catalog::PromptCatalogEntry;
pub use messages::resource_catalog::ResourceCatalogEntry;
pub use messages::state::AssistantState;
pub use messages::status::AssistantStatusMessage;
pub use messages::status::AssistantStatusMessageStabby;
pub use messages::tool_catalog::ToolCatalogEntry;
pub use messages::tool_result::ToolError;
pub use messages::tool_result::ToolResult;
pub use messages::topics::TOPIC_COMMAND;
pub use messages::topics::TOPIC_STATUS;
pub use messages::tts_config::TtsConfig;
pub use messages::tts_config::TtsModelType;
pub use messages::tts_config::TtsPhonemizerConfig;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(VoiceCommandMessageConverter, VoiceCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

smearor_swipe_launcher_plugin_api::impl_json_convertible!(VoiceCommandMessageStabbyConverter, VoiceCommandMessageStabby, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AssistantStatusMessageConverter, AssistantStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AssistantStatusMessageStabbyConverter, AssistantStatusMessageStabby, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for voice assistant messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    VoiceCommandMessageConverter::register_in_host(context);
    AssistantStatusMessageConverter::register_in_host(context);
}
