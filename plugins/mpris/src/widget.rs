use crate::config::MprisWidgetConfig;
use adw::gdk;
use adw::gdk::pango::EllipsizeMode;
use glib::ControlFlow;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::EventControllerScroll;
use gtk4::EventControllerScrollFlags;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;

/// Widget that displays and controls MPRIS media players.
pub struct MprisWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: MprisWidgetConfig,
    pub(crate) status_sender: Sender<MprisStatusMessage>,
    pub(crate) status_receiver: Option<Receiver<MprisStatusMessage>>,
    pub(crate) last_command_time: Arc<Mutex<Instant>>,
    pub(crate) album_art: Arc<Mutex<Option<Image>>>,
    pub(crate) progress_bar: Arc<Mutex<Option<LevelBar>>>,
    pub(crate) title_label: Arc<Mutex<Option<Label>>>,
    pub(crate) artist_label: Arc<Mutex<Option<Label>>>,
}

impl MprisWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let mpris_config = MprisWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = mpsc::channel();
        Ok(MprisWidget {
            meta,
            core_context,
            config: mpris_config,
            status_sender,
            status_receiver: Some(status_receiver),
            last_command_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))),
            album_art: Arc::new(Mutex::new(None)),
            progress_bar: Arc::new(Mutex::new(None)),
            title_label: Arc::new(Mutex::new(None)),
            artist_label: Arc::new(Mutex::new(None)),
        })
    }

    fn select_playback_icon(status: &MprisPlaybackStatus) -> &'static str {
        match status {
            MprisPlaybackStatus::Playing => "media-playback-pause-symbolic",
            MprisPlaybackStatus::Paused => "media-playback-start-symbolic",
            MprisPlaybackStatus::Stopped => "media-playback-start-symbolic",
        }
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

    fn start_status_listener(&self, receiver: Receiver<MprisStatusMessage>) {
        let album_art = Arc::clone(&self.album_art);
        let progress_bar = Arc::clone(&self.progress_bar);
        let title_label = Arc::clone(&self.title_label);
        let artist_label = Arc::clone(&self.artist_label);

        glib::timeout_add_local(Duration::from_millis(10), move || {
            while let Ok(status) = receiver.try_recv() {
                if let Ok(guard) = album_art.lock() {
                    if let Some(image) = guard.as_ref() {
                        if status.has_player {
                            Self::load_album_art(image, &status.metadata.as_ref().and_then(|m| m.art_url.clone()));
                        } else {
                            image.set_icon_name(Some("audio-x-generic-symbolic"));
                        }
                    }
                }

                if let Ok(guard) = progress_bar.lock() {
                    if let Some(bar) = guard.as_ref() {
                        let ratio = if let Some(ref meta) = status.metadata {
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

                if let Ok(guard) = title_label.lock() {
                    if let Some(label) = guard.as_ref() {
                        let text = status.metadata.as_ref().map(|m| m.title.as_str()).unwrap_or("No Player");
                        label.set_text(text);
                    }
                }

                if let Ok(guard) = artist_label.lock() {
                    if let Some(label) = guard.as_ref() {
                        let text = status.metadata.as_ref().map(|m| m.artist.as_str()).unwrap_or("");
                        label.set_text(text);
                    }
                }
            }
            ControlFlow::Continue
        });
    }
}

impl MessageHandler<FfiEnvelopePayload<MprisStatusMessage>> for MprisWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<MprisStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            error!("MprisWidget: Failed to send status to UI thread: {}", e);
        }
    }
}

impl MessageBroadcaster<MprisCommandMessage> for MprisWidget {}

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

