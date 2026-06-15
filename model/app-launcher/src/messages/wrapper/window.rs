use serde::Deserialize;
use serde::Serialize;

/// Window configuration section
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WindowConfigFile {
    /// Aspect ratio
    #[serde(default)]
    pub aspect_ratio: Option<f32>,

    /// Start in fullscreen mode
    #[serde(default)]
    pub fullscreen: Option<bool>,

    /// Initial height
    #[serde(default)]
    pub height: Option<i32>,

    /// Minimum height
    #[serde(default)]
    pub min_height: Option<i32>,

    /// Minimum width
    #[serde(default)]
    pub min_width: Option<i32>,

    /// Maximum height
    #[serde(default)]
    pub max_height: Option<i32>,

    /// Maximum width
    #[serde(default)]
    pub max_width: Option<i32>,

    /// Start in maximized mode
    #[serde(default)]
    pub maximized: Option<bool>,

    /// Whether the window should be resizable
    #[serde(default)]
    pub resizable: Option<bool>,

    /// Whether the window should have decorations
    #[serde(default)]
    pub show_decorations: Option<bool>,

    /// Title of the application window.
    #[serde(default)]
    pub title: Option<String>,

    /// Initial width
    #[serde(default)]
    pub width: Option<i32>,

    /// Window opacity for the compositor window (0.0 = fully transparent, 1.0 = fully opaque).
    #[serde(default)]
    pub window_opacity: Option<f32>,

    /// Initial x position
    #[serde(default)]
    pub x: Option<i32>,

    /// Initial y position
    #[serde(default)]
    pub y: Option<i32>,
}

impl WindowConfigFile {
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![];
        if !self.show_decorations.unwrap_or_default() {
            args.push("--decorated".to_string());
        }
        // TODO: Changed CLI args
        // if self.show_decorations.unwrap_or_default() {
        //     args.push("--show-decorations".to_string());
        // }
        args
    }
}
