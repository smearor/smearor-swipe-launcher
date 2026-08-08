//! Core prompt definitions, each implementing `PromptDefinitionCreator`.

mod area_control_help;
mod broker_message_guide;
mod launcher_overview;
mod tool_shortcut_guide;

pub use area_control_help::AreaControlHelpPrompt;
pub use broker_message_guide::BrokerMessageGuidePrompt;
pub use launcher_overview::LauncherOverviewPrompt;
pub use tool_shortcut_guide::ToolShortcutGuidePrompt;
