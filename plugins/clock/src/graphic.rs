use crate::widget::ClockWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for ClockWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("ClockWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_background(&mut pixels, width, height, background_color(false));

        let text_col = self
            .config
            .text_colors
            .main_text_color()
            .map(|c| c.to_rgba())
            .unwrap_or_else(|| text_color(false));
        let info_text_col = self.config.text_colors.info_text_color().map(|c| c.to_rgba()).unwrap_or(text_col);

        let time_string = self.clock.get_time_string();
        let date_string = self.clock.get_date_string();
        let weekday_name = self.clock.get_weekday_name();

        let time_font_size = (height as f32 * 0.35).min(28.0).max(14.0);
        draw_text_centered(&mut pixels, width, height, &time_string, height as f32 * 0.40, time_font_size, text_col);

        let date_font_size = (height as f32 * 0.20).min(14.0).max(8.0);
        draw_text_centered(&mut pixels, width, height, &date_string, height as f32 * 0.72, date_font_size, text_col);

        let weekday_font_size = (height as f32 * 0.16).min(12.0).max(8.0);
        draw_text_centered(&mut pixels, width, height, weekday_name, height as f32 * 0.92, weekday_font_size, info_text_col);

        FfiGraphic::from_pixels(width, height, pixels)
    }
}
