use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Arguments for the `add_wallpaper_theme` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AddWallpaperThemeArgs {
    /// Theme name
    pub name: String,
    /// Theme type: Video, Image, or Application
    #[serde(rename = "type")]
    pub theme_type: String,
    /// Theme-specific configuration object
    pub config: Value,
    /// Optional theme description
    pub description: Option<String>,
    /// Optional path to a preview image
    pub preview_image_path: Option<String>,
    /// Optional preview icon name
    pub preview_icon: Option<String>,
}

/// Arguments for the `remove_wallpaper_theme` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct RemoveWallpaperThemeArgs {
    /// Name of the theme to remove
    pub name: String,
}

/// Arguments for the `select_wallpaper_theme` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SelectWallpaperThemeArgs {
    /// Name of the theme to select
    pub name: String,
}