impl WidgetBuilder for MprisWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["mpris-widget"])
            .build();

        if self.config.show_album_art {
            let album_art = Image::from_icon_name("audio-x-generic-symbolic");
            album_art.set_pixel_size(48);
            main_box.append(&album_art);
            *self.album_art.lock().unwrap() = Some(album_art.clone());
        }

        if self.config.show_progress_bar {
            let progress_bar = LevelBar::builder()
                .min_value(0.0)
                .max_value(1.0)
                .value(0.0)
                .width_request(self.config.width - 20)
                .height_request(8)
                .css_classes(["mpris-progress-bar"])
                .build();
            main_box.append(&progress_bar);
            *self.progress_bar.lock().unwrap() = Some(progress_bar.clone());
        }

        let title_label = Label::builder()
            .label("No Player")
            .ellipsize(EllipsizeMode::End)
            .max_width_chars(12)
            .css_classes(["mpris-title-label"])
            .build();
        main_box.append(&title_label);
        *self.title_label.lock().unwrap() = Some(title_label.clone());

        let artist_label = Label::builder()
            .label("")
            .ellipsize(EllipsizeMode::End)
            .max_width_chars(12)
            .css_classes(["mpris-artist-label"])
            .build();
        main_box.append(&artist_label);
        *self.artist_label.lock().unwrap() = Some(artist_label.clone());

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.width)
            .height_request(self.config.height)
            .child(&main_box)
            .build();

        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let broadcaster_drag = self.get_broadcaster();
        drag_gesture.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
            // Drag gestures are reserved for future use (e.g. seeking).
            // No action is taken on drag end to avoid accidental triggers.
            let _ = broadcaster_drag;
        });
        button.add_controller(drag_gesture);

        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        let last_command_time_scroll = Arc::clone(&self.last_command_time);
        let broadcaster_scroll = self.get_broadcaster();
        scroll_controller.connect_scroll(move |_controller, _dx, dy| {
            let elapsed = {
                let last = last_command_time_scroll.lock().unwrap();
                Instant::now().duration_since(*last)
            };
            if elapsed < Duration::from_millis(150) {
                return glib::Propagation::Stop;
            }
            if dy < 0.0 {
                debug!("MprisWidget: Scroll up detected (next track)");
                *last_command_time_scroll.lock().unwrap() = Instant::now();
                broadcaster_scroll.broadcast_message_to_topic(MprisCommandMessage::next_track());
            } else if dy > 0.0 {
                debug!("MprisWidget: Scroll down detected (previous track)");
                *last_command_time_scroll.lock().unwrap() = Instant::now();
                broadcaster_scroll.broadcast_message_to_topic(MprisCommandMessage::previous_track());
            }
            glib::Propagation::Stop
        });
        button.add_controller(scroll_controller);

        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_click = self.get_broadcaster();
        click_gesture.connect_released(move |gesture, n_press, _x, _y| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            let button = gesture.current_button();
            debug!("Button = {button} n_press = {n_press}");
            match button {
                gdk::BUTTON_PRIMARY => {
                    if n_press == 2 {
                        debug!("MprisWidget: Double tap / double click (toggle play/pause)");
                        broadcaster_click.broadcast_message_to_topic(MprisCommandMessage::toggle_play_pause());
                    }
                }
                gdk::BUTTON_SECONDARY => {
                    debug!("MprisWidget: Right click detected (next player)");
                    broadcaster_click.broadcast_message_to_topic(MprisCommandMessage::next_player());
                }
                gdk::BUTTON_MIDDLE => {
                    debug!("MprisWidget: Middle click detected (toggle play/pause)");
                    broadcaster_click.broadcast_message_to_topic(MprisCommandMessage::toggle_play_pause());
                }
                _ => {}
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(click_gesture);

        let long_press_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_long = self.get_broadcaster();
        long_press_gesture.connect_pressed(move |gesture, _x, _y| {
            let button = gesture.current_button();
            debug!("Button = {button}");
            match button {
                gdk::BUTTON_PRIMARY => {
                    debug!("MprisWidget: Long press detected (next player)");
                    broadcaster_long.broadcast_message_to_topic(MprisCommandMessage::next_player());
                }
                gdk::BUTTON_SECONDARY => {
                    debug!("MprisWidget: Right long press detected (previous player)");
                    broadcaster_long.broadcast_message_to_topic(MprisCommandMessage::previous_player());
                }
                gdk::BUTTON_MIDDLE => {
                    debug!("MprisWidget: Middle long press detected (toggle play/pause)");
                    broadcaster_long.broadcast_message_to_topic(MprisCommandMessage::toggle_play_pause());
                }
                _ => {}
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(long_press_gesture);

        if let Some(receiver) = self.status_receiver.take() {
            self.start_status_listener(receiver);
        }

        button.clone().upcast::<Widget>()
    }
}
