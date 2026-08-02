use crate::atomic::AudioAtomicView;
use crate::atomic::AudioAtomicWidget;
use crate::atomic::render_atomic_view;
use smearor_audio_model::AudioStatusMessage;
use smearor_render_utils::Color;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::debug;

const BG_COLOR_ERROR: Color = [40, 20, 20, 255];
const TEXT_COLOR_ERROR: Color = [200, 100, 100, 255];

impl GraphicRenderer for AudioAtomicWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        debug!("AudioAtomicWidget ({:?}): render_graphic {}x{}", self.view, width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let status = self.latest_status.borrow();

        let (icon_char, main_text, info_text, is_error) = render_atomic_graphic(&status, self.view);

        let bg = if is_error { BG_COLOR_ERROR } else { background_color(false) };
        fill_background(&mut pixels, width, height, bg);

        let text_col = if is_error { TEXT_COLOR_ERROR } else { text_color(false) };

        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, text_col);

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
                text_col,
            );
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

fn render_atomic_graphic(status: &Option<AudioStatusMessage>, view: AudioAtomicView) -> (char, String, String, bool) {
    let Some(status) = status else {
        return ('\u{f028}', "--".to_string(), "Loading...".to_string(), true);
    };

    let (icon_str, main, info) = render_atomic_view(status, view);
    let icon_char = icon_str.chars().next().unwrap_or('\u{f028}');
    (icon_char, main, info, false)
}
