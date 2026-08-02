use crate::labels::AudioLabel;
use crate::widget::AudioView;
use crate::widget::AudioWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for AudioWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("AudioWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_background(&mut pixels, width, height, background_color(false));

        let text_col = self
            .config
            .text_colors
            .main_text_color()
            .map(|c| c.to_rgba())
            .unwrap_or_else(|| text_color(false));
        let info_text_col = self.config.text_colors.info_text_color().map(|c| c.to_rgba()).unwrap_or(text_col);
        let icon_col = self.config.icon_config.icon_color().map(|c| c.to_rgba()).unwrap_or(text_col);
        let locale = self.personalization.borrow().effective_locale();
        let view = *self.current_view.borrow();

        let status = self.last_status.borrow();
        let (volume, is_muted, device_name) = match &*status {
            Some(status) => (status.volume, status.is_muted, status.active_device.as_ref().map(|d| d.name.as_str()).unwrap_or("Unknown")),
            None => (*self.current_volume.lock().unwrap_or_else(|e| e.into_inner()), false, "Unknown"),
        };
        let icon_char = Self::select_icon_char(volume, is_muted);

        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

        match view {
            AudioView::Compact => {
                let pct = if is_muted {
                    AudioLabel::Muted.localized_label(locale).to_string()
                } else {
                    format!("{:.0}%", volume * 100.0)
                };
                draw_text_centered(&mut pixels, width, height, &pct, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), text_col);
            }
            AudioView::Expanded => {
                let label = AudioLabel::Volume.localized_label(locale);
                draw_text_centered(&mut pixels, width, height, label, height as f32 * 0.52, (height as f32 * 0.16).min(12.0).max(8.0), text_col);
                let pct = if is_muted {
                    AudioLabel::Muted.localized_label(locale).to_string()
                } else {
                    format!("{:.0}%", volume * 100.0)
                };
                draw_text_centered(&mut pixels, width, height, &pct, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), text_col);
                draw_text_centered(
                    &mut pixels,
                    width,
                    height,
                    device_name,
                    height as f32 * 0.92,
                    (height as f32 * 0.16).min(12.0).max(8.0),
                    info_text_col,
                );
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

impl AudioWidget {
    fn select_icon_char(volume: f32, is_muted: bool) -> char {
        if is_muted {
            '\u{f026}'
        } else if volume > 0.66 {
            '\u{f028}'
        } else if volume > 0.33 {
            '\u{f027}'
        } else if volume > 0.0 {
            '\u{f027}'
        } else {
            '\u{f026}'
        }
    }
}
