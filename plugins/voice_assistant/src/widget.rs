use crate::config::VoiceAssistantWidgetConfig;
use crate::personalization::PersonalizationOverride;
use crate::views::apply_state;
use crate::views::render_state;
use glib::MainContext;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Spinner;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_voice_assistant_model::AssistantState;
use smearor_voice_assistant_model::AssistantStatusMessage;
use smearor_voice_assistant_model::TOPIC_STATUS;
use smearor_voice_assistant_model::VoiceCommandMessage;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::error;
use tracing::trace;

/// View state for the voice assistant widget's multi-view rendering.
///
/// Maps the detailed `AssistantState` to three broad visual categories
/// used by headless and web renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VoiceAssistantView {
    /// Idle or standby: microphone icon.
    #[default]
    Idle,
    /// Listening or processing: pulsing mic icon + waveform.
    Listening,
    /// Speaking or error: speaker icon + truncated text.
    Speaking,
}

impl VoiceAssistantView {
    /// Maps an `AssistantState` to a `VoiceAssistantView`.
    pub fn from_state(state: &AssistantState) -> Self {
        match state {
            AssistantState::Idle | AssistantState::Standby => Self::Idle,
            AssistantState::Listening | AssistantState::ProcessingStt | AssistantState::ThinkingLlm | AssistantState::ExecutingAction => Self::Listening,
            AssistantState::Speaking | AssistantState::Error => Self::Speaking,
        }
    }

    /// Returns the default nerd font icon codepoint for this view.
    pub fn icon_char(&self) -> char {
        match self {
            Self::Idle => '\u{f021}',
            Self::Listening => '\u{f130}',
            Self::Speaking => '\u{f028}',
        }
    }

    /// Returns the default nerd font icon name for this view.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Idle => "nf-md-microphone_off",
            Self::Listening => "nf-md-microphone",
            Self::Speaking => "nf-md-volume_high",
        }
    }
}

/// Widget that displays voice assistant state and triggers voice commands.
pub struct VoiceAssistantWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: VoiceAssistantWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
    pub(crate) status_receiver: Rc<RefCell<Option<tokio::sync::mpsc::UnboundedReceiver<AssistantStatusMessage>>>>,
    pub(crate) current_state: Arc<Mutex<AssistantState>>,
    pub(crate) last_status: Rc<RefCell<Option<AssistantStatusMessage>>>,
    pub(crate) current_view: Rc<RefCell<VoiceAssistantView>>,
    pub(crate) personalization: Rc<RefCell<PersonalizationOverride>>,
    pub(crate) icon_widget: Arc<Mutex<Option<Image>>>,
    pub(crate) label_primary: Arc<Mutex<Option<Label>>>,
    pub(crate) label_secondary: Arc<Mutex<Option<Label>>>,
    pub(crate) spinner: Arc<Mutex<Option<Spinner>>>,
}

impl VoiceAssistantWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config = VoiceAssistantWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
        let widget = VoiceAssistantWidget {
            meta,
            core_context,
            config: widget_config,
            status_sender,
            status_receiver: Rc::new(RefCell::new(Some(status_receiver))),
            current_state: Arc::new(Mutex::new(AssistantState::Idle)),
            last_status: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(VoiceAssistantView::Idle)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
            icon_widget: Arc::new(Mutex::new(None)),
            label_primary: Arc::new(Mutex::new(None)),
            label_secondary: Arc::new(Mutex::new(None)),
            spinner: Arc::new(Mutex::new(None)),
        };
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);
    }

    /// Updates the current view based on the assistant state and broadcasts a widget update.
    pub(crate) fn update_view(&self, state: &AssistantState) {
        let new_view = VoiceAssistantView::from_state(state);
        let mut current = self.current_view.borrow_mut();
        if *current == new_view {
            return;
        }
        *current = new_view;
        drop(current);
        self.broadcast_widget_update();
    }

    fn start_status_listener(&self) {
        if let Some(mut receiver) = self.status_receiver.borrow_mut().take() {
            let icon_widget = Arc::clone(&self.icon_widget);
            let label_primary = Arc::clone(&self.label_primary);
            let label_secondary = Arc::clone(&self.label_secondary);
            let spinner = Arc::clone(&self.spinner);
            let current_state = Arc::clone(&self.current_state);
            let current_view = Rc::clone(&self.current_view);
            let personalization = Rc::clone(&self.personalization);
            let config = self.config.clone();

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    let state = status.current_state.clone();
                    let transcript = status.partial_transcript.clone();
                    let final_answer = status.final_answer.clone();
                    let active_tool = status.active_tool.clone();
                    let error_message = status.error_message.clone();

                    if let Ok(mut guard) = current_state.lock() {
                        *guard = state.clone();
                    }

                    let new_view = VoiceAssistantView::from_state(&state);
                    *current_view.borrow_mut() = new_view;

                    let locale = personalization.borrow().effective_locale();
                    let view_state = render_state(
                        &state,
                        &config,
                        &transcript,
                        final_answer.as_deref(),
                        active_tool.as_deref(),
                        error_message.as_deref(),
                        locale,
                    );

                    if let (Ok(icon_guard), Ok(primary_guard), Ok(secondary_guard), Ok(spinner_guard)) =
                        (icon_widget.lock(), label_primary.lock(), label_secondary.lock(), spinner.lock())
                    {
                        if let (Some(icon), Some(primary), Some(secondary), Some(spinner)) =
                            (icon_guard.as_ref(), primary_guard.as_ref(), secondary_guard.as_ref(), spinner_guard.as_ref())
                        {
                            apply_state(&view_state, icon, primary, secondary, spinner, config.show_icon, config.show_transcript);
                        }
                    }
                }
            });
        }
    }
}

