use crate::labels::VoiceAssistantLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::AtomicWidgetConfig;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use smearor_voice_assistant_model::AssistantState;
use smearor_voice_assistant_model::AssistantStatusMessage;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

/// Which voice assistant view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceAssistantAtomicView {
    /// Microphone icon + "Listen" text. Click to start listening.
    Listen,
    /// Microphone icon + "PTT" text. Hold for push-to-talk.
    PushToTalk,
    /// Stop icon + "Stop" text. Click to stop current response.
    Stop,
    /// Status icon + status text. Click to open overlay.
    Status,
}

impl VoiceAssistantAtomicView {
    /// Returns the default nerd font icon name for this view.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Listen => "nf-md-microphone",
            Self::PushToTalk => "nf-md-microphone",
            Self::Stop => "nf-md-stop",
            Self::Status => "nf-md-information",
        }
    }

    /// Returns the default nerd font icon codepoint for this view.
    pub fn icon_char(&self) -> char {
        match self {
            Self::Listen => '\u{f130}',
            Self::PushToTalk => '\u{f130}',
            Self::Stop => '\u{f04d}',
            Self::Status => '\u{f02c}',
        }
    }
}

impl FromStr for VoiceAssistantAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "voice_assistant_listen" => Ok(Self::Listen),
            "voice_assistant_push_to_talk" => Ok(Self::PushToTalk),
            "voice_assistant_stop" => Ok(Self::Stop),
            "voice_assistant_status" => Ok(Self::Status),
            _ => Err(format!("Unknown voice assistant atomic view: {s}")),
        }
    }
}

impl VoiceAssistantAtomicView {
    /// Renders this view's display data from the current assistant status and personalization override.
    pub fn render(&self, status: &AssistantStatusMessage, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.effective_locale();
        match self {
            Self::Listen => {
                let (icon, main) = match status.current_state {
                    AssistantState::Listening | AssistantState::ProcessingStt | AssistantState::ThinkingLlm | AssistantState::ExecutingAction => {
                        ("nf-md-microphone", VoiceAssistantLabel::Listening.localized_label(locale))
                    }
                    AssistantState::Speaking => ("nf-md-volume_high", VoiceAssistantLabel::Speaking.localized_label(locale)),
                    AssistantState::Error => ("nf-md-alert_circle", VoiceAssistantLabel::Error.localized_label(locale)),
                    _ => ("nf-md-microphone", VoiceAssistantLabel::Listen.localized_label(locale)),
                };
                ViewData::new(icon.to_string(), main.to_string(), String::new())
            }
            Self::PushToTalk => {
                let (icon, main) = match status.current_state {
                    AssistantState::Listening | AssistantState::ProcessingStt | AssistantState::ThinkingLlm | AssistantState::ExecutingAction => {
                        ("nf-md-microphone", VoiceAssistantLabel::Listening.localized_label(locale))
                    }
                    AssistantState::Speaking => ("nf-md-volume_high", VoiceAssistantLabel::Speaking.localized_label(locale)),
                    AssistantState::Error => ("nf-md-alert_circle", VoiceAssistantLabel::Error.localized_label(locale)),
                    _ => ("nf-md-microphone", VoiceAssistantLabel::Ptt.localized_label(locale)),
                };
                ViewData::new(icon.to_string(), main.to_string(), String::new())
            }
            Self::Stop => {
                let (icon, main) = match status.current_state {
                    AssistantState::Idle | AssistantState::Standby => ("nf-md-stop", VoiceAssistantLabel::Stop.localized_label(locale)),
                    _ => ("nf-md-stop_circle", VoiceAssistantLabel::Stop.localized_label(locale)),
                };
                ViewData::new(icon.to_string(), main.to_string(), String::new())
            }
            Self::Status => {
                let (icon, main, info) = match status.current_state {
                    AssistantState::Idle => ("nf-md-microphone_off", VoiceAssistantLabel::Idle.localized_label(locale), String::new()),
                    AssistantState::Standby => ("nf-md-microphone_off", VoiceAssistantLabel::Standby.localized_label(locale), String::new()),
                    AssistantState::Listening => {
                        ("nf-md-microphone", VoiceAssistantLabel::Listening.localized_label(locale), status.partial_transcript.clone())
                    }
                    AssistantState::ProcessingStt => ("nf-md-waveform", VoiceAssistantLabel::Transcribing.localized_label(locale), String::new()),
                    AssistantState::ThinkingLlm => ("nf-md-brain", VoiceAssistantLabel::Thinking.localized_label(locale), String::new()),
                    AssistantState::ExecutingAction => ("nf-md-cog_play", VoiceAssistantLabel::Executing.localized_label(locale), String::new()),
                    AssistantState::Speaking => ("nf-md-volume_high", VoiceAssistantLabel::Speaking.localized_label(locale), String::new()),
                    AssistantState::Error => ("nf-md-alert_circle", VoiceAssistantLabel::Error.localized_label(locale), String::new()),
                };
                let info_truncated: String = info.chars().take(20).collect();
                ViewData::new(icon.to_string(), main.to_string(), info_truncated)
            }
        }
    }

