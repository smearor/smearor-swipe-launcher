use crate::labels::WallpaperLabel;
use crate::widget::WallpaperWidget;
use crate::widget::WidgetView;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_image_centered;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use smearor_wallpaper_model::WallpaperThemeInfo;
use tracing::trace;

impl GraphicRenderer for WallpaperWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("WallpaperWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let view = *self.widget_view.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        match view {
            WidgetView::Compact => {
                render_compact(&mut pixels, width, height, self, &locale);
            }
            WidgetView::Grid => {
                render_grid(&mut pixels, width, height, self, &locale);
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

fn render_compact(pixels: &mut [u8], width: u32, height: u32, widget: &WallpaperWidget, locale: &smearor_swipe_launcher_plugin_api::Locale) {
    let bg = background_color(false);
    fill_background(pixels, width, height, bg);

    let text_col = widget
        .config
        .text_colors
        .main_text_color()
        .map(|c| c.to_rgba())
        .unwrap_or_else(|| text_color(false));
    let icon_col = widget.config.icon_config.icon_color().map(|c| c.to_rgba()).unwrap_or(text_col);
    let status = widget.latest_status.borrow();
    let theme_info: Option<WallpaperThemeInfo> = status.as_ref().and_then(|s| s.themes.get(s.selected_theme_index).cloned());

    let (preview_path, preview_icon, theme_name) = match &theme_info {
        Some(theme) => (theme.preview_image_path.to_string(), theme.preview_icon.to_string(), theme.name.to_string()),
        None => (String::new(), String::new(), WallpaperLabel::NoTheme.localized_label(*locale)),
    };

    let max_thumb_w = width * 3 / 4;
    let max_thumb_h = height * 3 / 4;
    let image_drawn = if !preview_path.is_empty() {
        draw_image_centered(pixels, width, height, &preview_path, max_thumb_w, max_thumb_h)
    } else {
        false
    };

    if !image_drawn {
        let icon_name = if preview_icon.is_empty() { "nf-md-wallpaper" } else { &preview_icon };
        let icon_char = resolve_icon_codepoint(icon_name).unwrap_or('\u{f1c5}');
        let icon_size = (width.min(height) as f32 * 0.5).min(36.0);
        draw_nerd_font_codepoint(pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.4, icon_size, icon_col);
    }

    if !widget.config.icon_config.icon_only() {
        draw_text_centered(pixels, width, height, &theme_name, height as f32 * 0.85, (height as f32 * 0.2).min(14.0).max(8.0), text_col);
    }
}

fn render_grid(pixels: &mut [u8], width: u32, height: u32, widget: &WallpaperWidget, _locale: &smearor_swipe_launcher_plugin_api::Locale) {
    let bg = background_color(false);
    fill_background(pixels, width, height, bg);

    let text_col = text_color(false);
    let status = widget.latest_status.borrow();
    let themes: Vec<WallpaperThemeInfo> = status.as_ref().map(|s| s.themes.iter().cloned().collect()).unwrap_or_default();
    let selected_index = status.as_ref().map(|s| s.selected_theme_index).unwrap_or(0);

    let grid_cols = 3u32;
    let grid_rows = 3u32;
    let cell_w = width / grid_cols;
    let cell_h = height / grid_rows;

    for i in 0..9 {
        let col = i as u32 % grid_cols;
        let row = i as u32 / grid_cols;
        let cell_x = col * cell_w;
        let cell_y = row * cell_h;

        let theme = themes.get(i);
        let is_selected = i == selected_index;

        if is_selected {
            let highlight: smearor_render_utils::Color = [60, 60, 80, 255];
            fill_cell(pixels, width, cell_x, cell_y, cell_w, cell_h, highlight);
        }

        if let Some(theme) = theme {
            let preview_path = theme.preview_image_path.to_string();
            let max_w = cell_w.saturating_sub(4);
            let max_h = cell_h.saturating_sub(4);
            let drawn = if !preview_path.is_empty() {
                draw_image_centered_offset(pixels, width, cell_x + 2, cell_y + 2, max_w, max_h, &preview_path)
            } else {
                false
            };
            if !drawn {
                let icon_name = if theme.preview_icon.is_empty() {
                    "nf-md-wallpaper"
                } else {
                    theme.preview_icon.as_str()
                };
                let icon_char = resolve_icon_codepoint(icon_name).unwrap_or('\u{f1c5}');
                let icon_size = (cell_w.min(cell_h) as f32 * 0.5).min(20.0);
                draw_nerd_font_codepoint(
                    pixels,
                    width,
                    height,
                    icon_char,
                    (cell_x + cell_w / 2) as f32,
                    (cell_y + cell_h / 2) as f32,
                    icon_size,
                    text_col,
                );
            }
        } else {
            let icon_char = resolve_icon_codepoint("nf-md-plus").unwrap_or('\u{f0fe5}');
            let icon_size = (cell_w.min(cell_h) as f32 * 0.3).min(16.0);
            draw_nerd_font_codepoint(
                pixels,
                width,
                height,
                icon_char,
                (cell_x + cell_w / 2) as f32,
                (cell_y + cell_h / 2) as f32,
                icon_size,
                text_col,
            );
        }
    }
}

fn fill_cell(pixels: &mut [u8], width: u32, cell_x: u32, cell_y: u32, cell_w: u32, cell_h: u32, color: smearor_render_utils::Color) {
    for y in 0..cell_h {
        for x in 0..cell_w {
            let px = cell_x + x;
            let py = cell_y + y;
            if px >= width {
                break;
            }
            let idx = ((py * width + px) * 4) as usize;
            if idx + 4 > pixels.len() {
                break;
            }
            pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn draw_image_centered_offset(pixels: &mut [u8], canvas_width: u32, offset_x: u32, offset_y: u32, max_w: u32, max_h: u32, image_path: &str) -> bool {
    let path = std::path::Path::new(image_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let rgba = if ext == "svg" {
        match smearor_render_utils::load_svg(image_path, max_w, max_h) {
            Some(img) => img,
            None => return false,
        }
    } else {
        match smearor_render_utils::load_raster(image_path) {
            Some(img) => img,
            None => return false,
        }
    };

    let img_w = rgba.width();
    let img_h = rgba.height();
    let raw = rgba.as_raw();

    let scale = (max_w as f32 / img_w as f32).min(max_h as f32 / img_h as f32).min(1.0);
    let target_w = (img_w as f32 * scale).round() as u32;
    let target_h = (img_h as f32 * scale).round() as u32;

    let center_offset_x = ((max_w - target_w) / 2) + offset_x;
    let center_offset_y = ((max_h - target_h) / 2) + offset_y;

    for y in 0..target_h {
        for x in 0..target_w {
            let src_x = (x as f32 / scale) as u32;
            let src_y = (y as f32 / scale) as u32;
            if src_x >= img_w || src_y >= img_h {
                continue;
            }
            let px = center_offset_x + x;
            let py = center_offset_y + y;
            if px >= canvas_width {
                break;
            }
            let dst_idx = ((py * canvas_width + px) * 4) as usize;
            if dst_idx + 4 > pixels.len() {
                break;
            }
            let src_idx = ((src_y * img_w + src_x) * 4) as usize;
            let a = raw[src_idx + 3];
            if a == 0 {
                continue;
            }
            if a == 255 {
                pixels[dst_idx] = raw[src_idx];
                pixels[dst_idx + 1] = raw[src_idx + 1];
                pixels[dst_idx + 2] = raw[src_idx + 2];
                pixels[dst_idx + 3] = a;
            } else {
                let alpha = a as f32 / 255.0;
                pixels[dst_idx] = (raw[src_idx] as f32 * alpha + pixels[dst_idx] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 1] = (raw[src_idx + 1] as f32 * alpha + pixels[dst_idx + 1] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 2] = (raw[src_idx + 2] as f32 * alpha + pixels[dst_idx + 2] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 3] = 255;
            }
        }
    }

    true
}
