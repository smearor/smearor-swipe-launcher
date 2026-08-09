use crate::config::AudioWidgetConfig;
use crate::labels::AudioLabel;
use crate::personalization::PersonalizationOverride;
use adw::gdk;
use glib::MainContext;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_audio_model::AudioCommandMessage;
use smearor_audio_model::AudioStatusMessage;
use smearor_audio_model::TOPIC_STATUS;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
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
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
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
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::apply_widget_scaled_css;
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_info_box;
use smearor_swipe_launcher_plugin_api::build_info_label_scaled;
use smearor_swipe_launcher_plugin_api::build_main_label_scaled;
use smearor_swipe_launcher_plugin_api::build_spacer_scaled;
use smearor_swipe_launcher_plugin_api::build_widget_icon_scaled;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use smearor_swipe_launcher_plugin_api::sanitize_scale;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tracing::error;
use tracing::trace;

/// View state for the audio widget's multi-view rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioView {
    /// Compact: volume icon + percentage.
    #[default]
    Compact,
    /// Expanded: volume bar + device name + mute indicator.
    Expanded,
}

/// Widget that displays and controls audio volume.
pub struct AudioWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: AudioWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<AudioStatusMessage>,
    pub(crate) status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<AudioStatusMessage>>,
    pub(crate) last_command_time: Arc<Mutex<Instant>>,
    pub(crate) icon_image: Arc<Mutex<Option<Image>>>,
    pub(crate) volume_bar: Arc<Mutex<Option<LevelBar>>>,
    pub(crate) device_label: Arc<Mutex<Option<Label>>>,
    pub(crate) main_label: Arc<Mutex<Option<Label>>>,
    pub(crate) info_label: Arc<Mutex<Option<Label>>>,
    pub(crate) current_volume: Arc<Mutex<f32>>,
    pub(crate) last_status: Rc<RefCell<Option<AudioStatusMessage>>>,
    pub(crate) current_view: Rc<RefCell<AudioView>>,
    pub(crate) personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl AudioWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let audio_config = AudioWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
        let widget = AudioWidget {
            meta,
            core_context,
            config: audio_config,
            status_sender,
            status_receiver: Some(status_receiver),
            last_command_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))),
            icon_image: Arc::new(Mutex::new(None)),
            volume_bar: Arc::new(Mutex::new(None)),
            device_label: Arc::new(Mutex::new(None)),
            main_label: Arc::new(Mutex::new(None)),
            info_label: Arc::new(Mutex::new(None)),
            current_volume: Arc::new(Mutex::new(0.5)),
            last_status: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(AudioView::Compact)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);
    }

    fn request_initial_status(&self) {
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(AudioCommandMessage::refresh());
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Switches the current view and triggers a re-render notification.
    pub(crate) fn set_view(&self, view: AudioView) {
        let mut current = self.current_view.borrow_mut();
        if *current == view {
            return;
        }
        *current = view;
        drop(current);
        self.broadcast_widget_update();
    }

    /// Toggles between Compact and Expanded views.
    pub(crate) fn toggle_view(&self) {
        let new_view = match *self.current_view.borrow() {
            AudioView::Compact => AudioView::Expanded,
            AudioView::Expanded => AudioView::Compact,
        };
        self.set_view(new_view);
    }

    pub(crate) fn select_icon_name(volume: f32, is_muted: bool) -> &'static str {
        crate::atomic::volume_icon_name(volume, is_muted)
    }

    fn set_audio_icon(image: &Image, icon_name: &str) {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            image.set_icon_name(Some(&gtk_icon_name));
            return;
        }
        image.set_icon_name(Some(icon_name));
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let icon_image = Arc::clone(&self.icon_image);
            let volume_bar = Arc::clone(&self.volume_bar);
            let device_label = Arc::clone(&self.device_label);
            let main_label = Arc::clone(&self.main_label);
            let info_label = Arc::clone(&self.info_label);
            let current_volume = Arc::clone(&self.current_volume);
            let show_compact_labels = !self.config.icon_config.icon_only();
            let locale = self.personalization.borrow().effective_locale();

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    if let Ok(guard) = icon_image.lock() {
                        if let Some(image) = guard.as_ref() {
                            Self::set_audio_icon(image, Self::select_icon_name(status.volume, status.is_muted));
                        }
                    }

                    if let Ok(guard) = volume_bar.lock() {
                        if let Some(bar) = guard.as_ref() {
                            bar.set_value(status.volume as f64);
                        }
                    }

                    if let Ok(mut guard) = current_volume.lock() {
                        *guard = status.volume;
                    }

                    let device_text = status.active_device.as_ref().map(|d| d.name.as_str()).unwrap_or("Unknown Device");

                    if let Ok(guard) = device_label.lock() {
                        if let Some(label) = guard.as_ref() {
                            label.set_text(device_text);
                        }
                    }

                    if show_compact_labels {
                        if let Ok(guard) = main_label.lock() {
                            if let Some(label) = guard.as_ref() {
                                let pct = if status.is_muted {
                                    AudioLabel::Muted.localized_label(locale).to_string()
                                } else {
                                    format!("{:.0}%", status.volume * 100.0)
                                };
                                label.set_text(&pct);
                            }
                        }

                        if let Ok(guard) = info_label.lock() {
                            if let Some(label) = guard.as_ref() {
                                label.set_text(device_text);
                            }
                        }
                    }
                }
            });
        }
    }
}

