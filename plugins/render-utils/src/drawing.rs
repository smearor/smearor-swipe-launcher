use crate::colors::Color;
use crate::fonts::label_font;
use crate::fonts::nerd_font;
use ab_glyph::Font;
use ab_glyph::FontVec;
use ab_glyph::Glyph;
use ab_glyph::PxScale;
use ab_glyph::PxScaleFont;
use ab_glyph::ScaleFont;
use tracing::debug;

/// Fill the entire pixel buffer with a solid background color.
pub fn fill_background(pixels: &mut [u8], width: u32, height: u32, color: Color) {
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

/// Draw a Nerd Font icon centered on the image, occupying the upper portion.
///
/// The icon name is resolved to a Nerd Font codepoint via the caller-provided
/// resolver function. If the font or icon is unavailable, a circular
/// placeholder is drawn instead.
pub fn draw_nerd_font_icon(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    icon_name: &str,
    is_active: bool,
    resolve_codepoint: impl Fn(&str) -> Option<char>,
    icon_color: Option<Color>,
) {
    let font = match nerd_font() {
        Some(f) => f,
        None => {
            debug!("render-utils: Nerd Font not available, drawing placeholder");
            draw_icon_placeholder(pixels, width, height, is_active);
            return;
        }
    };

    let codepoint = match resolve_codepoint(icon_name) {
        Some(c) => c,
        None => {
            debug!("render-utils: unknown icon name '{}', drawing placeholder", icon_name);
            draw_icon_placeholder(pixels, width, height, is_active);
            return;
        }
    };

    let color = icon_color.unwrap_or_else(|| crate::colors::text_color(is_active));

    let icon_size = (width.min(height) as f32 * 0.6).min(48.0);
    let scale = PxScale::from(icon_size);
    let scaled_font = font.as_scaled(scale);

    let glyph_id = font.glyph_id(codepoint);
    let glyph: Glyph = glyph_id.with_scale_and_position(
        scale,
        ab_glyph::point((width as f32 - scaled_font.h_advance(glyph_id)) / 2.0, (height as f32 * 0.4) + scaled_font.ascent() / 2.0),
    );

    draw_glyph(pixels, width, height, font, &glyph, color);
}

/// Draw a simple circular placeholder when the font or icon is unavailable.
pub fn draw_icon_placeholder(pixels: &mut [u8], width: u32, height: u32, is_active: bool) {
    let center_x = width / 2;
    let center_y = width / 2;
    let radius = (width.min(height) / 3).max(8);
    let color = crate::colors::text_color(is_active);

    let cx = center_x as i32;
    let cy = (center_y as i32) - (height as i32 / 6);
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r * r {
                let idx = ((y as u32 * width + x as u32) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

/// Draw a text label at the bottom of the image using the label font.
///
/// Falls back to the Nerd Font if the label font is unavailable, and to
/// a bitmap renderer if no font is available.
pub fn draw_label_text(pixels: &mut [u8], width: u32, height: u32, text: &str, is_active: bool, text_color_override: Option<Color>) {
    let font = match label_font() {
        Some(f) => f,
        None => {
            debug!("render-utils: label font not available, trying Nerd Font");
            match nerd_font() {
                Some(f) => f,
                None => {
                    debug!("render-utils: no font available for label, using bitmap fallback");
                    draw_text_bitmap(pixels, width, height, text, is_active);
                    return;
                }
            }
        }
    };

    let color = text_color_override.unwrap_or_else(|| crate::colors::text_color(is_active));

    let font_size = (height as f32 * 0.22).min(16.0).max(8.0);
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);

    let max_width = width as f32 * 0.9;
    let truncated = truncate_text_to_width(text, &scaled_font, max_width);
    if truncated.is_empty() {
        return;
    }

    let total_width: f32 = truncated.chars().map(|c| scaled_font.h_advance(font.glyph_id(c))).sum();
    let start_x = ((width as f32 - total_width) / 2.0).max(0.0);
    let baseline = height as f32 * 0.88;

    let mut pen_x = start_x;
    for ch in truncated.chars() {
        let glyph_id = font.glyph_id(ch);
        let glyph: Glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(pen_x, baseline));
        draw_glyph(pixels, width, height, font, &glyph, color);
        pen_x += scaled_font.h_advance(glyph_id);
    }
}

/// Draw a Nerd Font icon by its Unicode codepoint at a custom position and size.
///
/// Unlike `draw_nerd_font_icon`, this function takes a codepoint directly
/// and allows custom positioning and scaling.
pub fn draw_nerd_font_codepoint(pixels: &mut [u8], width: u32, height: u32, codepoint: char, center_x: f32, center_y: f32, icon_size: f32, color: Color) {
    let font = match nerd_font() {
        Some(f) => f,
        None => return,
    };

    let scale = PxScale::from(icon_size);
    let scaled_font = font.as_scaled(scale);
    let glyph_id = font.glyph_id(codepoint);
    let glyph: Glyph = glyph_id.with_scale_and_position(
        scale,
        ab_glyph::point(center_x - scaled_font.h_advance(glyph_id) / 2.0, center_y + scaled_font.ascent() / 2.0),
    );
    draw_glyph(pixels, width, height, font, &glyph, color);
}

/// Draw text centered horizontally at a given y-baseline with a configurable font size.
///
/// Uses the label font (JetBrains Mono) and falls back to the Nerd Font.
pub fn draw_text_centered(pixels: &mut [u8], width: u32, height: u32, text: &str, baseline_y: f32, font_size: f32, color: Color) {
    let font = match label_font().or_else(nerd_font) {
        Some(f) => f,
        None => {
            draw_text_bitmap(pixels, width, height, text, false);
            return;
        }
    };

    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);
    let max_width = width as f32 * 0.9;
    let truncated = truncate_text_to_width(text, &scaled_font, max_width);
    if truncated.is_empty() {
        return;
    }

    let total_width: f32 = truncated.chars().map(|c| scaled_font.h_advance(font.glyph_id(c))).sum();
    let start_x = ((width as f32 - total_width) / 2.0).max(0.0);

    let mut pen_x = start_x;
    for ch in truncated.chars() {
        let glyph_id = font.glyph_id(ch);
        let glyph: Glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(pen_x, baseline_y));
        draw_glyph(pixels, width, height, font, &glyph, color);
        pen_x += scaled_font.h_advance(glyph_id);
    }
}

