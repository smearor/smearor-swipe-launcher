use crate::labels::ThemeLabels;
use crate::widget::ThemeWidget;
use crate::widget::render_view;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for ThemeWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let personalization = self.latest_personalization.borrow();
        let labels = ThemeLabels::from_personalization(personalization.as_ref());
        let view_data = render_view(status.as_ref(), &self.config, &labels);

        let mut html = html_expanded_open(plugin_id, "theme");
        html.pop();
        html.push_str(&format!(
            r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
            instance_id
        ));

        let color_style = if let Some(color) = self.config.icon.icon_color() {
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

        html.push_str(&format!(
            r#"<div class="smearor-theme-icon smearor-theme-icon--fallback"><span class="nerd-icon nerd-{}"{}</span></div>"#,
            view_data.icon_name, color_style
        ));

        if !self.config.icon.icon_only() {
            html.push_str(&format!(r#"<div class="smearor-widget-text smearor-widget-main-text">{}</div>"#, view_data.main_text));
            html.push_str(&format!(r#"<div class="smearor-widget-text smearor-widget-info-text">{}</div>"#, view_data.info_text));
        }

        html.push_str(&html_expanded_close());
        html
    }
}
