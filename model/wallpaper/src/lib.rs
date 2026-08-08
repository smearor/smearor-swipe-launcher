mod mcp;
mod messages;
mod topics;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::WallpaperMcpPrompts;
pub use mcp::requests::AddWallpaperThemeArgs;
pub use mcp::requests::RemoveWallpaperThemeArgs;
pub use mcp::requests::SelectWallpaperThemeArgs;
pub use mcp::resources::WallpaperMcpResources;
pub use mcp::tools::WallpaperMcpTools;
pub use messages::app_config::AppConfig;
pub use messages::command::WallpaperCommandAction;
pub use messages::command::WallpaperCommandMessage;
pub use messages::image_config::ImageConfig;
pub use messages::monitor_process::MonitorProcess;
pub use messages::status::WallpaperStatusMessage;
pub use messages::theme::WallpaperTheme;
pub use messages::theme_config::WallpaperThemeConfig;
pub use messages::theme_info::WallpaperThemeInfo;
pub use messages::video_config::VideoConfig;
pub use messages::wallpaper_type::WallpaperType;
pub use messages::wallpaper_type::wallpaper_type_icon;
pub use topics::TOPIC_COMMAND;
pub use topics::TOPIC_STATUS;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WallpaperCommandMessageConverter, WallpaperCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WallpaperStatusMessageConverter, WallpaperStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for wallpaper messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    WallpaperCommandMessageConverter::register_in_host(context);
    WallpaperStatusMessageConverter::register_in_host(context);
}
