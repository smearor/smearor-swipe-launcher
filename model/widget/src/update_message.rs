use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_WIDGET_UPDATE;

/// Notification that a widget's visual state has changed and it needs re-rendering.
///
/// Widgets send this message after updating their internal state or view
/// (e.g. after a view switch, state update from a service, or internal timer).
/// The host listens for this topic and triggers the appropriate re-render
/// based on the instance type:
/// - **Headless**: calls `render_graphic()` and sends `SetButtonImage` to the MacroPad service.
/// - **Web**: calls `render_html()` and pushes the new HTML fragment via WebSocket.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WidgetUpdateMessage {
    /// Plugin ID that needs re-rendering.
    pub plugin_id: stabby::string::String,
    /// Instance ID that owns the plugin.
    pub instance_id: stabby::string::String,
}

impl WidgetUpdateMessage {
    /// Create a new widget update message.
    pub fn new(plugin_id: &str, instance_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            instance_id: instance_id.into(),
        }
    }
}

impl TypedMessage for WidgetUpdateMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_widget::WidgetUpdateMessage");
}

impl MessageTopic for WidgetUpdateMessage {
    fn topic() -> &'static str {
        TOPIC_WIDGET_UPDATE
    }
}

impl SharedMessage for WidgetUpdateMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WIDGET_UPDATE
    }
}
