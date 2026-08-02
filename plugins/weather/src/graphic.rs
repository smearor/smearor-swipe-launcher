use crate::widget::WeatherWidget;
use crate::widget::render_view;
use smearor_render_utils::Color;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_weather_model::WeatherView;
use tracing::trace;

/// Background color for error/stale states (dark red).
const BG_COLOR_ERROR: Color = [40, 20, 20, 255];

/// Text color for error/stale states (muted red).
const TEXT_COLOR_ERROR: Color = [200, 100, 100, 255];

impl GraphicRenderer for WeatherWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("WeatherWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];

        let status = self.latest_status.borrow();
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(WeatherView::Current);
        let override_data = self.personalization.borrow().clone();

        let view_data = match status.as_ref() {
            None => ViewData::error("nf-weather-alien".to_string(), "Loading...".to_string()),
            Some(s) if s.is_stale => {
                let error = s.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Stale data".to_string());
                ViewData::error("nf-weather-alien".to_string(), format!("Stale: {error}"))
            }
            Some(s) if !s.success => {
                let error = s.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "No data".to_string());
                ViewData::error("nf-weather-alien".to_string(), error)
            }
            Some(s) => render_view(s, view, &override_data),
        }
        .with_text_colors(&self.config.text_colors);

        let bg = if view_data.is_error { BG_COLOR_ERROR } else { background_color(false) };
        fill_background(&mut pixels, width, height, bg);

        let text_col = if view_data.is_error {
            TEXT_COLOR_ERROR
        } else {
            view_data.main_text_color.map(|c| c.to_rgba()).unwrap_or_else(|| text_color(false))
        };
        let info_text_col = if view_data.is_error {
            TEXT_COLOR_ERROR
        } else {
            view_data.info_text_color.map(|c| c.to_rgba()).unwrap_or(text_col)
        };
        let configured_icon_color = self.config.icon_config.icon_color().map(|c| c.to_rgba());
        let icon_col = view_data.icon_color.map(|c| c.to_rgba()).or(configured_icon_color).unwrap_or(text_col);

        // Icon size from config, clamped to reasonable bounds for the button dimensions.
        let icon_size = self.config.icon_config.icon_size() as f32;
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f07b}');
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

        // Temperature text in the middle.
        if !view_data.main_text.is_empty() {
            draw_text_centered(
                &mut pixels,
                width,
                height,
                &view_data.main_text,
                height as f32 * 0.72,
                (height as f32 * 0.22).min(16.0).max(10.0),
                text_col,
            );
        }

        // Info text at the bottom.
        if !view_data.info_text.is_empty() {
            draw_text_centered(
                &mut pixels,
                width,
                height,
                &view_data.info_text,
                height as f32 * 0.92,
                (height as f32 * 0.16).min(12.0).max(8.0),
                info_text_col,
            );
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}
