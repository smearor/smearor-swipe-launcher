//! FFI-safe graphic frame and renderer trait for non-GTK display surfaces.

mod factory;
mod graphic;
mod r#macro;
mod renderer;

pub use graphic::FfiGraphic;
pub use renderer::GraphicRenderer;
