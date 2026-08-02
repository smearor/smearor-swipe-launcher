//! Shared configuration and helpers for atomic widgets.
//!
//! All atomic widgets (weather, audio, mpris) use the same configuration
//! structure for MacroPad action routing and MCP tool registration, and share
//! common boilerplate for action dispatch and GTK widget construction.

mod action;
mod build;
mod config;
mod graphic;
mod graphic_data;
mod r#macro;
mod render_mode;

pub use action::AtomicAction;
pub use action::SpanActionHandler;
pub use action::UnknownAtomicActionError;
pub use build::AtomicWidgetBuildParams;
pub use build::build_atomic_widget;
pub use build::update_labels;
pub use config::AtomicWidgetConfig;
pub use graphic::AtomicGraphicRenderer;
pub use graphic::render_atomic_graphic_default;
pub use graphic_data::AtomicGraphicData;
pub use render_mode::AtomicRenderMode;
