use crate::labels::NotificationLabel;
use crate::widget::NotificationView;
use crate::widget::NotificationWidget;
use smearor_notifications_model::NotificationInfo;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for NotificationWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.last_status.borrow();
        let view = *self.current_view.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.effective_locale();

        let (unread_count, do_not_disturb, notifications) = match &*status {
            Some(s) => (s.unread_count, s.do_not_disturb, &s.notifications),
            None => (0, false, &stabby::vec::Vec::<NotificationInfo>::new()),
        };

        let icon_name = if do_not_disturb { "nf-md-bell_off" } else { "nf-fa-bell" };

        let mut html = html_expanded_open(plugin_id, "notifications");
        html.push_str(&format!(
            r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
            instance_id
        ));

        match view {
            NotificationView::Compact => {
                let icon_class = format!("nerd-icon nerd-{}", icon_name);
                html.push_str(&format!(r#"<div class="smearor-notifications-icon"><span class="{}"></span></div>"#, icon_class));
                let count_badge = if unread_count > 0 {
                    format!(r#"<span class="smearor-notifications-badge">{}</span>"#, unread_count)
                } else {
                    String::new()
                };
                html.push_str(&format!(
                    r#"<div class="smearor-notifications-main">{}{}</div>"#,
                    NotificationLabel::Notifications.localized_label(locale),
                    count_badge
                ));
            }
            NotificationView::Expanded => {
                let icon_class = format!("nerd-icon nerd-{}", icon_name);
                html.push_str(&format!(r#"<div class="smearor-notifications-icon"><span class="{}"></span></div>"#, icon_class));
                html.push_str(&format!(
                    r#"<div class="smearor-notifications-header">{}</div>"#,
                    NotificationLabel::Notifications.localized_label(locale)
                ));
                if do_not_disturb {
                    html.push_str(&format!(
                        r#"<div class="smearor-notifications-dnd">{}</div>"#,
                        NotificationLabel::DoNotDisturb.localized_label(locale)
                    ));
                } else if notifications.is_empty() {
                    html.push_str(&format!(
                        r#"<div class="smearor-notifications-empty">{}</div>"#,
                        NotificationLabel::NoNotifications.localized_label(locale)
                    ));
                } else {
                    html.push_str(r#"<div class="smearor-notifications-list">"#);
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    for notification in notifications.iter().take(5) {
                        let time_str = override_data.format_timestamp(notification.timestamp);
                        let relative = override_data.format_relative_time(notification.timestamp, now);
                        html.push_str(&format!(r#"<div class="smearor-notification-item" data-notification-id="{}">"#, notification.id));
                        html.push_str(&format!(r#"<div class="smearor-notification-app">{}</div>"#, notification.app_name));
                        if !notification.summary.is_empty() {
                            html.push_str(&format!(r#"<div class="smearor-notification-summary">{}</div>"#, notification.summary));
                        }
                        if !notification.body.is_empty() {
                            html.push_str(&format!(r#"<div class="smearor-notification-body">{}</div>"#, notification.body));
                        }
                        html.push_str(&format!(r#"<div class="smearor-notification-time" title="{}">{}</div>"#, time_str, relative));
                        html.push_str("</div>");
                    }
                    html.push_str("</div>");
                }
            }
        }

        html.push_str(html_expanded_close());
        html
    }
}
