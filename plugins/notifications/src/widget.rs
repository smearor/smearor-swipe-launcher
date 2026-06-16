use crate::config::NotificationWidgetConfig;
use adw::gdk;
use glib::ControlFlow;
use gtk4::Box;
use gtk4::EventControllerScroll;
use gtk4::EventControllerScrollFlags;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_notifications_model::NotificationCommandMessage;
use smearor_notifications_model::NotificationStatusMessage;
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

/// Widget that displays system notifications.
pub struct NotificationWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: NotificationWidgetConfig,
    pub(crate) status_sender: Sender<NotificationStatusMessage>,
    pub(crate) status_receiver: Option<Receiver<NotificationStatusMessage>>,
    pub(crate) last_command_time: Arc<Mutex<Instant>>,
}

impl NotificationWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let notification_config = NotificationWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = mpsc::channel();
        Ok(NotificationWidget {
            meta,
            core_context,
            config: notification_config,
            status_sender,
            status_receiver: Some(status_receiver),
            last_command_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))),
        })
    }

    fn start_status_listener(&self, receiver: Receiver<NotificationStatusMessage>, count_label: Label, dnd_label: Label) {
        glib::timeout_add_local(Duration::from_millis(100), move || {
            while let Ok(status) = receiver.try_recv() {
                let count = status.notifications.len();
                count_label.set_text(&format!("{count}"));
                let dnd_text = if status.do_not_disturb { "DND" } else { "" };
                dnd_label.set_text(dnd_text);
            }
            ControlFlow::Continue
        });
    }
}

impl MessageHandler<FfiEnvelopePayload<NotificationStatusMessage>> for NotificationWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<NotificationStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            error!("NotificationWidget: Failed to send status to UI thread: {}", e);
        }
    }
}

impl MessageBroadcaster<NotificationCommandMessage> for NotificationWidget {}
impl MessageTopicBroadcaster<NotificationCommandMessage> for NotificationWidget {}
impl PluginMetaGetter for NotificationWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}
impl AsRef<Option<FfiCoreContext>> for NotificationWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetBuilder for NotificationWidget {
    fn build_widget(&mut self) -> Widget {
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .width_request(self.config.width)
            .height_request(self.config.height)
            .css_classes(["scroll-item"])
            .build();

        let count_label = Label::builder().label("0").css_classes(["title"]).build();
        container.append(&count_label);

        let dnd_label = Label::builder().label("").css_classes(["subtitle"]).build();
        container.append(&dnd_label);

        let scroll_controller = EventControllerScroll::builder()
            .flags(EventControllerScrollFlags::VERTICAL)
            .propagation_phase(PropagationPhase::Capture)
            .build();
        let last_command_time_scroll = Arc::clone(&self.last_command_time);
        let broadcaster_scroll = self.get_broadcaster();
        scroll_controller.connect_scroll(move |_controller, _dx, dy| {
            if dy < 0.0 {
                let elapsed = {
                    let last = last_command_time_scroll.lock().unwrap();
                    Instant::now().duration_since(*last)
                };
                if elapsed >= Duration::from_millis(150) {
                    debug!("NotificationWidget: Scroll up detected (dismiss last)");
                    *last_command_time_scroll.lock().unwrap() = Instant::now();
                    broadcaster_scroll.broadcast_message_to_topic(NotificationCommandMessage::dismiss_last());
                }
            }
            glib::Propagation::Stop
        });
        container.add_controller(scroll_controller);

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
            debug!("Button = {button}");
            match button {
                gdk::BUTTON_PRIMARY => {
                    debug!("NotificationWidget: Primary click (dismiss all)");
                    broadcaster_click.broadcast_message_to_topic(NotificationCommandMessage::dismiss_all());
                }
                gdk::BUTTON_SECONDARY => {
                    debug!("NotificationWidget: Right click (toggle DND)");
                    broadcaster_click.broadcast_message_to_topic(NotificationCommandMessage::toggle_do_not_disturb());
                }
                _ => {}
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        container.add_controller(click_gesture);

        let long_press_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_long = self.get_broadcaster();
        long_press_gesture.connect_pressed(move |_gesture, _x, _y| {
            debug!("NotificationWidget: Long press (dismiss all)");
            broadcaster_long.broadcast_message_to_topic(NotificationCommandMessage::dismiss_all());
        });
        container.add_controller(long_press_gesture);

        if let Some(receiver) = self.status_receiver.take() {
            self.start_status_listener(receiver, count_label.clone(), dnd_label.clone());
        }

        container.clone().upcast::<Widget>()
    }
}
