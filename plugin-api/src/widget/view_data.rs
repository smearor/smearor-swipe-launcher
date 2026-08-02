use crate::widget::Color;
use crate::widget::WidgetTextColors;
use typed_builder::TypedBuilder;

/// Rendered view data for display across GTK, graphic, atomic, and HTML renderers.
///
/// Encapsulates the icon name, text lines, optional semantic icon color,
/// and an error flag. Used by widget rendering functions that need to
/// communicate what to display without coupling to a specific renderer.
#[derive(Debug, Clone, TypedBuilder)]
pub struct ViewData {
    /// The Nerd Font icon name (e.g. `nf-weather-day_sunny`).
    #[builder(setter(into))]
    pub icon_name: String,
    /// The main text to display (e.g. temperature, volume).
    #[builder(setter(into))]
    pub main_text: String,
    /// The info text to display (e.g. description or label).
    #[builder(setter(into))]
    pub info_text: String,
    /// Optional semantic color for the icon.
    pub icon_color: Option<Color>,
    /// Optional semantic color for the main text line.
    #[builder(default, setter(into, strip_option))]
    pub main_text_color: Option<Color>,
    /// Optional semantic color for the info text line.
    #[builder(default, setter(into, strip_option))]
    pub info_text_color: Option<Color>,
    /// Whether this view represents an error or stale state.
    pub is_error: bool,
}

impl ViewData {
    /// Creates a new `ViewData` with `is_error` set to `false` and no icon color.
    pub fn new(icon_name: String, main_text: String, info_text: String) -> Self {
        Self {
            icon_name,
            main_text,
            info_text,
            icon_color: None,
            main_text_color: None,
            info_text_color: None,
            is_error: false,
        }
    }

    /// Creates a new `ViewData` with the given icon color and `is_error` set to `false`.
    pub fn with_color(icon_name: String, main_text: String, info_text: String, icon_color: Option<Color>) -> Self {
        Self {
            icon_name,
            main_text,
            info_text,
            icon_color,
            main_text_color: None,
            info_text_color: None,
            is_error: false,
        }
    }

    /// Creates an error `ViewData` with the given icon name and info text.
    pub fn error(icon_name: String, info_text: String) -> Self {
        Self {
            icon_name,
            main_text: "--".to_string(),
            info_text,
            icon_color: None,
            main_text_color: None,
            info_text_color: None,
            is_error: true,
        }
    }

    /// Injects configured text colors as fallback when no semantic color is already set.
    pub fn with_text_colors(mut self, text_colors: &WidgetTextColors) -> Self {
        if self.main_text_color.is_none() {
            self.main_text_color = text_colors.main_text_color();
        }
        if self.info_text_color.is_none() {
            self.info_text_color = text_colors.info_text_color();
        }
        self
    }
}
