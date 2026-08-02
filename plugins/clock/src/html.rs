use crate::widget::ClockWidget;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for ClockWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let time_string = html_escape(&self.clock.get_time_string());
        let date_string = html_escape(&self.clock.get_date_string());
        let weekday_name = html_escape(self.clock.get_weekday_name());

        let date_color_style = if let Some(color) = self.config.text_colors.main_text_color() {
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
        let weekday_color_style = if let Some(color) = self.config.text_colors.info_text_color() {
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

        format!(
            r#"<div class="smearor-widget smearor-clock" data-plugin-id="{}" data-instance-id="{}"><div class="smearor-clock-time">{}</div><div class="smearor-clock-date"{}>{}</div><div class="smearor-clock-weekday"{}>{}</div></div>"#,
            html_escape(plugin_id),
            html_escape(instance_id),
            time_string,
            date_color_style,
            date_string,
            weekday_color_style,
            weekday_name
        )
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
