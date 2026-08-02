//! Renderer trait for non-GTK display surfaces.

use crate::graphic::graphic::FfiGraphic;

/// Trait for widgets that can render to a graphic (non-GTK).
///
/// Used by headless instances (e.g. MacroPad devices) that need pixel buffers
/// instead of GTK widgets. See `concepts/STREAMDECK_CONCEPT.md`.
pub trait GraphicRenderer {
    /// Render the widget to a graphic with the given dimensions.
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic;
}
