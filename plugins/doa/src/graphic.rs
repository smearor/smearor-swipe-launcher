use crate::labels::DoaLabel;
use crate::widget::DoaWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for DoaWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("DoaWidget: render_graphic {}x{}", width, height);

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
        let (icon_name, main_text, info_text) = match &*status {
            Some(status) => DoaWidget::render_view_data(status, &self.config, view, locale),
            None => {
                let label = DoaLabel::Disconnected.localized_label(locale);
                (self.config.icon_disconnected.clone(), label.to_string(), String::new())
            }
        };

        let icon_char = nerd_icon_char(&icon_name);
        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

        draw_text_centered(
            &mut pixels,
            width,
            height,
            &main_text,
            height as f32 * 0.72,
            (height as f32 * 0.22).min(16.0).max(10.0),
            text_col,
        );
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

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

fn nerd_icon_char(icon_name: &str) -> char {
    match icon_name {
        "nf-md-compass" => '\u{f0549}',
        "nf-md-compass_off" => '\u{f0b80}',
        "nf-md-arrow_up" => '\u{f005c}',
        "nf-md-arrow_right" => '\u{f0054}',
        "nf-md-arrow_down" => '\u{f0045}',
        "nf-md-arrow_left" => '\u{f004d}',
        "nf-md-microphone_variant" => '\u{f0377}',
        "nf-md-account_voice" => '\u{f0062}',
        _ => '\u{f0549}',
    }
}
