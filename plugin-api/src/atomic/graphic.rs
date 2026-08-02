//! Optional rendering hooks and centralised rendering pipeline for Atomic Widgets.
//!
//! Widgets that need custom graphics (album art backgrounds, analog clocks,
//! app icons) implement [`AtomicGraphicRenderer`]. The centralised function
//! [`render_atomic_graphic_default`] dispatches based on [`AtomicRenderMode`]
//! and delegates to the widget's hooks when appropriate.

use smearor_render_utils::Color;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;

use crate::atomic::config::AtomicWidgetConfig;
use crate::atomic::render_mode::AtomicRenderMode;

/// Background colour for error states.
const BG_COLOR_ERROR: Color = [40, 20, 20, 255];

/// Text colour for error states.
const TEXT_COLOR_ERROR: Color = [200, 100, 100, 255];

/// Optional rendering hooks for Atomic Widget custom graphics.
///
/// Widgets implement this trait to provide custom background, icon, or
/// full-button graphics. All methods have default no-op implementations
/// that return `false`, signalling the centralised renderer to use its
/// fallback behaviour.
pub trait AtomicGraphicRenderer {
    /// Render a full-button graphic. Return `true` if rendered.
    ///
    /// Called only in [`AtomicRenderMode::GraphicOnly`] mode. If `false` is
    /// returned, the centralised renderer falls back to [`AtomicRenderMode::Icon`].
    fn render_graphic(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }

    /// Render a custom background filling the entire button.
    ///
    /// Return `true` if a background was drawn, `false` to use the solid colour
    /// fallback. Called in [`AtomicRenderMode::BackgroundOnly`] and
    /// [`AtomicRenderMode::Background`] modes.
    fn render_background(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }

    /// Render a custom icon graphic in the icon area.
    ///
    /// The icon area is centered at `(width / 2, height * 0.35)` with size
    /// `(min(width, height) * 0.5).min(40.0)`. Called in
    /// [`AtomicRenderMode::GraphicIcon`] mode. If `false` is returned, the
    /// Nerd Font codepoint fallback is used.
    fn render_icon_graphic(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }
}

/// Centralised rendering pipeline for Atomic Widget headless graphics.
///
/// Dispatches based on the render mode configured in `config`. When
/// `renderer` is `Some`, the widget's [`AtomicGraphicRenderer`] hooks are
/// called for custom graphics. When `None`, fallback behaviour is used
/// (equivalent to a widget without custom rendering).
///
/// This function replaces the per-crate `atomic_graphic.rs` implementations.
pub fn render_atomic_graphic_default(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    icon_char: char,
    main_text: &str,
    info_text: &str,
    is_error: bool,
    config: &AtomicWidgetConfig,
    renderer: Option<&dyn AtomicGraphicRenderer>,
    icon_color: Option<[u8; 4]>,
    main_text_color: Option<[u8; 4]>,
    info_text_color: Option<[u8; 4]>,
) {
    let mode = config.render_mode.as_ref().unwrap_or(&AtomicRenderMode::Icon);
    let is_graphic_only = *mode == AtomicRenderMode::GraphicOnly;
    let show_main = !is_graphic_only && config.show_main_text.unwrap_or(true);
    let show_info = !is_graphic_only && config.show_info_text.unwrap_or(true);

    let bg = if is_error { BG_COLOR_ERROR } else { background_color(false) };
    let text_col = if is_error { TEXT_COLOR_ERROR } else { text_color(false) };
    let icon_col = icon_color.unwrap_or(text_col);
    let main_col = main_text_color.unwrap_or(text_col);
    let info_col = info_text_color.unwrap_or(text_col);

    // 1. GraphicOnly — widget takes over entirely
    let mut fallback_to_icon = false;
    if *mode == AtomicRenderMode::GraphicOnly {
        if let Some(r) = renderer {
            if r.render_graphic(pixels, width, height) {
                return;
            }
        }
        fallback_to_icon = true;
    }

    // 2. Background
    match mode {
        AtomicRenderMode::BackgroundOnly | AtomicRenderMode::Background => {
            let has_custom_bg = if let Some(r) = renderer {
                r.render_background(pixels, width, height)
            } else {
                false
            };
            if !has_custom_bg {
                fill_background(pixels, width, height, bg);
            }
        }
        _ => fill_background(pixels, width, height, bg),
    }

    // 3. Icon
    let icon_size = config.icon_size.map(|s| s as f32).unwrap_or_else(|| (width.min(height) as f32 * 0.5).min(40.0));
    if fallback_to_icon {
        draw_nerd_font_codepoint(pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);
    } else {
        match mode {
            AtomicRenderMode::Icon | AtomicRenderMode::Background => {
                draw_nerd_font_codepoint(pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);
            }
            AtomicRenderMode::GraphicIcon => {
                let has_custom_icon = if let Some(r) = renderer {
                    r.render_icon_graphic(pixels, width, height)
                } else {
                    false
                };
                if !has_custom_icon {
                    draw_nerd_font_codepoint(pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);
                }
            }
            _ => {} // BackgroundOnly, GraphicOnly: no icon
        }
    }

    // 4. Text (with semi-transparent backdrop for readability over custom backgrounds)
    let needs_backdrop = matches!(mode, AtomicRenderMode::BackgroundOnly | AtomicRenderMode::Background);
    let backdrop_opacity = config.text_backdrop_opacity.unwrap_or(0.5);

    if show_main && !main_text.is_empty() {
        if needs_backdrop {
            draw_text_backdrop(
                pixels,
                width,
                height,
                main_text,
                height as f32 * 0.72,
                (height as f32 * 0.22).min(16.0).max(10.0),
                backdrop_opacity,
            );
        }
        draw_text_centered(pixels, width, height, main_text, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), main_col);
    }

    if show_info && !info_text.is_empty() {
        if needs_backdrop {
            draw_text_backdrop(
                pixels,
                width,
                height,
                info_text,
                height as f32 * 0.92,
                (height as f32 * 0.16).min(12.0).max(8.0),
                backdrop_opacity,
            );
        }
        draw_text_centered(pixels, width, height, info_text, height as f32 * 0.92, (height as f32 * 0.16).min(12.0).max(8.0), info_col);
    }
}

