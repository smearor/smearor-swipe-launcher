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
    /// The wallpaper engine type (Video, Image, Application).
    pub wallpaper_type: WallpaperType,
    /// Type-specific configuration for this theme.
    pub config: WallpaperThemeConfig,
}
