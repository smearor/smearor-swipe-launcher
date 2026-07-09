use crate::WallpaperType;

/// Lightweight theme info for the FFI status message.
/// Contains only the fields needed by the widget for display.
#[derive(Clone, Debug, Default)]
#[stabby::stabby]
pub struct WallpaperThemeInfo {
    /// Human-readable name of the theme.
    pub name: stabby::string::String,
    /// Description of the theme.
    pub description: stabby::string::String,
    /// Path to a preview image file for the theme.
    pub preview_image_path: stabby::string::String,
    /// Nerd Font icon name shown as fallback when no preview image is available.
    pub preview_icon: stabby::string::String,
    /// The wallpaper engine type (Video, Image, Application).
    pub wallpaper_type: WallpaperType,
}

impl WallpaperThemeInfo {
    /// Creates a new theme info from name, description, preview path, preview icon and type.
    pub fn new(name: &str, description: &str, preview_image_path: &str, preview_icon: &str, wallpaper_type: WallpaperType) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            preview_image_path: preview_image_path.into(),
            preview_icon: preview_icon.into(),
            wallpaper_type,
        }
    }
}
