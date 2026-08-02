use crate::labels::PowerLabel;
use crate::widget::PowerWidget;
use crate::widget::WidgetView;
use smearor_power_model::PowerAction;
use smearor_power_model::power_action_icon;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for PowerWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let view = *self.widget_view.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        match view {
            WidgetView::Compact => render_html_compact(self, instance_id, plugin_id, &locale),
            WidgetView::Confirm => render_html_confirm(self, instance_id, plugin_id, &locale),
        }
    }
}

fn render_html_compact(widget: &PowerWidget, instance_id: &str, plugin_id: &str, locale: &smearor_swipe_launcher_plugin_api::Locale) -> String {
    let actions = widget.enabled_actions.borrow();
    let view_idx = *widget.current_view.borrow();
    let action = actions.get(view_idx).cloned().unwrap_or(PowerAction::Shutdown);
    let icon_name = power_action_icon(&action);
    let label = PowerLabel::from_action(&action, *locale);

    let mut html = html_expanded_open(plugin_id, "power");
    html.pop();
    html.push_str(&format!(
        r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
        instance_id
    ));
    let icon_class = format!("nerd-icon nerd-{}", icon_name);
    let color_style = if let Some(color) = widget.config.icon_config.icon_color() {
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
    html.push_str(&format!(r#"<div class="smearor-power-icon"><span class="{}"{}</span></div>"#, icon_class, color_style));
    let main_color_style = if let Some(color) = widget.config.text_colors.main_text_color() {
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
    html.push_str(&format!(r#"<div class="smearor-power-label"{}>{}</div>"#, main_color_style, label));
    html.push_str(html_expanded_close());
    html
}

fn render_html_confirm(widget: &PowerWidget, _instance_id: &str, plugin_id: &str, locale: &smearor_swipe_launcher_plugin_api::Locale) -> String {
    let confirm_actions = widget.confirm_actions();

    let mut html = html_expanded_open(plugin_id, "power");
    html.push_str(r#" data-action-source="true" data-view="confirm">"#);
    html.push_str(r#"<div class="smearor-power-grid">"#);

    for action in &confirm_actions {
        let icon_name = power_action_icon(action);
        let label = PowerLabel::from_action(action, *locale);
        let action_str = match action {
            PowerAction::Shutdown => "shutdown",
            PowerAction::Reboot => "reboot",
            PowerAction::Suspend => "suspend",
            PowerAction::Hibernate => "hibernate",
            PowerAction::Lock => "lock",
            PowerAction::Logout => "logout",
            PowerAction::RebootToFirmware => "reboot_to_firmware",
            PowerAction::Cancel => "cancel",
        };
        html.push_str(&format!(
            r#"<button class="smearor-power-action" data-click-topic="service.power.command" data-click-payload='{{"action":"execute","power_action":"{}"}}'><span class="nerd-icon nerd-{}"></span><span class="smearor-power-action-label">{}</span></button>"#,
            action_str, icon_name, label
        ));
    }

    html.push_str("</div>");
    html.push_str(html_expanded_close());
    html
}
