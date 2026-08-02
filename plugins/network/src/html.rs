use crate::NetworkWidget;
use crate::widget::render_view;
use smearor_network_model::NetworkView;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for NetworkWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let scan = self.latest_scan.borrow();
        let vpn = self.latest_vpn.borrow();
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);
        let override_data = self.personalization.borrow().clone();

        let view_data = match status.as_ref() {
            None => ViewData::new("nf-md-network".to_string(), "Loading...".to_string(), String::new()),
            Some(s) => render_view(s, scan.as_ref(), vpn.as_ref(), &self.config, view, &override_data),
        }
        .with_text_colors(&self.config.text_colors);

        let mut html = html_expanded_open(plugin_id, "network");
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
        html.push_str(&format!(r#"<div class="smearor-network-icon"><span class="{}"{}</span></div>"#, icon_class, color_style));
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
        html.push_str(&format!(r#"<div class="smearor-network-main"{}>{}</div>"#, main_color_style, view_data.main_text));
        if !view_data.info_text.is_empty() {
            let marquee_class = if view_data.info_text.len() > 20 { " marquee" } else { "" };
            html.push_str(&format!(
                r#"<div class="smearor-network-info{}"{}>{}</div>"#,
                marquee_class, info_color_style, view_data.info_text
            ));
        }
        html.push_str(html_expanded_close());
        html
    }
}
