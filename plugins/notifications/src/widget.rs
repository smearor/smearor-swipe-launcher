use crate::config::NotificationWidgetConfig;
use adw::gdk;
use adw::gdk::pango::EllipsizeMode;
use adw::gdk::pango::WrapMode;
use glib::ControlFlow;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::EventControllerScroll;
use gtk4::EventControllerScrollFlags;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::*;
use smearor_notifications_model::NotificationCommandMessage;
use smearor_notifications_model::NotificationInfo;
use smearor_notifications_model::NotificationStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
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

    fn start_status_listener(
        &self,
        receiver: Receiver<NotificationStatusMessage>,
        list_box: Box,
        count_label: Label,
        dnd_badge: Label,
        broadcaster: MessageBroadcasterInner,
    ) {
        glib::timeout_add_local(Duration::from_millis(100), move || {
            while let Ok(status) = receiver.try_recv() {
                count_label.set_text(&format!("{}", status.unread_count));
                dnd_badge.set_visible(status.do_not_disturb);

                while let Some(child) = list_box.first_child() {
                    list_box.remove(&child);
                }

                if status.do_not_disturb {
                    let label = Label::builder().label("Do Not Disturb").css_classes(["title"]).build();
                    list_box.append(&label);
                } else if status.notifications.is_empty() {
                    let label = Label::builder().label("No notifications").css_classes(["subtitle"]).build();
                    list_box.append(&label);
                } else {
                    for notification in status.notifications.iter().take(5) {
                        let card = Self::create_notification_card(notification, &broadcaster);
                        list_box.append(&card);
                    }
                }
            }
            ControlFlow::Continue
        });
    }

    fn create_notification_card(notification: &NotificationInfo, broadcaster: &MessageBroadcasterInner) -> Box {
        let card = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .css_classes(["notification-card"])
            .build();

        let header = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();

        let icon_name = notification.icon.as_deref().unwrap_or("dialog-information-symbolic");
        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(16);
        header.append(&icon);

        let app_label = Label::builder()
            .label(&notification.app_name)
            .css_classes(["notification-app-name"])
            .halign(Align::Start)
            .hexpand(true)
            .build();
        header.append(&app_label);

        let dismiss_button = Button::builder().icon_name("window-close-symbolic").css_classes(["flat", "circular"]).build();
        let notification_id = notification.id;
        let broadcaster_clone = MessageBroadcasterInner {
            meta: broadcaster.meta.clone(),
            core_context: broadcaster.core_context,
        };
        dismiss_button.connect_clicked(move |_button| {
            debug!("NotificationWidget: Dismiss notification {notification_id}");
            broadcaster_clone.broadcast_message_to_topic(NotificationCommandMessage::dismiss_id(notification_id));
        });
        header.append(&dismiss_button);

        card.append(&header);

        if !notification.summary.is_empty() {
            let summary_label = Label::builder()
                .label(&notification.summary)
                .css_classes(["notification-summary"])
                .halign(Align::Start)
                .ellipsize(EllipsizeMode::End)
                .max_width_chars(20)
                .build();
            card.append(&summary_label);
        }

        if !notification.body.is_empty() {
            let body_label = Label::builder()
                .label(&notification.body)
                .css_classes(["notification-body"])
                .halign(Align::Start)
                .wrap(true)
                .wrap_mode(WrapMode::WordChar)
                .max_width_chars(20)
                .lines(2)
                .build();
            card.append(&body_label);
        }

        if !notification.actions.is_empty() {
            let actions_box = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();
            for action in &notification.actions {
                let action_button = Button::builder().label(&action.label).css_classes(["pill"]).build();
                let notification_id = notification.id;
                let action_key = action.key.clone();
                let broadcaster_clone = MessageBroadcasterInner {
                    meta: broadcaster.meta.clone(),
                    core_context: broadcaster.core_context,
                };
                action_button.connect_clicked(move |_button| {
                    debug!("NotificationWidget: Invoke action {action_key} on notification {notification_id}");
                    broadcaster_clone.broadcast_message_to_topic(NotificationCommandMessage::invoke_action(notification_id, action_key.clone()));
                });
                actions_box.append(&action_button);
            }
            card.append(&actions_box);
        }

        card
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
        let main_box = Box::builder().orientation(Orientation::Vertical).spacing(4).build();

        let header = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();

        let count_label = Label::builder().label("0").css_classes(["title"]).build();
        header.append(&count_label);

        let dnd_badge = Label::builder().label("DND").css_classes(["badge"]).visible(false).build();
        header.append(&dnd_badge);

        main_box.append(&header);

        let notification_list = Box::builder().orientation(Orientation::Vertical).spacing(4).vexpand(true).build();
        let empty_label = Label::builder().label("No notifications").css_classes(["subtitle"]).build();
        notification_list.append(&empty_label);
        main_box.append(&notification_list);

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
        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.width)
            .height_request(self.config.height)
            .child(&main_box)
            .build();

        button.add_controller(scroll_controller);

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
        button.add_controller(click_gesture);

        let long_press_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_long = self.get_broadcaster();
        long_press_gesture.connect_pressed(move |_gesture, _x, _y| {
            debug!("NotificationWidget: Long press (dismiss all)");
            broadcaster_long.broadcast_message_to_topic(NotificationCommandMessage::dismiss_all());
        });
        button.add_controller(long_press_gesture);

        if let Some(receiver) = self.status_receiver.take() {
            let broadcaster = self.get_broadcaster();
            self.start_status_listener(receiver, notification_list, count_label, dnd_badge, broadcaster);
        }

        button.clone().upcast::<Widget>()
    }
}
