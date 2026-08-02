use crate::widget::AppLauncherWidget;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for AppLauncherWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let icon_html = {
            let icon_class = if self.icon_name.starts_with("nf-") {
                format!("nerd-icon nerd-{}", self.icon_name)
            } else {
                format!("icon icon-{}", self.icon_name)
            };
            let color_style = if let Some(color) = self.config.icon_config.icon_color() {
                format!(
                    " color: rgba({}, {}, {}, {});",
                    (color.r * 255.0).round() as u8,
                    (color.g * 255.0).round() as u8,
                    (color.b * 255.0).round() as u8,
                    color.a
                )
            } else {
                String::new()
            };
            format!(
                r#"<span class="app-launcher-icon {}" style="font-size: {}px;{}"></span>"#,
                icon_class,
                self.config.icon_config.icon_size(),
                color_style
            )
        };

        let label_html = if !self.config.icon_config.icon_only() {
            let main_color_style = if let Some(color) = self.config.text_colors.main_text_color() {
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
            format!(r#"<span class="widget-main-text"{}>{}</span>"#, main_color_style, html_escape(&self.app_name))
        } else {
            String::new()
        };

        let click_action_attr = if self.config.actions.click.topic.is_some() {
            r#" data-click-action="click""#
        } else {
            ""
        };

        let longpress_action_attr = if self.config.actions.longpress.topic.is_some() {
            r#" data-longpress-action="longpress""#
        } else {
            ""
        };

        format!(
            r#"<button class="web-app-launcher" data-plugin-id="{}" data-instance-id="{}"{}{}><div class="app-launcher-inner" style="gap: {}px;">{}{}</div></button>"#,
            html_escape(plugin_id),
            html_escape(instance_id),
            click_action_attr,
            longpress_action_attr,
            self.config.layout.spacing_or_default(),
            icon_html,
            label_html
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
