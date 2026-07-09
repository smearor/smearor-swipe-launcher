use serde::Deserialize;
use serde::Serialize;

use crate::WallpaperThemeConfig;
use crate::WallpaperType;

/// A wallpaper theme definition with metadata and type-specific configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WallpaperTheme {
    /// Human-readable name of the theme.
    pub name: String,
    /// Description of the theme.
    pub description: String,
    /// Path to a preview image file for the theme.
    pub preview_image_path: String,
    /// Nerd Font icon name shown as fallback when no preview image is available.
    pub preview_icon: String,
    /// The wallpaper engine type (Video, Image, Application).
    pub wallpaper_type: WallpaperType,
    /// Type-specific configuration for this theme.
    pub config: WallpaperThemeConfig,
}
