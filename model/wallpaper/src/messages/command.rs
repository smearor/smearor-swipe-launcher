use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_COMMAND;

/// Actions that can be sent from the widget to the wallpaper service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum WallpaperCommandAction {
    /// Select a theme by name without starting it.
    #[default]
    SelectTheme,
    /// Start the currently selected wallpaper theme.
    StartSelected,
    /// Stop the currently running wallpaper theme.
    StopCurrent,
    /// Refresh the status broadcast.
    Refresh,
}

/// Command message sent from the wallpaper widget to the wallpaper service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WallpaperCommandMessage {
    /// The action to execute.
    pub action: WallpaperCommandAction,
    /// The theme name for `SelectTheme` actions; ignored for other actions.
    pub theme_name: stabby::string::String,
}

impl WallpaperCommandMessage {
    /// Creates a new command message with the given action and theme name.
    pub fn new(action: WallpaperCommandAction, theme_name: &str) -> Self {
        Self {
            action,
            theme_name: theme_name.into(),
        }
    }

    /// Creates a `SelectTheme` command message.
    pub fn select_theme(name: &str) -> Self {
        Self::new(WallpaperCommandAction::SelectTheme, name)
    }

    /// Creates a `StartSelected` command message.
    pub fn start_selected() -> Self {
        Self::new(WallpaperCommandAction::StartSelected, "")
    }

    /// Creates a `StopCurrent` command message.
    pub fn stop_current() -> Self {
        Self::new(WallpaperCommandAction::StopCurrent, "")
    }
}

impl TypedMessage for WallpaperCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_wallpaper_model::WallpaperCommandMessage");
}

impl MessageTopic for WallpaperCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for WallpaperCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