impl DefaultFallback for VoiceAssistantWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::DoublePress => {
                broadcaster.broadcast_message_to_topic(VoiceCommandMessage::activate());
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(VoiceCommandMessage::deactivate());
            }
            ActionKind::SwipeUp
            | ActionKind::ScrollUp
            | ActionKind::SwipeDown
            | ActionKind::ScrollDown
            | ActionKind::MiddleClick
            | ActionKind::Hold
            | ActionKind::CompoundLongpress
            | ActionKind::Init => {}
        }
    }
}

impl MessageHandler<AssistantStatusMessage> for VoiceAssistantWidget {
    fn handle_message(&self, message: AssistantStatusMessage, _sender_id: &str) {
        *self.last_status.borrow_mut() = Some(message.clone());
        self.update_view(&message.current_state);
        if let Err(e) = self.status_sender.send(message) {
            error!("VoiceAssistantWidget: Failed to send status to UI thread: {}", e);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for VoiceAssistantWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("VoiceAssistantWidget: received personalization status");
        let status = message.0;
        let locale = status
            .locale
            .as_ref()
            .map(|l| Locale::from_str(l.as_str()).unwrap_or_default())
            .unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
    }
}

impl MessageBroadcaster for VoiceAssistantWidget {}

impl MessageTopicBroadcaster<VoiceCommandMessage> for VoiceAssistantWidget {}

impl PluginMetaGetter for VoiceAssistantWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for VoiceAssistantWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl AcceptTopic<FfiEnvelope> for VoiceAssistantWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl WidgetPlugin for VoiceAssistantWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == AssistantStatusMessage::TYPE_ID {
                    MessageHandler::<AssistantStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for VoiceAssistantWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let content_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(self.config.layout.spacing_or_default())
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["voice-assistant-widget"])
            .build();

        let icon = Image::new();
        icon.set_pixel_size(self.config.icon_size);
        crate::views::set_icon_image(&icon, &self.config.icon_idle);
        content_box.append(&icon);
        *self.icon_widget.lock().unwrap() = Some(icon.clone());

        let label_primary = Label::builder().label("").css_classes(["voice-assistant-primary"]).build();
        content_box.append(&label_primary);
        *self.label_primary.lock().unwrap() = Some(label_primary.clone());

        let label_secondary = Label::builder().label("").css_classes(["voice-assistant-secondary"]).build();
        label_secondary.set_visible(false);
        content_box.append(&label_secondary);
        *self.label_secondary.lock().unwrap() = Some(label_secondary.clone());

        let spinner = Spinner::new();
        spinner.set_visible(false);
        content_box.append(&spinner);
        *self.spinner.lock().unwrap() = Some(spinner.clone());

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.dimensions.width_or_default())
            .height_request(self.config.dimensions.height_or_default())
            .child(&content_box)
            .build();

        let broadcaster = self.get_broadcaster();
        let button_widget = button.upcast::<Widget>();

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: self.status_receiver.clone(),
            current_state: Arc::clone(&self.current_state),
            last_status: Rc::clone(&self.last_status),
            current_view: Rc::clone(&self.current_view),
            personalization: Rc::clone(&self.personalization),
            icon_widget: Arc::clone(&self.icon_widget),
            label_primary: Arc::clone(&self.label_primary),
            label_secondary: Arc::clone(&self.label_secondary),
            spinner: Arc::clone(&self.spinner),
        });
        widget_self.attach_gesture_handlers(&button_widget, &self.config.actions, &broadcaster, &GestureHandlersConfiguration::default());

        self.start_status_listener();

        button_widget
    }
}
