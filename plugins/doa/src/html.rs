use crate::widget::DoaWidget;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for DoaWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.last_status.borrow();
        let view = *self.current_view.borrow();
        let locale = self.personalization.borrow().effective_locale();

        let (icon_name, main_text, info_text) = match &*status {
            Some(status) => DoaWidget::render_view_data(status, &self.config, view, locale),
            None => {
                let label = crate::labels::DoaLabel::Disconnected.localized_label(locale);
                (self.config.icon_disconnected.clone(), label.to_string(), String::new())
            }
        };

        let view_data = ViewData::new(icon_name, main_text, info_text).with_text_colors(&self.config.text_colors);

        let mut html = html_expanded_open(plugin_id, "doa");
        html.pop();
        html.push_str(&format!(
            r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
            instance_id
        ));
        let icon_class = if view_data.icon_name.starts_with("nf-") {
            format!("nerd-icon nerd-{}", view_data.icon_name)
        } else {
            format!("nerd-icon {}", view_data.icon_name)
        };
        html.push_str(&format!(r#"<div class="widget-content doa-widget"><span class="{}"></span>"#, icon_class));
        if !view_data.main_text.is_empty() {
            html.push_str(&format!(r#"<span class="widget-main-text">{}</span>"#, view_data.main_text));
        }
        if !view_data.info_text.is_empty() {
            html.push_str(&format!(r#"<span class="widget-info-text">{}</span>"#, view_data.info_text));
        }
        html.push_str("</div>");
        html.push_str(&html_expanded_close());
        html
    }
}
