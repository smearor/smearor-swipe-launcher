use serde::Deserialize;
use serde::Serialize;

use crate::AppConfig;
use crate::ImageConfig;
use crate::VideoConfig;

/// Type-specific configuration for a wallpaper theme.
/// The variant must match the `wallpaper_type` field of the parent `WallpaperTheme`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum WallpaperThemeConfig {
    /// Configuration for video wallpapers.
    Video(VideoConfig),
    /// Configuration for image slideshow wallpapers.
    Image(ImageConfig),
    /// Configuration for application-based wallpapers.
    Application(AppConfig),
}

impl Default for WallpaperThemeConfig {
    fn default() -> Self {
        Self::Video(VideoConfig::default())
    }
}
