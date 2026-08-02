use crate::labels::AudioLabel;
use crate::widget::AudioView;
use crate::widget::AudioWidget;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for AudioWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.last_status.borrow();
        let view = *self.current_view.borrow();
        let locale = self.personalization.borrow().effective_locale();

        let (volume, is_muted, device_name) = match &*status {
            Some(status) => (status.volume, status.is_muted, status.active_device.as_ref().map(|d| d.name.as_str()).unwrap_or("Unknown")),
            None => (0.5f32, false, "Unknown"),
        };

        let icon_name = AudioWidget::select_icon_name(volume, is_muted);
        let main_text = if is_muted {
            AudioLabel::Muted.localized_label(locale).to_string()
        } else {
            format!("{:.0}%", volume * 100.0)
        };

        let view_data = match view {
            AudioView::Compact => ViewData::new(icon_name.to_string(), main_text, String::new()),
            AudioView::Expanded => ViewData::new(icon_name.to_string(), main_text, device_name.to_string()),
        }
        .with_text_colors(&self.config.text_colors);

        let mut html = html_expanded_open(plugin_id, "audio");
        html.pop();
        html.push_str(&format!(
            r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
            instance_id
        ));
        let icon_class = if view_data.icon_name.starts_with("nf-") {
            format!("nerd-icon nerd-{}", view_data.icon_name)
        } else {
            format!("icon icon-{}", view_data.icon_name)
        };
        let color_style = if let Some(color) = self.config.icon_config.icon_color() {
            format!(
                r#" style="color: rgba({}, {}, {}, {});""#,
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
                color.a
            )
        } else {
            String::new()
        };
        html.push_str(&format!(r#"<div class="smearor-audio-icon"><span class="{}"{}</span></div>"#, icon_class, color_style));
        let main_color_style = if let Some(color) = view_data.main_text_color {
            format!(
                r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
                color.a
            )
        } else {
            String::new()
        };
        let info_color_style = if let Some(color) = view_data.info_text_color {
            format!(
                r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
                color.a
            )
        } else {
            String::new()
        };
        html.push_str(&format!(r#"<div class="smearor-audio-main"{}>{}</div>"#, main_color_style, view_data.main_text));
        if !view_data.info_text.is_empty() {
            let marquee_class = if view_data.info_text.len() > 20 { " marquee" } else { "" };
            html.push_str(&format!(
                r#"<div class="smearor-audio-info{}"{}>{}</div>"#,
                marquee_class, info_color_style, view_data.info_text
            ));
        }
        html.push_str(html_expanded_close());
        html
    }
}
