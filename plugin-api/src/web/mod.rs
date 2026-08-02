//! FFI-safe HTML string and renderer trait for web-based display surfaces.

mod html;
mod r#macro;
mod renderer;

pub use html::FfiHtmlString;
pub use renderer::WebRenderer;