/// Draw a horizontal progress bar at the bottom of the image.
///
/// `value` is a fraction in the range 0.0..=1.0.
pub fn draw_progress_bar(pixels: &mut [u8], width: u32, height: u32, value: f32, color: Color) {
    let bar_height = 4u32;
    let bar_y = height.saturating_sub(bar_height + 2);
    let fill_width = ((width as f32 * value.clamp(0.0, 1.0)) as u32).min(width);

    for y in bar_y..bar_y + bar_height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if x < fill_width {
                pixels[idx..idx + 4].copy_from_slice(&color);
            } else {
                pixels[idx..idx + 4].copy_from_slice(&[color[0] / 3, color[1] / 3, color[2] / 3, 255]);
            }
        }
    }
}

/// Draw a 2x2 or 3x3 icon grid within the image area.
///
/// Each cell contains a Nerd Font icon. The grid columns are determined by
/// `grid_cols` (typically 2 or 3). Icons are resolved via the caller-provided
/// resolver function.
pub fn draw_icon_grid(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    icons: &[&str],
    grid_cols: u32,
    is_active: bool,
    resolve_codepoint: impl Fn(&str) -> Option<char>,
) {
    let font = match nerd_font() {
        Some(f) => f,
        None => return,
    };

    let rows = ((icons.len() as u32 + grid_cols - 1) / grid_cols).max(1);
    let cell_w = width / grid_cols;
    let cell_h = height / rows;
    let icon_size = (cell_w.min(cell_h) as f32 * 0.6).min(24.0);
    let scale = PxScale::from(icon_size);
    let scaled_font = font.as_scaled(scale);
    let color = crate::colors::text_color(is_active);

    for (i, icon_name) in icons.iter().enumerate() {
        let col = i as u32 % grid_cols;
        let row = i as u32 / grid_cols;
        let codepoint = match resolve_codepoint(icon_name) {
            Some(c) => c,
            None => continue,
        };
        let glyph_id = font.glyph_id(codepoint);
        let center_x = col * cell_w + cell_w / 2;
        let center_y = row * cell_h + cell_h / 2;
        let glyph: Glyph = glyph_id.with_scale_and_position(
            scale,
            ab_glyph::point(center_x as f32 - scaled_font.h_advance(glyph_id) / 2.0, center_y as f32 + scaled_font.ascent() / 2.0),
        );
        draw_glyph(pixels, width, height, font, &glyph, color);
    }
}

