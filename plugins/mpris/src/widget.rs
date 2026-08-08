use crate::config::MprisWidgetConfig;
use crate::labels::MprisLabel;
use crate::personalization::PersonalizationOverride;
use adw::gdk;
use glib::ControlFlow;
use gtk4::GestureZoom;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisStatusMessage;
use smearor_mpris_model::TOPIC_STATUS;
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
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_info_box;
use smearor_swipe_launcher_plugin_api::build_info_label;
use smearor_swipe_launcher_plugin_api::build_main_label;
use smearor_swipe_launcher_plugin_api::build_spacer;
use smearor_swipe_launcher_plugin_api::build_widget_icon;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// View state for the MPRIS widget's multi-view rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MprisView {
    /// Compact: play/pause icon + track name.
    #[default]
    Compact,
    /// Expanded: controls + progress bar + metadata.
    Expanded,
}

/// Widget that displays and controls MPRIS media players.
pub struct MprisWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: MprisWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<MprisStatusMessage>,
    pub(crate) status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<MprisStatusMessage>>,
    pub(crate) last_command_time: Arc<Mutex<Instant>>,
    pub(crate) album_art: Arc<Mutex<Option<Image>>>,
    pub(crate) playback_icon: Arc<Mutex<Option<Image>>>,
    pub(crate) progress_bar: Arc<Mutex<Option<LevelBar>>>,
    pub(crate) title_label: Arc<Mutex<Option<Label>>>,
    pub(crate) artist_label: Arc<Mutex<Option<Label>>>,
    pub(crate) main_label: Arc<Mutex<Option<Label>>>,
    pub(crate) info_label: Arc<Mutex<Option<Label>>>,
    pub(crate) last_status: Rc<RefCell<Option<MprisStatusMessage>>>,
    pub(crate) current_view: Rc<RefCell<MprisView>>,
    pub(crate) personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl MprisWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let mpris_config = MprisWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
        let widget = MprisWidget {
            meta,
            core_context,
            config: mpris_config,
            status_sender,
            status_receiver: Some(status_receiver),
            last_command_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))),
            album_art: Arc::new(Mutex::new(None)),
            playback_icon: Arc::new(Mutex::new(None)),
            progress_bar: Arc::new(Mutex::new(None)),
            title_label: Arc::new(Mutex::new(None)),
            artist_label: Arc::new(Mutex::new(None)),
            main_label: Arc::new(Mutex::new(None)),
            info_label: Arc::new(Mutex::new(None)),
            last_status: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(MprisView::Compact)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        Ok(widget)
    }

    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);
    }

    fn request_initial_status(&self) {
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(MprisCommandMessage::refresh());
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Switches the current view and triggers a re-render notification.
    pub(crate) fn set_view(&self, view: MprisView) {
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
            MprisView::Compact => MprisView::Expanded,
            MprisView::Expanded => MprisView::Compact,
        };
        self.set_view(new_view);
    }

    fn set_mpris_icon(image: &Image, icon_name: &str) {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            image.set_icon_name(Some(&gtk_icon_name));
            return;
        }
        image.set_icon_name(Some(icon_name));
    }

    fn load_album_art(image: &Image, art_url: &Option<String>) {
        match art_url {
            Some(url) if url.starts_with("file://") => {
                let path = &url[7..];
                image.set_from_file(Some(path));
            }
            Some(_) => {
                // URLs require async download; fall back to symbolic icon for now.
                image.set_icon_name(Some("audio-x-generic-symbolic"));
            }
            None => {
                image.set_icon_name(Some("audio-x-generic-symbolic"));
            }
        }
    }

    fn start_status_listener(&mut self) {
        let album_art = Arc::clone(&self.album_art);
        let playback_icon = Arc::clone(&self.playback_icon);
        let progress_bar = Arc::clone(&self.progress_bar);
        let title_label = Arc::clone(&self.title_label);
        let artist_label = Arc::clone(&self.artist_label);
        let main_label = Arc::clone(&self.main_label);
        let info_label = Arc::clone(&self.info_label);
        let show_compact_labels = !self.config.icon_config.icon_only();
        let locale = self.personalization.borrow().effective_locale();
        let current_status: Arc<Mutex<Option<MprisStatusMessage>>> = Arc::new(Mutex::new(None));
        let current_status_for_ticker = Arc::clone(&current_status);
        let progress_bar_for_ticker = Arc::clone(&self.progress_bar);

        if let Some(mut receiver) = self.status_receiver.take() {
            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    if let Ok(guard) = playback_icon.lock() {
                        if let Some(image) = guard.as_ref() {
                            if status.has_player {
                                Self::set_mpris_icon(image, status.playback_status.playback_icon_name());
                            } else {
                                Self::set_mpris_icon(image, "nf-fa-music");
                            }
                        }
                    }

                    if let Ok(guard) = album_art.lock() {
                        if let Some(image) = guard.as_ref() {
                            if status.has_player {
                                let art_url: Option<String> = status.metadata.as_ref().and_then(|m| m.art_url.as_ref().map(|s| s.as_str().to_string()));
                                Self::load_album_art(image, &art_url);
                            } else {
                                image.set_icon_name(Some("audio-x-generic-symbolic"));
                            }
                        }
                    }

                    let title_text = if status.has_player {
                        match status.playback_status {
                            MprisPlaybackStatus::Paused => MprisLabel::Paused.localized_label(locale),
                            MprisPlaybackStatus::Stopped => MprisLabel::Stopped.localized_label(locale),
                            _ => status
                                .metadata
                                .as_ref()
                                .and_then(|m| if m.title.is_empty() { None } else { Some(m.title.as_str()) })
                                .unwrap_or(MprisLabel::Playing.localized_label(locale)),
                        }
                    } else {
                        MprisLabel::NoPlayer.localized_label(locale)
                    };

                    let artist_text = status
                        .metadata
                        .as_ref()
                        .and_then(|m| if m.artist.is_empty() { None } else { Some(m.artist.as_str()) })
                        .unwrap_or("");

                    if let Ok(guard) = title_label.lock() {
                        if let Some(label) = guard.as_ref() {
                            label.set_text(title_text);
                        }
                    }

                    if let Ok(guard) = artist_label.lock() {
                        if let Some(label) = guard.as_ref() {
                            label.set_text(artist_text);
                        }
                    }

                    if show_compact_labels {
                        if let Ok(guard) = main_label.lock() {
                            if let Some(label) = guard.as_ref() {
                                label.set_text(title_text);
                            }
                        }

                        if let Ok(guard) = info_label.lock() {
                            if let Some(label) = guard.as_ref() {
                                label.set_text(artist_text);
                            }
                        }
                    }

                    if let Ok(guard) = progress_bar.lock() {
                        if let Some(bar) = guard.as_ref() {
                            let ratio = if let Some(meta) = status.metadata.as_ref() {
                                if meta.length > 0 {
                                    (status.position as f64 / meta.length as f64).min(1.0)
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            bar.set_value(ratio);
                        }
                    }

                    *current_status.lock().unwrap() = Some(status);
                }
            });
        }

        glib::timeout_add_local(Duration::from_millis(50), move || {
            if let Ok(guard) = progress_bar_for_ticker.lock() {
                if let Some(bar) = guard.as_ref() {
                    let ratio = if let Ok(status_guard) = current_status_for_ticker.lock() {
                        if let Some(ref status) = *status_guard {
                            if let Some(meta) = status.metadata.as_ref() {
                                if meta.length > 0 {
                                    (status.position as f64 / meta.length as f64).min(1.0)
                                } else if status.playback_status == MprisPlaybackStatus::Playing {
                                    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as f64;
                                    (0.5 + 0.4 * (t / 1200.0).sin()).clamp(0.1, 0.9)
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    bar.set_value(ratio);
                }
            }
            ControlFlow::Continue
        });
    }
}

impl DefaultFallback for MprisWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::DoublePress => {
                broadcaster.broadcast_message_to_topic(MprisCommandMessage::toggle_play_pause());
            }
            ActionKind::Longpress => {
                broadcaster.broadcast_message_to_topic(MprisCommandMessage::next_player());
            }
            ActionKind::SwipeUp | ActionKind::ScrollUp => {
                broadcaster.broadcast_message_to_topic(MprisCommandMessage::next_track());
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                broadcaster.broadcast_message_to_topic(MprisCommandMessage::previous_track());
            }
            ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(MprisCommandMessage::raise());
            }
            ActionKind::MiddleClick => {
                broadcaster.broadcast_message_to_topic(MprisCommandMessage::quit());
            }
            ActionKind::Click | ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
            ActionKind::Expand => {
                self.set_view(MprisView::Expanded);
            }
            ActionKind::Collapse => {
                self.set_view(MprisView::Compact);
            }
            ActionKind::ToggleView => {
                self.toggle_view();
            }
        }
    }

    fn default_fallback_with_button(&self, kind: &ActionKind, button: u32, broadcaster: &MessageBroadcasterInner) {
        if *kind == ActionKind::Longpress {
            match button {
                gdk::BUTTON_PRIMARY => broadcaster.broadcast_message_to_topic(MprisCommandMessage::next_player()),
                gdk::BUTTON_SECONDARY => broadcaster.broadcast_message_to_topic(MprisCommandMessage::previous_player()),
                gdk::BUTTON_MIDDLE => broadcaster.broadcast_message_to_topic(MprisCommandMessage::toggle_play_pause()),
                _ => {}
            }
        } else {
            self.default_fallback(kind, broadcaster);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<MprisStatusMessage>> for MprisWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<MprisStatusMessage>, _sender_id: &str) {
        *self.last_status.borrow_mut() = Some(message.0.clone());
        if let Err(e) = self.status_sender.send(message.0) {
            error!("MprisWidget: Failed to send status to UI thread: {}", e);
        }
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for MprisWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("MprisWidget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let time_format = status.time_format;
        let override_data = PersonalizationOverride { locale, time_format };
        *self.personalization.borrow_mut() = override_data;
    }
}

impl AcceptTopic<FfiEnvelope> for MprisWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_MCP_INVOKE_TOOL || topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl MessageBroadcaster for MprisWidget {}

impl MessageTopicBroadcaster<MprisCommandMessage> for MprisWidget {}

impl PluginMetaGetter for MprisWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for MprisWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for MprisWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<MprisStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<MprisStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for MprisWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let content_box = build_content_box(self.config.layout.spacing_or_default(), &["mpris-widget", "menu_button_inner"]);

        let icon_size = self.config.icon_config.icon_size();

        match self.config.mode {
            WidgetMode::Compact => {
                let playback_icon = build_widget_icon(icon_size, self.config.icon_config.icon_color(), |icon| {
                    Self::set_mpris_icon(icon, "nf-fa-play");
                });
                content_box.append(&playback_icon);
                *self.playback_icon.lock().unwrap() = Some(playback_icon.clone());

                let show_labels = !self.config.icon_config.icon_only();

                let main_label = build_main_label(
                    if show_labels { "No Player" } else { "" },
                    self.config.text_colors.main_text_color(),
                    true,
                    Some(self.config.max_width_chars),
                );
                content_box.append(&main_label);
                *self.main_label.lock().unwrap() = Some(main_label.clone());

                let info_label = build_info_label("", self.config.text_colors.info_text_color(), true, Some(self.config.max_width_chars));
                content_box.append(&info_label);
                *self.info_label.lock().unwrap() = Some(info_label.clone());

                let spacer = build_spacer(16);
                content_box.append(&spacer);
            }
            WidgetMode::Wide => {
                if self.config.show_album_art {
                    let album_art = Image::from_icon_name("audio-x-generic-symbolic");
                    album_art.set_pixel_size(icon_size);
                    album_art.add_css_class("nerd-icon");
                    content_box.append(&album_art);
                    *self.album_art.lock().unwrap() = Some(album_art.clone());
                }

                let info_box = build_info_box(self.config.layout.spacing_or_default());

                let title_label = build_main_label("No Player", self.config.text_colors.main_text_color(), true, Some(self.config.max_width_chars));
                info_box.append(&title_label);
                *self.title_label.lock().unwrap() = Some(title_label.clone());

                let artist_label = build_info_label("", self.config.text_colors.info_text_color(), true, Some(self.config.max_width_chars));
                info_box.append(&artist_label);
                *self.artist_label.lock().unwrap() = Some(artist_label.clone());

                if self.config.show_progress_bar {
                    let progress_bar = LevelBar::builder()
                        .min_value(0.0)
                        .max_value(1.0)
                        .value(0.0)
                        .width_request(self.config.dimensions.max_width_or_default(self.config.mode) - 20)
                        .height_request(16)
                        .css_classes(["mpris-progress-bar"])
                        .build();
                    info_box.append(&progress_bar);
                    *self.progress_bar.lock().unwrap() = Some(progress_bar.clone());
                }

                content_box.append(&info_box);
            }
        }

        let button = self.config.dimensions.build_button(self.config.mode, &content_box, "max-width-");

        self.request_initial_status();
        self.request_personalization_status();
        self.start_status_listener();

        let broadcaster = self.get_broadcaster();
        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: None,
            last_command_time: Arc::clone(&self.last_command_time),
            album_art: Arc::clone(&self.album_art),
            playback_icon: Arc::clone(&self.playback_icon),
            progress_bar: Arc::clone(&self.progress_bar),
            title_label: Arc::clone(&self.title_label),
            artist_label: Arc::clone(&self.artist_label),
            main_label: Arc::clone(&self.main_label),
            info_label: Arc::clone(&self.info_label),
            last_status: Rc::clone(&self.last_status),
            current_view: Rc::clone(&self.current_view),
            personalization: Rc::clone(&self.personalization),
        });
        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        widget_self.attach_gesture_handlers(
            &button_widget,
            &widget_self.config.actions,
            &broadcaster,
            &GestureHandlersConfiguration {
                swipe_threshold: 60.0,
                drag_throttling: Some(150),
                scroll_throttling: Some(150),
                group_gestures: false,
                ..Default::default()
            },
        );

        let zoom_gesture = GestureZoom::builder().propagation_phase(PropagationPhase::Capture).build();
        let last_zoom_command_time = Arc::clone(&self.last_command_time);
        let broadcaster_zoom = self.get_broadcaster();
        let zoom_start_scale = Rc::new(RefCell::new(1.0_f64));
        let zoom_current_scale = Rc::new(RefCell::new(1.0_f64));
        let zoom_start_clone = Rc::clone(&zoom_start_scale);
        let zoom_current_clone = Rc::clone(&zoom_current_scale);
        zoom_gesture.connect_begin(move |_gesture, _sequence| {
            *zoom_start_clone.borrow_mut() = 1.0;
            *zoom_current_clone.borrow_mut() = 1.0;
        });
        let zoom_current_clone2 = Rc::clone(&zoom_current_scale);
        zoom_gesture.connect_scale_changed(move |_gesture, scale| {
            *zoom_current_clone2.borrow_mut() = scale;
        });
        let zoom_start_clone3 = Rc::clone(&zoom_start_scale);
        let zoom_current_clone3 = Rc::clone(&zoom_current_scale);
        zoom_gesture.connect_end(move |_gesture, _sequence| {
            let start = *zoom_start_clone3.borrow();
            let current = *zoom_current_clone3.borrow();
            if (current - start).abs() < 0.3 {
                return;
            }
            let elapsed = {
                let last = last_zoom_command_time.lock().unwrap();
                Instant::now().duration_since(*last)
            };
            if elapsed < Duration::from_millis(150) {
                return;
            }
            if current > start {
                debug!("MprisWidget: Pinch out detected (raise player)");
                *last_zoom_command_time.lock().unwrap() = Instant::now();
                broadcaster_zoom.broadcast_message_to_topic(MprisCommandMessage::raise());
            } else {
                debug!("MprisWidget: Pinch in detected (quit player)");
                *last_zoom_command_time.lock().unwrap() = Instant::now();
                broadcaster_zoom.broadcast_message_to_topic(MprisCommandMessage::quit());
            }
        });
        button_widget.add_controller(zoom_gesture);

        button_widget
    }
}
