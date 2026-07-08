use serde::Deserialize;
use serde::Serialize;

/// Configuration for an image-based wallpaper slideshow using `mpvpaper`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ImageConfig {
    /// Directory containing image files to display.
    pub directory: String,
    /// Target display outputs. `["ALL"]` targets all connected monitors.
    pub outputs: Vec<String>,
    /// Duration each image is displayed, in milliseconds.
    pub display_duration_ms: u32,
    /// Whether to shuffle the image order.
    pub shuffle: bool,
    /// Whether to enable transitions between images.
    pub transitions: bool,
    /// Transition effect name (e.g., "fade", "slide").
    pub transition_effect: String,
    /// Transition duration in milliseconds.
    pub transition_duration_ms: u32,
    /// Additional mpv arguments passed verbatim.
    pub extra_arguments: Vec<String>,
}
