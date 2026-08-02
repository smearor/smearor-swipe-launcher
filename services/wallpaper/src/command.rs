use smearor_wallpaper_model::WallpaperTheme;

/// Internal command union for the service event loop.
pub enum WallpaperCommand {
    /// Select a theme without starting it.
    SelectTheme(String),
    /// Start the currently selected theme (stops any running theme first).
    StartSelected,
    /// Stop the currently running wallpaper process.
    StopCurrent,
    /// Refresh the status broadcast.
    Refresh,
    /// Permanently add a new theme to the configuration store.
    AddTheme(WallpaperTheme),
    /// Permanently remove a theme from the configuration store.
    RemoveTheme(String),
}
