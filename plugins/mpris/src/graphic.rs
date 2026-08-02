use crate::labels::MprisLabel;
use crate::widget::MprisView;
use crate::widget::MprisWidget;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_progress_bar;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for MprisWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("MprisWidget: render_graphic {}x{}", width, height);

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
        let status = self.last_status.borrow();
        let view = *self.current_view.borrow();
        let personalization = self.personalization.borrow().clone();
        let locale = personalization.effective_locale();

        let (icon_char, main_text, info_text) = match &*status {
            None => ('\u{f001}', MprisLabel::NoPlayer.localized_label(locale).to_string(), String::new()),
            Some(status) if !status.has_player => ('\u{f001}', MprisLabel::NoPlayer.localized_label(locale).to_string(), String::new()),
            Some(status) => {
                let icon = match status.playback_status {
                    MprisPlaybackStatus::Playing => '\u{f04b}',
                    MprisPlaybackStatus::Paused => '\u{f04c}',
                    MprisPlaybackStatus::Stopped => '\u{f04d}',
                };
                let title = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.title.is_empty() { None } else { Some(m.title.as_str()) })
                    .unwrap_or(MprisLabel::UnknownTitle.localized_label(locale));
                let artist = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.artist.is_empty() { None } else { Some(m.artist.as_str()) })
                    .unwrap_or("");
                (icon, title.to_string(), artist.to_string())
            }
        };

        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

        match view {
            MprisView::Compact => {
                if !main_text.is_empty() {
                    draw_text_centered(
                        &mut pixels,
                        width,
                        height,
                        &main_text,
                        height as f32 * 0.72,
                        (height as f32 * 0.22).min(16.0).max(10.0),
                        text_col,
                    );
                }

                if !info_text.is_empty() {
                    draw_text_centered(
                        &mut pixels,
                        width,
                        height,
                        &info_text,
                        height as f32 * 0.92,
                        (height as f32 * 0.16).min(12.0).max(8.0),
                        info_text_col,
                    );
                }
            }
            MprisView::Expanded => {
                if !main_text.is_empty() {
                    draw_text_centered(
                        &mut pixels,
                        width,
                        height,
                        &main_text,
                        height as f32 * 0.62,
                        (height as f32 * 0.18).min(14.0).max(10.0),
                        text_col,
                    );
                }

                if !info_text.is_empty() {
                    draw_text_centered(
                        &mut pixels,
                        width,
                        height,
                        &info_text,
                        height as f32 * 0.78,
                        (height as f32 * 0.16).min(12.0).max(8.0),
                        info_text_col,
                    );
                }

                if let Some(status) = status.as_ref() {
                    if status.has_player {
                        if let Some(meta) = status.metadata.as_ref() {
                            if meta.length > 0 {
                                let ratio = (status.position as f64 / meta.length as f64).clamp(0.0, 1.0) as f32;
                                draw_progress_bar(&mut pixels, width, height, ratio, text_col);

                                let elapsed = personalization.format_duration(status.position);
                                let total = personalization.format_duration(meta.length);
                                let time_text = format!("{} / {}", elapsed, total);
                                draw_text_centered(
                                    &mut pixels,
                                    width,
                                    height,
                                    &time_text,
                                    height as f32 * 0.95,
                                    (height as f32 * 0.14).min(10.0).max(7.0),
                                    text_col,
                                );
                            }
                        }
                    }
                }
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}