    /// Renders display data when no status is available yet.
    pub fn render_default(&self, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.effective_locale();
        match self {
            Self::Listen => ViewData::new(self.icon_name().to_string(), VoiceAssistantLabel::Listen.localized_label(locale).to_string(), String::new()),
            Self::PushToTalk => ViewData::new(self.icon_name().to_string(), VoiceAssistantLabel::Ptt.localized_label(locale).to_string(), String::new()),
            Self::Stop => ViewData::new(self.icon_name().to_string(), VoiceAssistantLabel::Stop.localized_label(locale).to_string(), String::new()),
            Self::Status => ViewData::new(self.icon_name().to_string(), VoiceAssistantLabel::Status.localized_label(locale).to_string(), String::new()),
        }
    }
}

/// Atomic voice assistant widget that renders a single voice assistant view.
///
/// Subscribes to `service.voice_assistant.status` and renders only the view
/// specified at construction time. No view switching — each atomic widget is
/// a single-purpose display with independent Click/Longpress/Hold actions
/// configured via `AtomicWidgetConfig`.
pub struct VoiceAssistantAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: VoiceAssistantAtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<AssistantStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl VoiceAssistantAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = VoiceAssistantAtomicView::from_str(widget_name).unwrap_or(VoiceAssistantAtomicView::Listen);

        let widget = VoiceAssistantAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn update_ui(&self) {
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui_with_status(status);
        } else {
            self.update_ui_default();
        }
    }

    fn update_ui_with_status(&self, status: &AssistantStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or(self.view.icon_char());
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
    }

    fn update_ui_default(&self) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render_default(&override_data);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or(self.view.icon_char());
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
    }

    /// Extract graphic rendering data from the latest status.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            let override_data = self.personalization.borrow().clone();
            let view_data = self.view.render_default(&override_data);
            let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or(self.view.icon_char());
            return AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        };

        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or(self.view.icon_char());
        AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text)
    }
}

atomic_widget_impl! {
    widget: VoiceAssistantAtomicWidget,
    debug_tag: "voice-assistant-atomic",
    mcp_description: "Voice assistant atomic widget",
    css_prefix: "voice-assistant",
    default_icon: '\u{f130}',
    default_main: "--",
    default_info: "Loading...",
    extra_message_types: [AssistantStatusMessage, FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl MessageHandler<AssistantStatusMessage> for VoiceAssistantAtomicWidget {
    fn handle_message(&self, message: AssistantStatusMessage, _sender_id: &str) {
        trace!("voice assistant atomic widget: received status {:?}", message.current_state);
        *self.latest_status.borrow_mut() = Some(message.clone());
        self.update_ui_with_status(&message);
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for VoiceAssistantAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("voice assistant atomic widget: received personalization status");
        let status = message.0;
        let locale = status
            .locale
            .as_ref()
            .map(|l| Locale::from_str(l.as_str()).unwrap_or_default())
            .unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui_with_status(status);
        } else {
            self.update_ui_default();
        }
    }
}
