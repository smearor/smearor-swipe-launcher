use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::MonitorProcess;
use crate::TOPIC_STATUS;
use crate::WallpaperThemeInfo;

/// Status message broadcast by the wallpaper service to widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WallpaperStatusMessage {
    /// Name of the currently running theme (`None` if stopped).
    pub current_theme: stabby::option::Option<stabby::string::String>,
    /// PIDs of active wallpaper processes per monitor (empty if none).
    pub current_processes: stabby::vec::Vec<MonitorProcess>,
    /// Name of the theme currently staged/focused in the UI.
    pub selected_theme: stabby::option::Option<stabby::string::String>,
    /// List of all configured themes (lightweight info for display).
    pub themes: stabby::vec::Vec<WallpaperThemeInfo>,
    /// Index of the selected theme in the `themes` list.
    pub selected_theme_index: usize,
}

impl WallpaperStatusMessage {
    /// Creates a new status message with the given themes and selected theme index.
    pub fn new(themes: Vec<WallpaperThemeInfo>, selected_theme_index: usize) -> Self {
        let selected_theme: Option<stabby::string::String> = themes.get(selected_theme_index).map(|t| t.name.clone());
        let themes_stabby: stabby::vec::Vec<WallpaperThemeInfo> = {
            let mut v = stabby::vec::Vec::with_capacity(themes.len());
            for theme in themes {
                v.push(theme);
            }
            v
        };
        Self {
            current_theme: stabby::option::Option::from(None),
            current_processes: stabby::vec::Vec::new(),
            selected_theme: stabby::option::Option::from(selected_theme),
            themes: themes_stabby,
            selected_theme_index,
        }
    }

    /// Returns whether any wallpaper process is currently running.
    pub fn is_running(&self) -> bool {
        self.current_theme.is_some() && !self.current_processes.is_empty()
    }
}

impl TypedMessage for WallpaperStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_wallpaper_model::WallpaperStatusMessage");
}

impl MessageTopic for WallpaperStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for WallpaperStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
