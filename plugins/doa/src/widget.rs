use crate::config::DoaWidgetConfig;
use crate::labels::DoaLabel;
use crate::personalization::PersonalizationOverride;
use glib::MainContext;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_doa_model::DoaCommandMessage;
use smearor_doa_model::DoaStatusMessage;
use smearor_doa_model::DoaView;
use smearor_doa_model::TOPIC_STATUS;
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
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::error;
use tracing::trace;

/// Widget that displays Direction of Arrival information from a ReSpeaker XVF3800.
pub struct DoaWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: DoaWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<DoaStatusMessage>,
    pub(crate) status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<DoaStatusMessage>>,
    pub(crate) icon_image: Arc<Mutex<Option<Image>>>,
    pub(crate) main_label: Arc<Mutex<Option<Label>>>,
    pub(crate) info_label: Arc<Mutex<Option<Label>>>,
    pub(crate) last_status: Rc<RefCell<Option<DoaStatusMessage>>>,
    pub(crate) current_view: Rc<RefCell<DoaView>>,
    pub(crate) view_index: Rc<RefCell<usize>>,
    pub(crate) personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl DoaWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let doa_config = DoaWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
        let widget = DoaWidget {
            meta,
            core_context,
            config: doa_config,
            status_sender,
            status_receiver: Some(status_receiver),
            icon_image: Arc::new(Mutex::new(None)),
            main_label: Arc::new(Mutex::new(None)),
            info_label: Arc::new(Mutex::new(None)),
            last_status: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(DoaView::Compass)),
            view_index: Rc::new(RefCell::new(0)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
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
        broadcaster.broadcast_message_to_topic(DoaCommandMessage {
            action: smearor_doa_model::DoaCommandAction::Reconnect,
            value: 0,
        });
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Cycles to the next view in the configured view list.
    pub(crate) fn cycle_view(&self) {
        let views = &self.config.views;
        if views.is_empty() {
            return;
        }
        let mut idx = self.view_index.borrow_mut();
        *idx = (*idx + 1) % views.len();
        let new_view = views[*idx];
        drop(idx);
        let mut current = self.current_view.borrow_mut();
        if *current == new_view {
            return;
        }
        *current = new_view;
        let view = *current;
        drop(current);

        if let Some(status) = self.last_status.borrow().as_ref() {
            let locale = self.personalization.borrow().effective_locale();
            let (icon_name, main_text, info_text) = Self::render_view_data(status, &self.config, view, locale);
            if let Ok(guard) = self.icon_image.lock() {
                if let Some(image) = guard.as_ref() {
                    Self::set_doa_icon(image, &icon_name);
                }
            }
            if !self.config.icon_config.icon_only() {
                if let Ok(guard) = self.main_label.lock() {
                    if let Some(label) = guard.as_ref() {
                        label.set_text(&main_text);
                    }
                }
                if let Ok(guard) = self.info_label.lock() {
                    if let Some(label) = guard.as_ref() {
                        label.set_text(&info_text);
                    }
                }
            }
        }
        self.broadcast_widget_update();
    }

    fn set_doa_icon(image: &Image, icon_name: &str) {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            image.set_icon_name(Some(&gtk_icon_name));
            return;
        }
        image.set_icon_name(Some(icon_name));
    }

    pub(crate) fn render_view_data(status: &DoaStatusMessage, config: &DoaWidgetConfig, view: DoaView, locale: Locale) -> (String, String, String) {
        if !status.connected {
            let label = DoaLabel::Disconnected.localized_label(locale);
            return (config.icon_disconnected.clone(), label.to_string(), String::new());
        }
        if status.paused {
            let label = DoaLabel::Paused.localized_label(locale);
            return (config.icon_disconnected.clone(), label.to_string(), String::new());
        }
        match view {
            DoaView::Compass => {
                let main = format!("{}°", status.calibrated_angle);
                let direction_label = DoaLabel::from(status.direction);
                let info = format!("{} {}", DoaLabel::Compass.localized_label(locale), direction_label.localized_label(locale));
                (config.icon_compass.clone(), main, info)
            }
            DoaView::Direction => {
                let icon = config.direction_icon(&status.direction).to_string();
                let direction_label = DoaLabel::from(status.direction);
                let main = direction_label.localized_label(locale).to_string();
                let info = if status.speech_detected {
                    format!("{} {}", DoaLabel::Direction.localized_label(locale), DoaLabel::SpeechDetected.localized_label(locale))
                } else {
                    format!("{} {}", DoaLabel::Direction.localized_label(locale), DoaLabel::Silence.localized_label(locale))
                };
                (icon, main, info)
            }
            DoaView::DeviceInfo => {
                let icon = if status.speech_detected {
                    config.icon_speech.clone()
                } else {
                    config.icon_device.clone()
                };
                let main = DoaLabel::DeviceInfo.localized_label(locale).to_string();
                let info = format!("VID:{:#06x} PID:{:#06x}", status.vendor_id, status.product_id);
                (icon, main, info)
            }
        }
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let icon_image = Arc::clone(&self.icon_image);
            let main_label = Arc::clone(&self.main_label);
            let info_label = Arc::clone(&self.info_label);
            let config = self.config.clone();
            let current_view = Rc::clone(&self.current_view);
            let show_labels = !self.config.icon_config.icon_only();
            let locale = self.personalization.borrow().effective_locale();

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    let view = *current_view.borrow();
                    let (icon_name, main_text, info_text) = Self::render_view_data(&status, &config, view, locale);

                    if let Ok(guard) = icon_image.lock() {
                        if let Some(image) = guard.as_ref() {
                            Self::set_doa_icon(image, &icon_name);
                        }
                    }

                    if show_labels {
                        if let Ok(guard) = main_label.lock() {
                            if let Some(label) = guard.as_ref() {
                                label.set_text(&main_text);
                            }
                        }
                        if let Ok(guard) = info_label.lock() {
                            if let Some(label) = guard.as_ref() {
                                label.set_text(&info_text);
                            }
                        }
                    }
                }
            });
        }
    }
}

