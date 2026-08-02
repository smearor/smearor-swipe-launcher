use crate::widget::WeatherWidget;
use crate::widget::render_view;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WebRenderer;
use smearor_weather_model::WeatherView;

impl WebRenderer for WeatherWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(WeatherView::Current);

        let view_data = match status.as_ref() {
            None => ViewData::error("nf-weather-na".to_string(), "Loading...".to_string()),
            Some(s) if s.is_stale => {
                let error = s.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Stale data".to_string());
                ViewData::error("nf-weather-na".to_string(), format!("Stale: {error}"))
            }
            Some(s) if !s.success => {
                let error = s.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "No data".to_string());
                ViewData::error("nf-weather-na".to_string(), error)
            }
            Some(s) => {
                let override_data = self.personalization.borrow().clone();
                render_view(s, view, &override_data)
            }
        }
        .with_text_colors(&self.config.text_colors);

        let mut html = html_expanded_open(plugin_id, "weather");
        // html_expanded_open ends with '>', replace it with our extra attributes
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
        let color_style = if let Some(color) = view_data.icon_color.or(self.config.icon_config.icon_color()) {
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
        html.push_str(&format!(r#"<div class="smearor-weather-icon"><span class="{}"{}</span></div>"#, icon_class, color_style));
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
        html.push_str(&format!(r#"<div class="smearor-weather-temp"{}>{}</div>"#, main_color_style, view_data.main_text));
        let marquee_class = if view_data.info_text.len() > 20 { " marquee" } else { "" };
        html.push_str(&format!(
            r#"<div class="smearor-weather-info{}"{}>{}</div>"#,
            marquee_class, info_color_style, view_data.info_text
        ));
        html.push_str(html_expanded_close());
        html
    }
}