impl DefaultFallback for AudioWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::DoublePress | ActionKind::MiddleClick => {
                broadcaster.broadcast_message_to_topic(AudioCommandMessage::toggle_mute());
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(AudioCommandMessage::next_device());
            }
            ActionKind::SwipeUp | ActionKind::ScrollUp => {
                broadcaster.broadcast_message_to_topic(AudioCommandMessage::volume_up());
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                broadcaster.broadcast_message_to_topic(AudioCommandMessage::volume_down());
            }
            ActionKind::Click | ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
            ActionKind::Expand => {
                self.set_view(AudioView::Expanded);
            }
            ActionKind::Collapse => {
                self.set_view(AudioView::Compact);
            }
            ActionKind::ToggleView => {
                self.toggle_view();
            }
        }
    }

    fn default_fallback_with_button(&self, kind: &ActionKind, button: u32, broadcaster: &MessageBroadcasterInner) {
        if *kind == ActionKind::Longpress {
            match button {
                gdk::BUTTON_PRIMARY => broadcaster.broadcast_message_to_topic(AudioCommandMessage::next_device()),
                gdk::BUTTON_SECONDARY => broadcaster.broadcast_message_to_topic(AudioCommandMessage::previous_device()),
                gdk::BUTTON_MIDDLE => broadcaster.broadcast_message_to_topic(AudioCommandMessage::unmute()),
                _ => {}
            }
        } else {
            self.default_fallback(kind, broadcaster);
        }
    }

    fn default_fallback_drag(&self, kind: &ActionKind, offset_y: f64, broadcaster: &MessageBroadcasterInner) {
        const MAX_CHANGE: f32 = 0.30;
        const DRAG_RANGE: f64 = 300.0;
        let ratio = (offset_y.abs() / DRAG_RANGE).min(1.0) as f32;
        let change = ratio * MAX_CHANGE;
        let current = { if let Ok(guard) = self.current_volume.lock() { *guard } else { 0.5 } };
        match kind {
            ActionKind::SwipeUp => {
                let new_volume = (current + change).min(1.0);
                broadcaster.broadcast_message_to_topic(AudioCommandMessage::set_volume(new_volume));
            }
            ActionKind::SwipeDown => {
                let new_volume = (current - change).max(0.0);
                broadcaster.broadcast_message_to_topic(AudioCommandMessage::set_volume(new_volume));
            }
            _ => self.default_fallback(kind, broadcaster),
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<AudioStatusMessage>> for AudioWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<AudioStatusMessage>, _sender_id: &str) {
        *self.last_status.borrow_mut() = Some(message.0.clone());
        if let Ok(mut guard) = self.current_volume.lock() {
            *guard = message.0.volume;
        }
        if let Err(e) = self.status_sender.send(message.0) {
            error!("AudioWidget: Failed to send status to UI thread: {}", e);
        }
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for AudioWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("AudioWidget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
    }
}

impl AcceptTopic<FfiEnvelope> for AudioWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_MCP_INVOKE_TOOL || topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl MessageBroadcaster for AudioWidget {}

impl MessageTopicBroadcaster<AudioCommandMessage> for AudioWidget {}

impl PluginMetaGetter for AudioWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for AudioWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for AudioWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<AudioStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<AudioStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for AudioWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();
        let scale = sanitize_scale(self.config.dimensions.scale.unwrap_or(1.0));

        let content_box = build_content_box(self.config.layout.spacing_scaled(scale), &["audio-widget", "menu_button_inner"]);

        let icon = build_widget_icon_scaled(
            self.config.icon_config.icon_size(),
            self.config.icon_config.icon_color(),
            |icon| {
                Self::set_audio_icon(icon, Self::select_icon_name(0.5, false));
            },
            scale,
        );
        content_box.append(&icon);
        *self.icon_image.lock().unwrap() = Some(icon.clone());

        match self.config.mode {
            WidgetMode::Compact => {
                let show_labels = !self.config.icon_config.icon_only();

                let main_label = build_main_label_scaled(if show_labels { "50%" } else { "" }, self.config.text_colors.main_text_color(), false, None, scale);
                content_box.append(&main_label);
                *self.main_label.lock().unwrap() = Some(main_label.clone());

                let info_label = build_info_label_scaled(
                    if show_labels { "Unknown Device" } else { "" },
                    self.config.text_colors.info_text_color(),
                    true,
                    Some(self.config.max_width_chars),
                    scale,
                );
                content_box.append(&info_label);
                *self.info_label.lock().unwrap() = Some(info_label.clone());

                let spacer = build_spacer_scaled(16, scale);
                content_box.append(&spacer);
            }
            WidgetMode::Wide => {
                let info_box = build_info_box(self.config.layout.spacing_scaled(scale));

                let device_label =
                    build_main_label_scaled("Unknown Device", self.config.text_colors.main_text_color(), true, Some(self.config.max_width_chars), scale);
                info_box.append(&device_label);
                *self.device_label.lock().unwrap() = Some(device_label.clone());

                let info_label = build_info_label_scaled("", self.config.text_colors.info_text_color(), true, Some(self.config.max_width_chars), scale);
                info_box.append(&info_label);
                *self.info_label.lock().unwrap() = Some(info_label.clone());

                if self.config.show_volume_bar {
                    let volume_bar = LevelBar::builder()
                        .min_value(0.0)
                        .max_value(if self.config.allow_overdrive { 1.5 } else { 1.0 })
                        .value(0.5)
                        .width_request(self.config.dimensions.max_width_scaled(self.config.mode, scale) - 20)
                        .height_request((16.0 * scale).round() as i32)
                        .css_classes(["audio-volume-bar"])
                        .build();
                    info_box.append(&volume_bar);
                    *self.volume_bar.lock().unwrap() = Some(volume_bar.clone());
                }

                content_box.append(&info_box);
            }
        }

        let button = self.config.dimensions.build_button_scaled(self.config.mode, &content_box, "max-width-", scale);

        self.request_initial_status();
        self.start_status_listener();

        let broadcaster = self.get_broadcaster();
        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: None,
            last_command_time: Arc::clone(&self.last_command_time),
            icon_image: Arc::clone(&self.icon_image),
            volume_bar: Arc::clone(&self.volume_bar),
            device_label: Arc::clone(&self.device_label),
            main_label: Arc::clone(&self.main_label),
            info_label: Arc::clone(&self.info_label),
            current_volume: Arc::clone(&self.current_volume),
            last_status: Rc::clone(&self.last_status),
            current_view: Rc::clone(&self.current_view),
            personalization: Rc::clone(&self.personalization),
        });
        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        if scale != 1.0 {
            apply_widget_scaled_css(&button_widget, scale);
        }
        widget_self.attach_gesture_handlers(
            &button_widget,
            &widget_self.config.actions,
            &broadcaster,
            &GestureHandlersConfiguration {
                swipe_threshold: 30.0,
                scroll_throttling: Some(150),
                group_gestures: false,
                ..Default::default()
            },
        );

        button_widget
    }
}
