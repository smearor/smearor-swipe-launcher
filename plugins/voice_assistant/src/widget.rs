use crate::config::VoiceAssistantWidgetConfig;
use crate::views::apply_state;
use crate::views::render_state;
use adw::gdk;
use glib::MainContext;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Spinner;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_voice_assistant_model::AssistantState;
use smearor_voice_assistant_model::AssistantStatusMessage;
use smearor_voice_assistant_model::VoiceCommandMessage;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::error;

/// Widget that displays voice assistant state and triggers voice commands.
pub struct VoiceAssistantWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: VoiceAssistantWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
    pub(crate) status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<AssistantStatusMessage>>,
    pub(crate) current_state: Arc<Mutex<AssistantState>>,
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
        Ok(VoiceAssistantWidget {
            meta,
            core_context,
            config: widget_config,
            status_sender,
            status_receiver: Some(status_receiver),
            current_state: Arc::new(Mutex::new(AssistantState::Idle)),
            icon_widget: Arc::new(Mutex::new(None)),
            label_primary: Arc::new(Mutex::new(None)),
            label_secondary: Arc::new(Mutex::new(None)),
            spinner: Arc::new(Mutex::new(None)),
        })
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let icon_widget = Arc::clone(&self.icon_widget);
            let label_primary = Arc::clone(&self.label_primary);
            let label_secondary = Arc::clone(&self.label_secondary);
            let spinner = Arc::clone(&self.spinner);
            let current_state = Arc::clone(&self.current_state);
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

                    let view_state = render_state(&state, &config, &transcript, final_answer.as_deref(), active_tool.as_deref(), error_message.as_deref());

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

impl MessageHandler<FfiEnvelopePayload<AssistantStatusMessage>> for VoiceAssistantWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<AssistantStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            error!("VoiceAssistantWidget: Failed to send status to UI thread: {}", e);
        }
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

impl Plugin for VoiceAssistantWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<AssistantStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<AssistantStatusMessage>>::handle_envelope_message(self, envelope);
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
            .spacing(self.config.spacing)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["voice-assistant-widget"])
            .build();

        let icon = Image::new();
        icon.set_pixel_size(self.config.icon_size);
        icon.set_icon_name(Some(&self.config.icon_idle));
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
            .width_request(self.config.width)
            .height_request(self.config.height)
            .child(&content_box)
            .build();

        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_click = self.get_broadcaster();
        click_gesture.connect_released(move |gesture, _n_press, _x, _y| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            let button = gesture.current_button();
            debug!("VoiceAssistantWidget: Click button={button}");
            if button == gdk::BUTTON_PRIMARY {
                debug!("VoiceAssistantWidget: Primary click -> Activate");
                broadcaster_click.broadcast_message_to_topic(VoiceCommandMessage::activate());
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(click_gesture);

        let long_press_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_long = self.get_broadcaster();
        long_press_gesture.connect_pressed(move |gesture, _x, _y| {
            let button = gesture.current_button();
            debug!("VoiceAssistantWidget: Long press button={button}");
            if button == gdk::BUTTON_PRIMARY {
                debug!("VoiceAssistantWidget: Long press -> Deactivate");
                broadcaster_long.broadcast_message_to_topic(VoiceCommandMessage::deactivate());
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(long_press_gesture);

        self.start_status_listener();

        button.clone().upcast::<Widget>()
    }
}