/// Draw a semi-transparent dark backdrop behind a text line for readability
/// over custom backgrounds.
///
/// The backdrop covers a horizontal band centered on the text baseline,
/// spanning the full button width. `opacity` controls the blend strength
/// (0.0 = invisible, 1.0 = fully opaque black).
fn draw_text_backdrop(pixels: &mut [u8], width: u32, height: u32, _text: &str, baseline_y: f32, font_size: f32, opacity: f32) {
    let band_height = (font_size * 1.4) as u32;
    let band_y_start = (baseline_y - font_size).max(0.0) as u32;
    let band_y_end = (band_y_start + band_height).min(height);

    let alpha = (opacity * 255.0).clamp(0.0, 255.0) as u8;

    for y in band_y_start..band_y_end {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let bg_r = pixels[idx] as f32;
            let bg_g = pixels[idx + 1] as f32;
            let bg_b = pixels[idx + 2] as f32;
            let src_a = alpha as f32 / 255.0;
            pixels[idx] = (bg_r * (1.0 - src_a)) as u8;
            pixels[idx + 1] = (bg_g * (1.0 - src_a)) as u8;
            pixels[idx + 2] = (bg_b * (1.0 - src_a)) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_render_mode_default() {
        let mode = AtomicRenderMode::default();
        assert_eq!(mode, AtomicRenderMode::Icon);
    }

    #[test]
    fn test_atomic_render_mode_serde_snake_case() {
        let json = "\"background_only\"";
        let mode: AtomicRenderMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, AtomicRenderMode::BackgroundOnly);

        let json = "\"graphic_only\"";
        let mode: AtomicRenderMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, AtomicRenderMode::GraphicOnly);

        let json = "\"graphic_icon\"";
        let mode: AtomicRenderMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, AtomicRenderMode::GraphicIcon);

        let json = "\"background\"";
        let mode: AtomicRenderMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, AtomicRenderMode::Background);

        let json = "\"icon\"";
        let mode: AtomicRenderMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, AtomicRenderMode::Icon);
    }

    #[test]
    fn test_atomic_widget_config_default_values() {
        let config = AtomicWidgetConfig::default();
        assert_eq!(config.render_mode, None);
        assert_eq!(config.show_main_text, None);
        assert_eq!(config.show_info_text, None);
        assert_eq!(config.text_backdrop_opacity, None);
    }

    #[test]
    fn test_atomic_widget_config_parse_render_mode() {
        let json = r#"{"render_mode": "background_only", "show_info_text": false, "text_backdrop_opacity": 0.7}"#;
        let config: AtomicWidgetConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.render_mode, Some(AtomicRenderMode::BackgroundOnly));
        assert_eq!(config.show_info_text, Some(false));
        assert_eq!(config.text_backdrop_opacity, Some(0.7));
    }

    #[test]
    fn test_atomic_widget_config_default_render_mode_is_icon() {
        let json = r#"{}"#;
        let config: AtomicWidgetConfig = serde_json::from_str(json).unwrap();
        let mode = config.render_mode.unwrap_or_default();
        assert_eq!(mode, AtomicRenderMode::Icon);
    }

    #[test]
    fn test_render_atomic_graphic_default_icon_mode() {
        let config = AtomicWidgetConfig::default();
        let mut pixels = vec![0u8; 72 * 72 * 4];
        render_atomic_graphic_default(&mut pixels, 72, 72, '\u{f028}', "60%", "Vol", false, &config, None, None, None, None);
        // Check that the background was filled (not all zeros)
        assert!(pixels[0] != 0 || pixels[1] != 0 || pixels[2] != 0);
    }

    #[test]
    fn test_render_atomic_graphic_default_graphic_only_fallback() {
        let config = AtomicWidgetConfig {
            render_mode: Some(AtomicRenderMode::GraphicOnly),
            show_main_text: Some(false),
            show_info_text: Some(false),
            ..Default::default()
        };
        let mut pixels = vec![0u8; 72 * 72 * 4];
        // No renderer → should fall back to Icon mode (fill background)
        render_atomic_graphic_default(&mut pixels, 72, 72, '\u{f028}', "60%", "Vol", false, &config, None, None, None, None);
        // Background should be filled
        assert!(pixels[0] != 0 || pixels[1] != 0 || pixels[2] != 0);
    }
}
