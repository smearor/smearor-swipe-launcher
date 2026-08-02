use smearor_wallpaper_model::MonitorProcess;

/// Internal state of the wallpaper service.
#[derive(Clone, Default)]
pub struct WallpaperState {
    /// Name of the currently running theme.
    pub current_theme: Option<String>,
    /// PIDs of active wallpaper processes per monitor.
    pub current_processes: Vec<MonitorProcess>,
    /// Index of the selected theme in the themes list.
    pub selected_theme_index: usize,
}