impl DefaultFallback for DoaWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::SwipeUp | ActionKind::ScrollUp => {
                self.cycle_view();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.cycle_view();
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(DoaCommandMessage {
                    action: smearor_doa_model::DoaCommandAction::Reconnect,
                    value: 0,
                });
            }
            ActionKind::DoublePress | ActionKind::MiddleClick => {
                let paused = self.last_status.borrow().as_ref().map(|s| s.paused).unwrap_or(false);
                let action = if paused {
                    smearor_doa_model::DoaCommandAction::Resume
                } else {
                    smearor_doa_model::DoaCommandAction::Pause
                };
                broadcaster.broadcast_message_to_topic(DoaCommandMessage { action, value: 0 });
            }
            ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
        }
    }

    fn default_fallback_with_button(&self, kind: &ActionKind, _button: u32, broadcaster: &MessageBroadcasterInner) {
        self.default_fallback(kind, broadcaster);
    }

    fn default_fallback_drag(&self, kind: &ActionKind, _offset_y: f64, broadcaster: &MessageBroadcasterInner) {
        self.default_fallback(kind, broadcaster);
    }
}

impl MessageHandler<FfiEnvelopePayload<DoaStatusMessage>> for DoaWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<DoaStatusMessage>, _sender_id: &str) {
        *self.last_status.borrow_mut() = Some(message.0.clone());
        if let Err(e) = self.status_sender.send(message.0) {
            error!("DoaWidget: Failed to send status to UI thread: {}", e);
        }
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for DoaWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("DoaWidget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
    }
}

impl AcceptTopic<FfiEnvelope> for DoaWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl MessageBroadcaster for DoaWidget {}

impl MessageTopicBroadcaster<DoaCommandMessage> for DoaWidget {}

impl MessageTopicBroadcaster<WidgetUpdateMessage> for DoaWidget {}

impl PluginMetaGetter for DoaWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for DoaWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for DoaWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<DoaStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<DoaStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for DoaWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let content_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(self.config.layout.spacing_or_default())
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["doa-widget", "menu_button_inner"])
            .build();

        let icon = Image::new();
        icon.set_pixel_size(self.config.icon_config.icon_size());
        icon.add_css_class("nerd-icon");
        Self::set_doa_icon(&icon, &self.config.icon_disconnected);
        if let Some(color) = self.config.icon_config.icon_color() {
            apply_icon_color(&icon, color);
        }
        content_box.append(&icon);
        *self.icon_image.lock().unwrap() = Some(icon.clone());

        let show_labels = !self.config.icon_config.icon_only();

        let main_label = Label::builder()
            .label(if show_labels { "---" } else { "" })
            .css_classes(["widget-main-text"])
            .build();
        main_label.set_height_request(20);
        apply_text_color(&main_label, self.config.text_colors.main_text_color());
        content_box.append(&main_label);
        *self.main_label.lock().unwrap() = Some(main_label.clone());

        let info_label = Label::builder()
            .label(if show_labels { "" } else { "" })
            .css_classes(["widget-info-text"])
            .build();
        info_label.set_height_request(16);
        apply_text_color(&info_label, self.config.text_colors.info_text_color());
        content_box.append(&info_label);
        *self.info_label.lock().unwrap() = Some(info_label.clone());

        let effective_width = self
            .config
            .dimensions
            .width_or_default()
            .min(self.config.dimensions.max_width_or_default(self.config.mode));
        let mut button_builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(effective_width)
            .height_request(self.config.dimensions.height_or_default())
            .child(&content_box);
        if let Some(max_w) = self.config.dimensions.max_width {
            button_builder = button_builder.hexpand(false).halign(Align::Start);
            let css_class = format!("max-width-{}", max_w);
            button_builder = button_builder.css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".max-width-{} {{ max-width: {}px; }}", max_w, max_w);
            if let Some(display) = gtk4::gdk::Display::default() {
                let provider = CssProvider::new();
                provider.load_from_string(&css);
                gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            }
        }
        let button = button_builder.build();

        self.request_initial_status();
        self.start_status_listener();

        let broadcaster = self.get_broadcaster();
        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: None,
            icon_image: Arc::clone(&self.icon_image),
            main_label: Arc::clone(&self.main_label),
            info_label: Arc::clone(&self.info_label),
            last_status: Rc::clone(&self.last_status),
            current_view: Rc::clone(&self.current_view),
            view_index: Rc::clone(&self.view_index),
            personalization: Rc::clone(&self.personalization),
        });
        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        widget_self.attach_gesture_handlers(
            &button_widget,
            &widget_self.config.actions,
            &broadcaster,
            &GestureHandlersConfiguration {
                swipe_threshold: 30.0,
                scroll_throttling: Some(150),
                group_gestures: true,
                ..Default::default()
            },
        );

        button_widget
    }
}
