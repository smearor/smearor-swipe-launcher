use crate::labels::VoiceAssistantLabel;
use crate::views::StatusSnapshot;
use crate::widget::VoiceAssistantView;
use crate::widget::VoiceAssistantWidget;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::WebRenderer;
use smearor_voice_assistant_model::AssistantState;

impl WebRenderer for VoiceAssistantWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let status = self.last_status.borrow();
        let view = *self.current_view.borrow();
        let locale = self.personalization.borrow().effective_locale();

        let snapshot = StatusSnapshot::from_status(status.as_ref());

        let icon_name = view.icon_name();

        let mut html = html_expanded_open(plugin_id, "voice-assistant");
        html.push_str(&format!(
            r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
            instance_id
        ));

        let icon_class = format!("nerd-icon nerd-{}", icon_name);
        html.push_str(&format!(r#"<div class="smearor-voice-assistant-icon"><span class="{}"></span></div>"#, icon_class));

        match view {
            VoiceAssistantView::Idle => {
                if let Some(answer) = &snapshot.final_answer {
                    let truncated: String = answer.chars().take(40).collect();
                    html.push_str(&format!(r#"<div class="smearor-voice-assistant-main">{}</div>"#, truncated));
                } else {
                    let label = VoiceAssistantLabel::Idle.localized_label(locale);
                    html.push_str(&format!(r#"<div class="smearor-voice-assistant-main">{}</div>"#, label));
                }
            }
            VoiceAssistantView::Listening => {
                let label = VoiceAssistantLabel::Listening.localized_label(locale);
                html.push_str(&format!(r#"<div class="smearor-voice-assistant-main">{}</div>"#, label));
                if !snapshot.transcript.is_empty() {
                    let truncated: String = snapshot.transcript.chars().take(40).collect();
                    html.push_str(&format!(r#"<div class="smearor-voice-assistant-info marquee">{}</div>"#, truncated));
                }
            }
            VoiceAssistantView::Speaking => {
                let label = if snapshot.state == AssistantState::Error {
                    VoiceAssistantLabel::Error.localized_label(locale)
                } else {
                    VoiceAssistantLabel::Speaking.localized_label(locale)
                };
                html.push_str(&format!(r#"<div class="smearor-voice-assistant-main">{}</div>"#, label));
                let secondary = if snapshot.state == AssistantState::Error {
                    snapshot.error_message.clone()
                } else {
                    snapshot.final_answer.clone()
                };
                if let Some(text) = secondary {
                    let truncated: String = text.chars().take(40).collect();
                    let marquee_class = if text.len() > 40 { " marquee" } else { "" };
                    html.push_str(&format!(r#"<div class="smearor-voice-assistant-info{}">{}</div>"#, marquee_class, truncated));
                }
            }
        }

        html.push_str(html_expanded_close());
        html
    }
}
