use crate::config::AudioWidgetConfig;
use adw::gdk;
use adw::gdk::pango::EllipsizeMode;
use glib::MainContext;
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
use smearor_audio_model::AudioCommandMessage;
use smearor_audio_model::AudioStatusMessage;
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
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;

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
    pub(crate) current_volume: Arc<Mutex<f32>>,
}

impl AudioWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let audio_config = AudioWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
        Ok(AudioWidget {
            meta,
            core_context,
            config: audio_config,
            status_sender,
            status_receiver: Some(status_receiver),
            last_command_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))),
            icon_image: Arc::new(Mutex::new(None)),
            volume_bar: Arc::new(Mutex::new(None)),
            device_label: Arc::new(Mutex::new(None)),
            current_volume: Arc::new(Mutex::new(0.5)),
        })
    }

    fn select_icon_name(volume: f32, is_muted: bool) -> &'static str {
        if is_muted {
            "audio-volume-muted-symbolic"
        } else if volume > 1.0 {
            "audio-volume-overamplified-symbolic"
        } else if volume > 0.66 {
            "audio-volume-high-symbolic"
        } else if volume > 0.33 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-low-symbolic"
        }
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let icon_image = Arc::clone(&self.icon_image);
            let volume_bar = Arc::clone(&self.volume_bar);
            let device_label = Arc::clone(&self.device_label);
            let current_volume = Arc::clone(&self.current_volume);

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    if let Ok(guard) = icon_image.lock() {
                        if let Some(image) = guard.as_ref() {
                            image.set_icon_name(Some(Self::select_icon_name(status.volume, status.is_muted)));
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

                    if let Ok(guard) = device_label.lock() {
                        if let Some(label) = guard.as_ref() {
                            let text = status.active_device.as_ref().map(|d| d.name.as_str()).unwrap_or("Unknown Device");
                            label.set_text(text);
                        }
                    }
                }
            });
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<AudioStatusMessage>> for AudioWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<AudioStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            error!("AudioWidget: Failed to send status to UI thread: {}", e);
        }
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

impl Plugin for AudioWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<AudioStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<AudioStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for AudioWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let content_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(self.config.spacing)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["audio-widget"])
            .build();

        let icon = Image::from_icon_name(Self::select_icon_name(0.5, false));
        icon.set_pixel_size(self.config.icon_size);
        content_box.append(&icon);
        *self.icon_image.lock().unwrap() = Some(icon.clone());

        let info_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(self.config.spacing)
            .valign(Align::Center)
            .halign(Align::Start)
            .build();

        if self.config.show_device_label {
            let device_label = Label::builder()
                .label("Unknown Device")
                .ellipsize(EllipsizeMode::End)
                .max_width_chars(self.config.max_width_chars)
                .css_classes(["audio-device-label"])
                .build();
            info_box.append(&device_label);
            *self.device_label.lock().unwrap() = Some(device_label.clone());
        }

        if self.config.show_volume_bar {
            let volume_bar = LevelBar::builder()
                .min_value(0.0)
                .max_value(if self.config.allow_overdrive { 1.5 } else { 1.0 })
                .value(0.5)
                .width_request(self.config.width - 20)
                .height_request(8)
                .css_classes(["audio-volume-bar"])
                .build();
            info_box.append(&volume_bar);
            *self.volume_bar.lock().unwrap() = Some(volume_bar.clone());
        }

        content_box.append(&info_box);

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.width)
            .height_request(self.config.height)
            .child(&content_box)
            .build();

        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let current_volume_drag = Arc::clone(&self.current_volume);
        let broadcaster_drag = self.get_broadcaster();
        drag_gesture.connect_drag_end(move |_gesture, _offset_x, offset_y| {
            const DRAG_THRESHOLD: f64 = 30.0;
            const MAX_CHANGE: f32 = 0.30;
            const DRAG_RANGE: f64 = 300.0;

            if offset_y.abs() < DRAG_THRESHOLD {
                return;
            }

            let ratio = (offset_y.abs() / DRAG_RANGE).min(1.0) as f32;
            let change = ratio * MAX_CHANGE;

            let current = { if let Ok(guard) = current_volume_drag.lock() { *guard } else { 0.5 } };

            let new_volume = if offset_y < 0.0 {
                // Drag up: increase volume
                (current + change).min(1.0)
            } else {
                // Drag down: decrease volume
                (current - change).max(0.0)
            };

            debug!("AudioWidget: Drag detected (offset_y={offset_y}), new_volume={new_volume}");
            broadcaster_drag.broadcast_message_to_topic(AudioCommandMessage::set_volume(new_volume));
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
                debug!("AudioWidget: Scroll up detected");
                *last_command_time_scroll.lock().unwrap() = Instant::now();
                broadcaster_scroll.broadcast_message_to_topic(AudioCommandMessage::volume_up());
            } else if dy > 0.0 {
                debug!("AudioWidget: Scroll down detected");
                *last_command_time_scroll.lock().unwrap() = Instant::now();
                broadcaster_scroll.broadcast_message_to_topic(AudioCommandMessage::volume_down());
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
                        debug!("AudioWidget: Double tap / double click (toggle mute)");
                        broadcaster_click.broadcast_message_to_topic(AudioCommandMessage::toggle_mute());
                    }
                }
                gdk::BUTTON_SECONDARY => {
                    debug!("AudioWidget: Double right click detected (next device)");
                    broadcaster_click.broadcast_message_to_topic(AudioCommandMessage::next_device());
                }
                gdk::BUTTON_MIDDLE => {
                    debug!("AudioWidget: Middle click detected (toggle mute)");
                    broadcaster_click.broadcast_message_to_topic(AudioCommandMessage::toggle_mute());
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
                    debug!("AudioWidget: Long press detected (next device)");
                    broadcaster_long.broadcast_message_to_topic(AudioCommandMessage::next_device());
                }
                gdk::BUTTON_SECONDARY => {
                    debug!("AudioWidget: Right long press detected (next device)");
                    broadcaster_long.broadcast_message_to_topic(AudioCommandMessage::previous_device());
                }
                gdk::BUTTON_MIDDLE => {
                    debug!("AudioWidget: Middle long press detected (toggle mute)");
                    broadcaster_long.broadcast_message_to_topic(AudioCommandMessage::unmute());
                }
                _ => {}
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(long_press_gesture);

        self.start_status_listener();

        button.clone().upcast::<Widget>()
    }
}
