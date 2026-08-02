use crate::widget::AppLauncherWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_image_centered;
use smearor_render_utils::draw_label_text;
use smearor_render_utils::draw_nerd_font_icon;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use std::path::Path;
use tracing::debug;

impl GraphicRenderer for AppLauncherWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        debug!("AppLauncherWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];

        let is_active = false;
        let bg = background_color(is_active);
        fill_background(&mut pixels, width, height, bg);

        let icon_color = self.config.icon_config.icon_color().map(|c| c.to_rgba());

        let icon_drawn = if self.icon_name.starts_with("nf-") {
            draw_nerd_font_icon(&mut pixels, width, height, &self.icon_name, is_active, resolve_icon_codepoint, icon_color);
            true
        } else {
            let icon_size = (width.min(height) as f32 * 0.6) as u32;
            if let Some(icon_path) = resolve_desktop_icon_path(&self.icon_name, icon_size) {
                debug!("AppLauncherWidget: resolved icon '{}' to '{}'", self.icon_name, icon_path);
                draw_image_centered(&mut pixels, width, height, &icon_path, icon_size, icon_size)
            } else {
                debug!("AppLauncherWidget: no image found for icon '{}', using placeholder", self.icon_name);
                false
            }
        };

        if !icon_drawn {
            draw_nerd_font_icon(&mut pixels, width, height, "nf-md-apps", is_active, resolve_icon_codepoint, None);
        }

        if !self.config.icon_config.icon_only() {
            let label_text = &self.app_name;
            if !label_text.is_empty() {
                draw_label_text(
                    &mut pixels,
                    width,
                    height,
                    label_text,
                    is_active,
                    self.config.text_colors.main_text_color().map(|c| c.to_rgba()),
                );
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

/// Resolve a freedesktop icon name to a file path on disk.
///
/// Searches standard icon theme directories for PNG or SVG files matching the
/// given icon name, picking the size closest to `target_size`.
fn resolve_desktop_icon_path(icon_name: &str, target_size: u32) -> Option<String> {
    let icon_dirs = get_icon_directories();
    let mut best_match: Option<(u32, String)> = None;

    for dir in &icon_dirs {
        let base = Path::new(dir);
        if !base.is_dir() {
            continue;
        }
        if let Ok(size_entries) = std::fs::read_dir(base) {
            for size_entry in size_entries.flatten() {
                let apps_dir = size_entry.path().join("apps");
                if !apps_dir.is_dir() {
                    continue;
                }
                if let Ok(entries) = std::fs::read_dir(&apps_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
                        if stem != icon_name {
                            continue;
                        }
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext != "png" && ext != "svg" {
                            continue;
                        }
                        let size = if ext == "svg" { target_size } else { parse_size_from_path(&path) };
                        let score = if size >= target_size { size - target_size } else { target_size - size };
                        if best_match.as_ref().is_none_or(|(s, _)| score < *s) {
                            best_match = Some((score, path.to_string_lossy().into_owned()));
                        }
                    }
                }
            }
        }
    }

    if let Some((_, path)) = &best_match {
        return Some(path.clone());
    }

    for ext in &["png", "svg"] {
        let pixmap_path = format!("/usr/share/pixmaps/{}.{}", icon_name, ext);
        if Path::new(&pixmap_path).exists() {
            return Some(pixmap_path);
        }
    }

    None
}

/// Get standard freedesktop icon directories from XDG_DATA_DIRS and fallbacks.
fn get_icon_directories() -> Vec<String> {
    let mut dirs = Vec::new();

    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for data_dir in xdg_data_dirs.split(':') {
            if !data_dir.is_empty() {
                dirs.push(format!("{}/icons/hicolor", data_dir));
            }
        }
    }

    dirs.push("/usr/share/icons/hicolor".to_string());
    dirs.push("/usr/local/share/icons/hicolor".to_string());

    dirs
}

/// Parse the size from an icon path like `/usr/share/icons/hicolor/48x48/apps/firefox.png`.
fn parse_size_from_path(path: &Path) -> u32 {
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            if let Some(size_str) = name.split('x').next() {
                if let Ok(size) = size_str.parse::<u32>() {
                    return size;
                }
            }
        }
    }
    48
}
