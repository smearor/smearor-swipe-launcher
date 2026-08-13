mod mcp;
mod messages;
mod topics;
mod view;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::ThemeMcpPrompts;
pub use mcp::requests::SetThemeArgs;
pub use mcp::resources::ThemeMcpResources;
pub use mcp::tools::ThemeMcpTools;
pub use messages::command::ThemeCommandAction;
pub use messages::command::ThemeCommandMessage;
pub use messages::status::ThemeStatusMessage;
pub use messages::theme::Theme;
pub use messages::theme_colors::ThemeColors;
pub use messages::theme_colors::ThemePalette;
pub use messages::theme_info::ThemeColorsStabby;
pub use messages::theme_info::ThemeInfo;
pub use messages::theme_info::ThemePaletteStabby;
pub use messages::theme_mode::ThemeMode;
pub use topics::TOPIC_COMMAND;
pub use topics::TOPIC_STATUS;
pub use view::ThemeView;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ThemeStatusMessageConverter, ThemeStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ThemeCommandMessageConverter, ThemeCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for theme messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    ThemeStatusMessageConverter::register_in_host(context);
    ThemeCommandMessageConverter::register_in_host(context);
}
