use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_COMMAND;

/// Actions that the theme service can perform.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ThemeCommandAction {
    /// Select a theme by name. Does not apply CSS — use `ApplySelected` to apply.
    #[default]
    SelectTheme,
    /// Apply the currently selected theme (load CSS, optionally start wallpaper).
    ApplySelected,
    /// Select a theme by name and apply it immediately.
    SelectAndApply,
    /// Refresh status and re-broadcast.
    Refresh,
    /// Add a new theme to the configuration.
    AddTheme,
    /// Remove a theme from the configuration by name.
    RemoveTheme,
}

/// Command message sent to the theme service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThemeCommandMessage {
    /// The action to perform.
    pub action: ThemeCommandAction,
    /// Theme name for name-based actions (`SelectTheme`, `SelectAndApply`, `RemoveTheme`).
    pub name: stabby::option::Option<stabby::string::String>,
    /// Full theme definition as JSON for `AddTheme`.
    /// The service deserializes this into a `Theme` struct.
    pub theme_json: stabby::option::Option<stabby::string::String>,
}

impl ThemeCommandMessage {
    /// Creates a `SelectTheme` command message.
    pub fn select_theme(name: &str) -> Self {
        Self {
            action: ThemeCommandAction::SelectTheme,
            name: Some(name.into()).into(),
            theme_json: None.into(),
        }
    }

    /// Creates an `ApplySelected` command message.
    pub fn apply_selected() -> Self {
        Self {
            action: ThemeCommandAction::ApplySelected,
            name: None.into(),
            theme_json: None.into(),
        }
    }

    /// Creates a `SelectAndApply` command message.
    pub fn select_and_apply(name: &str) -> Self {
        Self {
            action: ThemeCommandAction::SelectAndApply,
            name: Some(name.into()).into(),
            theme_json: None.into(),
        }
    }

    /// Creates a `Refresh` command message.
    pub fn refresh() -> Self {
        Self {
            action: ThemeCommandAction::Refresh,
            name: None.into(),
            theme_json: None.into(),
        }
    }

    /// Creates a `RemoveTheme` command message.
    pub fn remove_theme(name: &str) -> Self {
        Self {
            action: ThemeCommandAction::RemoveTheme,
            name: Some(name.into()).into(),
            theme_json: None.into(),
        }
    }
}

impl TypedMessage for ThemeCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_theme_model::ThemeCommandMessage");
}

impl MessageTopic for ThemeCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for ThemeCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
