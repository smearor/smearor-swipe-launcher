//! Render mode enum for Atomic Widget headless graphic output.

use serde::Deserialize;

/// Render mode for an Atomic Widget's headless graphic output.
///
/// Controls how the centralised rendering pipeline draws the widget's button
/// on non-GTK display surfaces (e.g. MacroPad devices).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AtomicRenderMode {
    /// Nerd Font icon + text on solid background (default).
    #[default]
    Icon,
    /// Custom background + text, no icon.
    BackgroundOnly,
    /// Custom background + Nerd Font icon + text.
    Background,
    /// Full custom graphic, no icon, no text.
    GraphicOnly,
    /// Custom icon graphic + text on solid background.
    GraphicIcon,
}
