/// Return value of `render_atomic_graphic_data` for atomic widgets.
///
/// Encapsulates all data needed by the centralised rendering pipeline to
/// draw an atomic widget button: icon codepoint, text lines, error flag,
/// and optional semantic icon color as RGBA.
#[derive(Clone, Debug)]
pub struct AtomicGraphicData {
    /// Nerd Font codepoint to render as the icon.
    pub icon_char: char,
    /// Primary text line (e.g. temperature, volume).
    pub main_text: String,
    /// Secondary text line (e.g. description, status).
    pub info_text: String,
    /// Whether this data represents an error or loading state.
    pub is_error: bool,
    /// Optional RGBA color for the icon, overriding the default text color.
    pub icon_color: Option<[u8; 4]>,
    /// Optional RGBA color for the main text line.
    pub main_text_color: Option<[u8; 4]>,
    /// Optional RGBA color for the info text line.
    pub info_text_color: Option<[u8; 4]>,
}

impl AtomicGraphicData {
    /// Creates a new `AtomicGraphicData` with no icon color.
    pub fn new(icon_char: char, main_text: String, info_text: String) -> Self {
        Self {
            icon_char,
            main_text,
            info_text,
            is_error: false,
            icon_color: None,
            main_text_color: None,
            info_text_color: None,
        }
    }

    /// Creates a new `AtomicGraphicData` with an explicit icon color.
    pub fn with_color(icon_char: char, main_text: String, info_text: String, icon_color: [u8; 4]) -> Self {
        Self {
            icon_char,
            main_text,
            info_text,
            is_error: false,
            icon_color: Some(icon_color),
            main_text_color: None,
            info_text_color: None,
        }
    }

    /// Creates an error `AtomicGraphicData` with the given icon and message.
    pub fn error(icon_char: char, info_text: String) -> Self {
        Self {
            icon_char,
            main_text: "--".to_string(),
            info_text,
            is_error: true,
            icon_color: None,
            main_text_color: None,
            info_text_color: None,
        }
    }
}
