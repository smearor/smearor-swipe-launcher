mod area_type;
mod config;
mod messages;
mod transition;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use smearor_swipe_launcher_plugin_api::JsonConvertible;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

pub use area_type::AreaType;
pub use area_type::AreaTypeStabby;
pub use config::AreaAlign;
pub use config::AreaConfig;
pub use config::AreaConfigStabby;
pub use config::DEFAULT_AREA_WIDTH;
pub use messages::add::AddAreaMessage;
pub use messages::add::AddAreaMessageStabby;
pub use messages::close::CloseAreaMessage;
pub use messages::open::OpenAreaMessage;
pub use messages::remove::RemoveAreaMessage;
pub use transition::AreaTransition;
pub use transition::AreaTransitionStabby;

impl_json_convertible!(OpenAreaMessageConverter, OpenAreaMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(CloseAreaMessageConverter, CloseAreaMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(RemoveAreaMessageConverter, RemoveAreaMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(AddAreaMessageStabbyConverter, AddAreaMessageStabby, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());

/// Register all JSON converter implementations for area messages via the Host FFI callback.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    OpenAreaMessageConverter::register_in_host(context);
    CloseAreaMessageConverter::register_in_host(context);
    RemoveAreaMessageConverter::register_in_host(context);
    AddAreaMessageStabbyConverter::register_in_host(context);
}

/// Register all JSON converter implementations for area messages directly in a registry.
///
/// Call this once during host application startup (e.g. inside `AreaManager::new`).
pub fn register_json_converters_in_registry(registry: &JsonConverterRegistry) {
    OpenAreaMessageConverter::register_json_converter(registry);
    CloseAreaMessageConverter::register_json_converter(registry);
    RemoveAreaMessageConverter::register_json_converter(registry);
    AddAreaMessageStabbyConverter::register_json_converter(registry);
}