/// Truncate text to fit within a maximum pixel width.
pub fn truncate_text_to_width(text: &str, scaled_font: &PxScaleFont<&FontVec>, max_width: f32) -> String {
    let mut result = String::new();
    let mut current_width: f32 = 0.0;
    for ch in text.chars() {
        let char_width = scaled_font.h_advance(scaled_font.glyph_id(ch));
        if current_width + char_width > max_width {
            break;
        }
        result.push(ch);
        current_width += char_width;
    }
    result
}

/// Rasterize a single glyph and blend it into the pixel buffer.
pub fn draw_glyph(pixels: &mut [u8], width: u32, height: u32, font: &FontVec, glyph: &Glyph, color: Color) {
    if let Some(outlined) = font.outline_glyph(glyph.clone()) {
        let bounds = outlined.px_bounds();
        let px_start_x = bounds.min.x.max(0.0) as u32;
        let px_start_y = bounds.min.y.max(0.0) as u32;

        outlined.draw(|gx, gy, coverage| {
            let x = px_start_x + gx;
            let y = px_start_y + gy;
            if x < width && y < height {
                let idx = ((y * width + x) * 4) as usize;
                let alpha = (coverage * color[3] as f32) as u8;
                let bg_r = pixels[idx];
                let bg_g = pixels[idx + 1];
                let bg_b = pixels[idx + 2];
                let bg_a = pixels[idx + 3] as f32 / 255.0;
                let src_a = alpha as f32 / 255.0;
                let out_a = src_a + bg_a * (1.0 - src_a);
                if out_a > 0.0 {
                    pixels[idx] = ((color[0] as f32 * src_a + bg_r as f32 * bg_a * (1.0 - src_a)) / out_a) as u8;
                    pixels[idx + 1] = ((color[1] as f32 * src_a + bg_g as f32 * bg_a * (1.0 - src_a)) / out_a) as u8;
                    pixels[idx + 2] = ((color[2] as f32 * src_a + bg_b as f32 * bg_a * (1.0 - src_a)) / out_a) as u8;
                    pixels[idx + 3] = (out_a * 255.0) as u8;
                }
            }
        });
    }
}

