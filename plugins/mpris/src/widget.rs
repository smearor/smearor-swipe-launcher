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
use gtk4::GestureZoom;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
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
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tracing::debug;
use tracing::error;

/// Widget that displays and controls MPRIS media players.
pub struct MprisWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: MprisWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<MprisStatusMessage>,
    pub(crate) status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<MprisStatusMessage>>,
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
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
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

    fn start_status_listener(&mut self) {
        let album_art = Arc::clone(&self.album_art);
        let progress_bar = Arc::clone(&self.progress_bar);
        let title_label = Arc::clone(&self.title_label);
        let artist_label = Arc::clone(&self.artist_label);
        let current_status: Arc<Mutex<Option<MprisStatusMessage>>> = Arc::new(Mutex::new(None));
        let current_status_for_ticker = Arc::clone(&current_status);
        let progress_bar_for_ticker = Arc::clone(&self.progress_bar);

        if let Some(mut receiver) = self.status_receiver.take() {
            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
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

                    if let Ok(guard) = title_label.lock() {
                        if let Some(label) = guard.as_ref() {
                            let text = if status.has_player {
                                status
                                    .metadata
                                    .as_ref()
                                    .and_then(|m| if m.title.is_empty() { None } else { Some(m.title.as_str()) })
                                    .unwrap_or("Playing")
                            } else {
                                "No Player"
                            };
                            label.set_text(text);
                        }
                    }

                    if let Ok(guard) = artist_label.lock() {
                        if let Some(label) = guard.as_ref() {
                            let text = status
                                .metadata
                                .as_ref()
                                .and_then(|m| if m.artist.is_empty() { None } else { Some(m.artist.as_str()) })
                                .unwrap_or("");
                            label.set_text(text);
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

impl MessageHandler<FfiEnvelopePayload<MprisStatusMessage>> for MprisWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<MprisStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            error!("MprisWidget: Failed to send status to UI thread: {}", e);
        }
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

impl Plugin for MprisWidget {}

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
        let last_drag_command_time = Arc::clone(&self.last_command_time);
        let broadcaster_drag = self.get_broadcaster();
        drag_gesture.connect_drag_end(move |_gesture, offset_x, offset_y| {
            const SWIPE_THRESHOLD: f64 = 60.0;
            if offset_y.abs() > offset_x.abs() && offset_y.abs() > SWIPE_THRESHOLD {
                let elapsed = {
                    let last = last_drag_command_time.lock().unwrap();
                    Instant::now().duration_since(*last)
                };
                if elapsed < Duration::from_millis(150) {
                    return;
                }
                if offset_y < 0.0 {
                    debug!("MprisWidget: Swipe up detected (next track)");
                    *last_drag_command_time.lock().unwrap() = Instant::now();
                    broadcaster_drag.broadcast_message_to_topic(MprisCommandMessage::next_track());
                } else {
                    debug!("MprisWidget: Swipe down detected (previous track)");
                    *last_drag_command_time.lock().unwrap() = Instant::now();
                    broadcaster_drag.broadcast_message_to_topic(MprisCommandMessage::previous_track());
                }
            }
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
                    debug!("MprisWidget: Right click detected (raise player)");
                    broadcaster_click.broadcast_message_to_topic(MprisCommandMessage::raise());
                }
                gdk::BUTTON_MIDDLE => {
                    debug!("MprisWidget: Middle click detected (quit player)");
                    broadcaster_click.broadcast_message_to_topic(MprisCommandMessage::quit());
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
        button.add_controller(zoom_gesture);

        self.start_status_listener();

        button.clone().upcast::<Widget>()
    }
}
