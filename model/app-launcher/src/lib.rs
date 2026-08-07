mod app_info;
mod mcp;
mod messages;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use app_info::AppInfo;
pub use app_info::AvailableAppsResponse;
pub use app_info::Pagination;
pub use mcp::prompts::AppLauncherMcpPrompts;
pub use mcp::resources::AppLauncherMcpResources;
pub use mcp::tools::AppLauncherMcpTools;
pub use messages::command::DesktopFileCommandAction;
pub use messages::command::DesktopFileCommandMessage;
pub use messages::command::DesktopFileCommandMessageStabby;
pub use messages::status::DesktopFileStatus;
pub use messages::status::DesktopFileStatusMessage;
pub use messages::status::DesktopFileStatusMessageStabby;
pub use messages::status::TOPIC_STATUS;
pub use messages::wrapper::color_mask::ColorMaskConfigFile;
pub use messages::wrapper::color_mask::ColorMaskConfigFileStabby;
pub use messages::wrapper::layer::LayerConfigFile;
pub use messages::wrapper::layer::LayerConfigFileStabby;
pub use messages::wrapper::layer::StabbyLayer;
pub use messages::wrapper::rotation::RotationConfigFile;
pub use messages::wrapper::rotation::RotationConfigFileStabby;
pub use messages::wrapper::window::WindowConfigFile;
pub use messages::wrapper::window::WindowConfigFileStabby;
pub use messages::wrapper::wrapper::SmearorWindowRotationWrapper;
pub use messages::wrapper::wrapper::SmearorWindowRotationWrapperStabby;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DesktopFileCommandMessageConverter, DesktopFileCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    DesktopFileCommandMessageStabbyConverter,
    DesktopFileCommandMessageStabby,
    |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default()
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DesktopFileStatusMessageConverter, DesktopFileStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    DesktopFileStatusMessageStabbyConverter,
    DesktopFileStatusMessageStabby,
    |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default()
);

/// Register all JSON converter implementations for app-launcher messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    DesktopFileCommandMessageConverter::register_in_host(context);
    DesktopFileStatusMessageConverter::register_in_host(context);
}