/// Fallback bitmap text rendering when no font is available.
pub fn draw_text_bitmap(pixels: &mut [u8], width: u32, height: u32, text: &str, is_active: bool) {
    let color = crate::colors::text_color(is_active);
    let char_width = 6u32;
    let char_height = 8u32;
    let max_chars = (width / char_width).max(1) as usize;
    let truncated: String = text.chars().take(max_chars).collect();
    let text_width = truncated.len() as u32 * char_width;
    let start_x = (width.saturating_sub(text_width)) / 2;
    let start_y = height.saturating_sub(char_height + 4);

    for (char_idx, ch) in truncated.chars().enumerate() {
        let px = start_x + char_idx as u32 * char_width;
        let glyph = simple_glyph(ch);
        for gy in 0..char_height {
            for gx in 0..char_width {
                let bit = (glyph >> ((char_height - 1 - gy) * char_width + (char_width - 1 - gx))) & 1;
                if bit == 1 {
                    let x = px + gx;
                    let y = start_y + gy;
                    if x < width && y < height {
                        let idx = ((y * width + x) * 4) as usize;
                        pixels[idx..idx + 4].copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

/// Load an image file (PNG, JPEG, or SVG) and draw it centered onto the pixel buffer.
///
/// The image is resized to fit within `(max_width, max_height)` while preserving
/// aspect ratio. If the image has an alpha channel, it is alpha-composited onto
/// the existing background.
///
/// Returns `true` if the image was successfully loaded and drawn, `false` otherwise.
pub fn draw_image_centered(pixels: &mut [u8], width: u32, height: u32, image_path: &str, max_width: u32, max_height: u32) -> bool {
    let path = std::path::Path::new(image_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let rgba = if ext == "svg" {
        match load_svg(image_path, max_width, max_height) {
            Some(img) => img,
            None => return false,
        }
    } else {
        match load_raster(image_path) {
            Some(img) => img,
            None => return false,
        }
    };

    let img_w = rgba.width();
    let img_h = rgba.height();
    let raw = rgba.as_raw();

    let scale = (max_width as f32 / img_w as f32).min(max_height as f32 / img_h as f32).min(1.0);
    let target_w = (img_w as f32 * scale).round() as u32;
    let target_h = (img_h as f32 * scale).round() as u32;

    let offset_x = ((width - target_w) / 2) as i32;
    let offset_y = ((height - target_h) / 2) as i32;

    for y in 0..target_h {
        for x in 0..target_w {
            let src_x = (x as f32 / scale) as u32;
            let src_y = (y as f32 / scale) as u32;
            if src_x >= img_w || src_y >= img_h {
                continue;
            }
            let px = offset_x + x as i32;
            let py = offset_y + y as i32;
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                continue;
            }
            let src_idx = ((src_y * img_w + src_x) * 4) as usize;
            let dst_idx = ((py as u32 * width + px as u32) * 4) as usize;
            let r = raw[src_idx];
            let g = raw[src_idx + 1];
            let b = raw[src_idx + 2];
            let a = raw[src_idx + 3];
            if a == 0 {
                continue;
            }
            if a == 255 {
                pixels[dst_idx] = r;
                pixels[dst_idx + 1] = g;
                pixels[dst_idx + 2] = b;
                pixels[dst_idx + 3] = a;
            } else {
                let alpha = a as f32 / 255.0;
                pixels[dst_idx] = (r as f32 * alpha + pixels[dst_idx] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 1] = (g as f32 * alpha + pixels[dst_idx + 1] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 2] = (b as f32 * alpha + pixels[dst_idx + 2] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 3] = 255;
            }
        }
    }

    true
}

/// Convert a `file://` URI to a filesystem path, decoding percent-encoded
/// characters. Returns the input unchanged if it is already a plain path.
pub fn resolve_file_uri(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix("file://") {
        percent_decode_path(path)
    } else {
        uri.to_string()
    }
}

/// Decode percent-encoded characters in a file path (e.g. `%20` → space).
fn percent_decode_path(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                result.push((h * 16 + l) as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Draw an image scaled to cover the entire button area (cover-fit).
///
/// The image is scaled so it completely fills the `width` x `height` area,
/// cropping overflow. Returns `true` if the image was successfully loaded and drawn.
pub fn draw_image_cover(pixels: &mut [u8], width: u32, height: u32, image_path: &str) -> bool {
    let path = std::path::Path::new(image_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let rgba = if ext == "svg" {
        match load_svg(image_path, width, height) {
            Some(img) => img,
            None => return false,
        }
    } else {
        match load_raster(image_path) {
            Some(img) => img,
            None => return false,
        }
    };

    let img_w = rgba.width();
    let img_h = rgba.height();
    let raw = rgba.as_raw();

    let scale_x = width as f32 / img_w as f32;
    let scale_y = height as f32 / img_h as f32;
    let scale = scale_x.max(scale_y);

    let scaled_w = (img_w as f32 * scale).round() as u32;
    let scaled_h = (img_h as f32 * scale).round() as u32;

    let offset_x = ((scaled_w - width) / 2) as i32;
    let offset_y = ((scaled_h - height) / 2) as i32;

    for y in 0..height {
        for x in 0..width {
            let src_x = ((x as i32 + offset_x) as f32 / scale) as u32;
            let src_y = ((y as i32 + offset_y) as f32 / scale) as u32;
            if src_x >= img_w || src_y >= img_h {
                continue;
            }
            let src_idx = ((src_y * img_w + src_x) * 4) as usize;
            let dst_idx = ((y * width + x) * 4) as usize;
            let r = raw[src_idx];
            let g = raw[src_idx + 1];
            let b = raw[src_idx + 2];
            let a = raw[src_idx + 3];
            if a == 0 {
                continue;
            }
            if a == 255 {
                pixels[dst_idx] = r;
                pixels[dst_idx + 1] = g;
                pixels[dst_idx + 2] = b;
                pixels[dst_idx + 3] = a;
            } else {
                let alpha = a as f32 / 255.0;
                pixels[dst_idx] = (r as f32 * alpha + pixels[dst_idx] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 1] = (g as f32 * alpha + pixels[dst_idx + 1] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 2] = (b as f32 * alpha + pixels[dst_idx + 2] as f32 * (1.0 - alpha)) as u8;
                pixels[dst_idx + 3] = 255;
            }
        }
    }

    true
}

/// Load a raster image (PNG, JPEG) from disk.
pub fn load_raster(image_path: &str) -> Option<image::RgbaImage> {
    use image::ImageReader;

    let reader = ImageReader::open(image_path).ok()?;
    let reader = reader.with_guessed_format().ok()?;
    let img = reader.decode().ok()?;
    Some(img.to_rgba8())
}

/// Load and rasterize an SVG file at the given target size.
pub fn load_svg(image_path: &str, target_width: u32, target_height: u32) -> Option<image::RgbaImage> {
    let svg_data = std::fs::read(image_path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(target_width, target_height)?;
    let transform = resvg::tiny_skia::Transform::from_scale(target_width as f32 / tree.size().width(), target_height as f32 / tree.size().height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let data = pixmap.take();
    image::RgbaImage::from_raw(target_width, target_height, data)
}

/// Simple 8x6 bitmap glyph for fallback text rendering.
fn simple_glyph(ch: char) -> u64 {
    let bitmap: u64 = match ch {
        '0' => 0x3c42_4242_4242_3c00,
        '1' => 0x1818_1818_1818_1800,
        '2' => 0x3c42_020c_3040_7e00,
        '3' => 0x7e02_0c02_4242_3c00,
        '4' => 0x0c14_2444_7e04_0400,
        '5' => 0x7e40_7c02_4242_3c00,
        '6' => 0x3c40_7c42_4242_3c00,
        '7' => 0x7e02_0408_1010_1000,
        '8' => 0x3c42_423c_4242_3c00,
        '9' => 0x3c42_4242_3e02_3c00,
        'A'..='Z' | 'a'..='z' => 0x1800_3c3c_7e7e_1800,
        ' ' => 0x0000_0000_0000_0000,
        '-' => 0x0000_0000_7e00_0000,
        ':' => 0x1800_0000_1800_0000,
        '.' => 0x0000_0000_0300_0000,
        '%' => 0x0000_0000_0000_0000,
        _ => 0x7e7e_7e7e_7e7e_7e00,
    };
    bitmap
}
