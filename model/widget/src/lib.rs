//! Shared message types for widget update notifications.
//!
//! This crate defines the `WidgetUpdateMessage` used by widgets to notify
//! the host that their visual state has changed and they need re-rendering.
//!
//! # Topics
//!
//! - `widget.update` — Widget visual state changed, re-render needed

mod topics;
mod update_message;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use smearor_swipe_launcher_plugin_api::AtomicAction;
pub use smearor_swipe_launcher_plugin_api::AtomicWidgetConfig;
pub use smearor_swipe_launcher_plugin_api::ResolvedAction;
pub use topics::TOPIC_WIDGET_UPDATE;
pub use update_message::WidgetUpdateMessage;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WidgetUpdateMessageConverter, WidgetUpdateMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for widget messages.
///
/// Call this once during startup.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    WidgetUpdateMessageConverter::register_in_host(context);
}
