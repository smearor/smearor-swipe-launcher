use serde::Deserialize;
use serde::Serialize;

/// The type of wallpaper engine to use for a theme.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum WallpaperType {
    /// Video wallpaper using `mpvpaper`.
    #[default]
    Video,
    /// Image slideshow wallpaper using `mpvpaper`.
    Image,
    /// Application-based wallpaper using `smearor-wrot` or a custom command.
    Application,
}

/// Returns a Nerd Font icon string for the given wallpaper type.
pub fn wallpaper_type_icon(wallpaper_type: &WallpaperType) -> &'static str {
    match wallpaper_type {
        WallpaperType::Video => "\u{f03d}",
        WallpaperType::Image => "\u{f03e}",
        WallpaperType::Application => "\u{f2d0}",
    }
}
