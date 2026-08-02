use crate::labels::VoiceAssistantLabel;
use crate::views::StatusSnapshot;
use crate::widget::VoiceAssistantView;
use crate::widget::VoiceAssistantWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use smearor_voice_assistant_model::AssistantState;
use tracing::trace;

impl GraphicRenderer for VoiceAssistantWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("VoiceAssistantWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_background(&mut pixels, width, height, background_color(false));

        let text_col = text_color(false);
        let locale = self.personalization.borrow().effective_locale();
        let view = *self.current_view.borrow();

        let status = self.last_status.borrow();
        let snapshot = StatusSnapshot::from_status(status.as_ref());

        let icon_char = view.icon_char();
        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, text_col);

        match view {
            VoiceAssistantView::Idle => {
                if let Some(answer) = &snapshot.final_answer {
                    let truncated: String = answer.chars().take(20).collect();
                    draw_text_centered(
                        &mut pixels,
                        width,
                        height,
                        &truncated,
                        height as f32 * 0.72,
                        (height as f32 * 0.22).min(16.0).max(10.0),
                        text_col,
                    );
                } else {
                    let label = VoiceAssistantLabel::Idle.localized_label(locale);
                    draw_text_centered(&mut pixels, width, height, label, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), text_col);
                }
            }
            VoiceAssistantView::Listening => {
                let label = VoiceAssistantLabel::Listening.localized_label(locale);
                draw_text_centered(&mut pixels, width, height, label, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), text_col);
                if !snapshot.transcript.is_empty() {
                    let truncated: String = snapshot.transcript.chars().take(20).collect();
                    draw_text_centered(
                        &mut pixels,
                        width,
                        height,
                        &truncated,
                        height as f32 * 0.92,
                        (height as f32 * 0.16).min(12.0).max(8.0),
                        text_col,
                    );
                }
            }
            VoiceAssistantView::Speaking => {
                let label = if snapshot.state == AssistantState::Error {
                    VoiceAssistantLabel::Error.localized_label(locale)
                } else {
                    VoiceAssistantLabel::Speaking.localized_label(locale)
                };
                draw_text_centered(&mut pixels, width, height, label, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), text_col);
                let secondary = if snapshot.state == AssistantState::Error {
                    snapshot.error_message.as_ref().map(|e| e.chars().take(20).collect::<String>())
                } else {
                    snapshot.final_answer.as_ref().map(|a| a.chars().take(20).collect::<String>())
                };
                if let Some(text) = secondary {
                    draw_text_centered(&mut pixels, width, height, &text, height as f32 * 0.92, (height as f32 * 0.16).min(12.0).max(8.0), text_col);
                }
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}
