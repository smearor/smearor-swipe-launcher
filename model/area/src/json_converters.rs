use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use smearor_swipe_launcher_plugin_api::JsonConvertible;

use crate::AddAreaMessageStabby;
use crate::CloseAreaMessage;
use crate::OpenAreaMessage;
use crate::RemoveAreaMessage;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(OpenAreaMessageConverter, OpenAreaMessage, |json: serde_json::Value| {
    let area_id = json.get("area_id").and_then(|v| v.as_str()).unwrap_or("");
    OpenAreaMessage::new(area_id)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(CloseAreaMessageConverter, CloseAreaMessage, |json: serde_json::Value| {
    let area_id = json.get("area_id").and_then(|v| v.as_str()).unwrap_or("");
    CloseAreaMessage::new(area_id)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(RemoveAreaMessageConverter, RemoveAreaMessage, |json: serde_json::Value| {
    let area_id = json.get("area_id").and_then(|v| v.as_str()).unwrap_or("");
    RemoveAreaMessage::new(area_id)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AddAreaMessageStabbyConverter, AddAreaMessageStabby, |json: serde_json::Value| {
    let area_id = json.get("area_id").and_then(|v| v.as_str()).unwrap_or("");
    AddAreaMessageStabby {
        area_id: area_id.into(),
        area_config: crate::AreaConfigStabby {
            area_type: crate::AreaTypeStabby::default(),
            width: stabby::option::Option::None(),
            width_percent: stabby::option::Option::None(),
            min_width: stabby::option::Option::None(),
            max_width: stabby::option::Option::None(),
            open_transition: crate::AreaTransitionStabby::None,
            close_transition: stabby::option::Option::None(),
            auto_close: false,
            close_on_escape: false,
            plugins: stabby::vec::Vec::new(),
        },
    }
});

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
