use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_STATUS;
use crate::ThemeInfo;
use crate::ThemeMode;

/// Status message broadcast by the theme service.
/// Consumed by the theme switcher widget and other interested services.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThemeStatusMessage {
    /// All configured themes (display info only).
    pub themes: stabby::vec::Vec<ThemeInfo>,
    /// Name of the currently applied theme, if any.
    pub current_theme: stabby::option::Option<stabby::string::String>,
    /// Timestamp of the last status update (ISO 8601).
    pub last_updated: stabby::string::String,
    /// Index of the currently selected theme in the `themes` list.
    pub selected_theme_index: u32,
    /// The effective mode after System resolution (Dark or Light).
    /// For fixed-mode themes, this equals the theme's mode.
    pub effective_mode: ThemeMode,
}

impl TypedMessage for ThemeStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_theme_model::ThemeStatusMessage");
}

impl MessageTopic for ThemeStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for ThemeStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
