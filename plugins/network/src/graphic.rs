use crate::NetworkWidget;
use crate::widget::render_view;
use smearor_network_model::NetworkView;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use smearor_swipe_launcher_plugin_api::ViewData;
use tracing::trace;

impl GraphicRenderer for NetworkWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("NetworkWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_background(&mut pixels, width, height, background_color(false));

        let configured_icon_color = self.config.icon_config.icon_color().map(|c| c.to_rgba());
        let override_data = self.personalization.borrow().clone();

        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);

        let status = self.latest_status.borrow();
        let scan = self.latest_scan.borrow();
        let vpn = self.latest_vpn.borrow();

        let view_data = match status.as_ref() {
            None => ViewData::new("nf-md-network".to_string(), "Loading...".to_string(), String::new()),
            Some(s) => render_view(s, scan.as_ref(), vpn.as_ref(), &self.config, view, &override_data),
        }
        .with_text_colors(&self.config.text_colors);

        let text_col = view_data.main_text_color.map(|c| c.to_rgba()).unwrap_or_else(|| text_color(false));
        let info_text_col = view_data.info_text_color.map(|c| c.to_rgba()).unwrap_or(text_col);
        let icon_col = view_data.icon_color.map(|c| c.to_rgba()).or(configured_icon_color).unwrap_or(text_col);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f0928}');
        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

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
