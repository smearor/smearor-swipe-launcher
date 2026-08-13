use crate::labels::ThemeLabels;
use crate::widget::ThemeWidget;
use crate::widget::render_view;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for ThemeWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("ThemeWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let override_data = self.personalization.borrow().clone();
        let is_dark = matches!(override_data.effective_color_scheme(), smearor_personalization_model::ColorScheme::Dark);
        let bg = background_color(is_dark);
        fill_background(&mut pixels, width, height, bg);

        let text_col = self
            .config
            .text_colors
            .main_text_color()
            .map(|c| c.to_rgba())
            .unwrap_or_else(|| text_color(is_dark));
        let icon_col = self.config.icon.icon_color().map(|c| c.to_rgba()).unwrap_or(text_col);

        let status = self.latest_status.borrow();
        let personalization = self.latest_personalization.borrow();
        let labels = ThemeLabels::from_personalization(personalization.as_ref());
        let view_data = render_view(status.as_ref(), &self.config, &labels);

        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f1c5}');
        let icon_size = (width.min(height) as f32 * 0.5).min(36.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

        if !self.config.icon.icon_only() {
            draw_text_centered(
                &mut pixels,
                width,
                height,
                &view_data.main_text,
                height as f32 * 0.72,
                (height as f32 * 0.2).min(14.0).max(8.0),
                text_col,
            );
            draw_text_centered(
                &mut pixels,
                width,
                height,
                &view_data.info_text,
                height as f32 * 0.88,
                (height as f32 * 0.16).min(12.0).max(7.0),
                text_col,
            );
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}
