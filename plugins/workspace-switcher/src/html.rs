use crate::widget::WorkspaceSwitcherWidget;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for WorkspaceSwitcherWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let ws_list = self.workspaces.borrow();

        let (icon_class, label_text, info_text, fraction) = if ws_list.is_empty() {
            ("nf-md-loading".to_string(), "...".to_string(), "0/0".to_string(), 0.0)
        } else {
            let idx = *self.current_view.borrow();
            let idx = idx.min(ws_list.len() - 1);
            let ws = &ws_list[idx];
            let key = ws.workspace_id.to_string();
            let icon = self.config.icon_map.get(&key).cloned().unwrap_or_else(|| self.config.default_icon.clone());
            let label = if self.config.show_label {
                ws.workspace_name.to_string()
            } else {
                String::new()
            };
            let info = format!("{}/{}", idx + 1, ws_list.len());
            let frac = if ws_list.len() > 1 { idx as f32 / (ws_list.len() - 1) as f32 } else { 0.0 };
            (icon, label, info, frac)
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

        let icon_html = if icon_class.starts_with("nf-") {
            format!(
                r#"<span class="workspace-switcher-icon nerd-icon nerd-{}" style="font-size: {}px;{}"></span>"#,
                html_escape(&icon_class),
                self.config.icon_config.icon_size(),
                color_style
            )
        } else {
            format!(
                r#"<span class="workspace-switcher-icon icon-{}" style="font-size: {}px;{}"></span>"#,
                html_escape(&icon_class),
                self.config.icon_config.icon_size(),
                color_style
            )
        };

        let label_html = if !self.config.icon_config.icon_only() && !label_text.is_empty() {
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
            format!(r#"<span class="widget-main-text"{}>{}</span>"#, main_color_style, html_escape(&label_text))
        } else {
            String::new()
        };

        let info_html = if !self.config.icon_config.icon_only() {
            let info_color_style = if let Some(color) = self.config.text_colors.info_text_color() {
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
            format!(r#"<span class="widget-info-text"{}>{}</span>"#, info_color_style, html_escape(&info_text))
        } else {
            String::new()
        };

        let scrollbar_html = if !self.config.icon_config.icon_only() && self.config.show_scrollbar {
            format!(
                r#"<div class="workspace-switcher-scrollbar" style="width: 100%; height: 4px; background: rgba(68,68,68,0.5);"><div style="width: {:.0}%; height: 100%; background: rgba(240,240,240,0.9);"></div></div>"#,
                fraction * 100.0
            )
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
            r#"<button class="web-workspace-switcher" data-plugin-id="{}" data-instance-id="{}"{}{}><div class="workspace-switcher-inner" style="gap: {}px;">{}{}{}{}</div></button>"#,
            html_escape(plugin_id),
            html_escape(instance_id),
            click_action_attr,
            longpress_action_attr,
            self.config.layout.spacing_or_default(),
            icon_html,
            label_html,
            info_html,
            scrollbar_html
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
