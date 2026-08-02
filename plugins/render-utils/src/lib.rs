//! Shared rendering utilities for headless widget rendering.
//!
//! This crate provides common font loading, drawing utilities, and color
//! constants used by widget plugins that implement `GraphicRenderer` for
//! headless (non-GTK) instances such as MacroPad devices.
//!
//! No GTK dependency — pure Rust with `ab_glyph`, `image`, and `imageproc`.

mod fonts;

pub mod colors;
pub mod drawing;
pub mod html;

mod icons;

pub use colors::*;
pub use drawing::*;
pub use fonts::label_font;
pub use fonts::nerd_font;
pub use icons::resolve_icon_codepoint;
