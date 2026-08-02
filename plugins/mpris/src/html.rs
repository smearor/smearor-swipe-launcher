use crate::labels::MprisLabel;
use crate::widget::MprisView;
use crate::widget::MprisWidget;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WebRenderer;

impl WebRenderer for MprisWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.last_status.borrow();
        let view = *self.current_view.borrow();
        let personalization = self.personalization.borrow().clone();
        let locale = personalization.effective_locale();

        let (icon_name, main_text, info_text) = match &*status {
            None => ("nf-fa-music".to_string(), MprisLabel::NoPlayer.localized_label(locale).to_string(), String::new()),
            Some(status) if !status.has_player => ("nf-fa-music".to_string(), MprisLabel::NoPlayer.localized_label(locale).to_string(), String::new()),
            Some(status) => {
                let icon = match status.playback_status {
                    MprisPlaybackStatus::Playing => "nf-fa-pause",
                    MprisPlaybackStatus::Paused => "nf-fa-play",
                    MprisPlaybackStatus::Stopped => "nf-fa-play",
                };
                let title = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.title.is_empty() { None } else { Some(m.title.as_str()) })
                    .unwrap_or(MprisLabel::UnknownTitle.localized_label(locale));
                let artist = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.artist.is_empty() { None } else { Some(m.artist.as_str()) })
                    .unwrap_or("");
                (icon.to_string(), title.to_string(), artist.to_string())
            }
        };

        let view_data = match view {
            MprisView::Compact => ViewData::new(icon_name, main_text, info_text),
            MprisView::Expanded => {
                let mut info = info_text.clone();
                if let Some(status) = status.as_ref() {
                    if status.has_player {
                        if let Some(meta) = status.metadata.as_ref() {
                            if meta.length > 0 {
                                let elapsed = personalization.format_duration(status.position);
                                let total = personalization.format_duration(meta.length);
                                if info.is_empty() {
                                    info = format!("{} / {}", elapsed, total);
                                } else {
                                    info = format!("{} \u{2022} {} / {}", info, elapsed, total);
                                }
                            }
                        }
                    }
                }
                ViewData::new(icon_name, main_text, info)
            }
        }
        .with_text_colors(&self.config.text_colors);

        let mut html = html_expanded_open(plugin_id, "mpris");
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
        html.push_str(&format!(r#"<div class="smearor-mpris-icon"><span class="{}"{}</span></div>"#, icon_class, color_style));
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
        html.push_str(&format!(r#"<div class="smearor-mpris-main"{}>{}</div>"#, main_color_style, view_data.main_text));
        if !view_data.info_text.is_empty() {
            let marquee_class = if view_data.info_text.len() > 20 { " marquee" } else { "" };
            html.push_str(&format!(
                r#"<div class="smearor-mpris-info{}"{}>{}</div>"#,
                marquee_class, info_color_style, view_data.info_text
            ));
        }
        html.push_str(html_expanded_close());
        html
    }
}
