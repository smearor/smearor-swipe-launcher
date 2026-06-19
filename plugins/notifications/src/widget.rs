use crate::config::NotificationWidgetConfig;
use adw::gdk;
use adw::gdk::pango::EllipsizeMode;
use adw::gdk::pango::WrapMode;
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
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use smearor_notifications_model::NotificationCommandMessage;
use smearor_notifications_model::NotificationInfo;
use smearor_notifications_model::NotificationStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
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
use tracing::debug;
use tracing::error;

/// Widget that displays system notifications.
pub struct NotificationWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: NotificationWidgetConfig,
    pub(crate) status_sender: tokio::sync::mpsc::UnboundedSender<NotificationStatusMessage>,
    pub(crate) status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<NotificationStatusMessage>>,
}

impl NotificationWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let notification_config = NotificationWidgetConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;
        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel();
        Ok(NotificationWidget {
            meta,
            core_context,
            config: notification_config,
            status_sender,
            status_receiver: Some(status_receiver),
        })
    }

    fn start_status_listener(&mut self, list_box: Box, count_label: Label, dnd_badge: Label, broadcaster: MessageBroadcasterInner) {
        if let Some(mut receiver) = self.status_receiver.take() {
            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
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
            });
        }
    }

    fn create_notification_card(notification: &NotificationInfo, broadcaster: &MessageBroadcasterInner) -> Box {
        let card = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .css_classes(["notification-card"])
            .width_request(180)
            .halign(Align::Start)
            .build();

        let header = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();

        let icon_name = notification.icon.as_ref().map(|s| s.as_str()).unwrap_or("dialog-information-symbolic");
        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(16);
        header.append(&icon);

        let app_label = Label::builder()
            .label(notification.app_name.as_str())
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
                .label(notification.summary.as_str())
                .css_classes(["notification-summary"])
                .halign(Align::Start)
                .ellipsize(EllipsizeMode::End)
                .max_width_chars(20)
                .build();
            card.append(&summary_label);
        }

        if !notification.body.is_empty() {
            let body_label = Label::builder()
                .label(notification.body.as_str())
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
                let action_button = Button::builder().label(action.label.as_str()).css_classes(["pill"]).build();
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

        let card_notification_id = notification.id;
        let card_broadcaster = MessageBroadcasterInner {
            meta: broadcaster.meta.clone(),
            core_context: broadcaster.core_context,
        };
        let card_click = GestureClick::builder()
            .button(gdk::BUTTON_SECONDARY)
            .propagation_phase(PropagationPhase::Bubble)
            .build();
        card_click.connect_released(move |gesture, _n_press, _x, _y| {
            debug!("NotificationWidget: Right-click dismiss notification {card_notification_id}");
            card_broadcaster.broadcast_message_to_topic(NotificationCommandMessage::dismiss_id(card_notification_id));
            gesture.set_state(EventSequenceState::Claimed);
        });
        card.add_controller(card_click);

        let long_notification_id = notification.id;
        let long_broadcaster = MessageBroadcasterInner {
            meta: broadcaster.meta.clone(),
            core_context: broadcaster.core_context,
        };
        let card_long_press = GestureLongPress::builder().propagation_phase(PropagationPhase::Bubble).build();
        card_long_press.connect_pressed(move |_gesture, _x, _y| {
            debug!("NotificationWidget: Long press dismiss notification {long_notification_id}");
            long_broadcaster.broadcast_message_to_topic(NotificationCommandMessage::dismiss_id(long_notification_id));
        });
        card.add_controller(card_long_press);

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

impl MessageBroadcaster for NotificationWidget {}
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

impl Plugin for NotificationWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<NotificationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<NotificationStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for NotificationWidget {
    fn build_widget(&mut self) -> Widget {
        let main_box = Box::builder().orientation(Orientation::Vertical).spacing(self.config.spacing).build();

        let header = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();

        let count_label = Label::builder().label("0").css_classes(["title"]).build();
        header.append(&count_label);

        let dnd_badge = Label::builder().label("DND").css_classes(["badge"]).visible(false).build();
        header.append(&dnd_badge);

        main_box.append(&header);

        let notification_list = Box::builder().orientation(Orientation::Vertical).spacing(4).build();
        let empty_label = Label::builder().label("No notifications").css_classes(["subtitle"]).build();
        notification_list.append(&empty_label);

        let scrolled = ScrolledWindow::builder()
            .child(&notification_list)
            .height_request(200)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();
        main_box.append(&scrolled);

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.width)
            .height_request(self.config.height)
            .child(&main_box)
            .build();

        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Bubble).build();
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
                    debug!("NotificationWidget: Right click (dismiss all)");
                    broadcaster_click.broadcast_message_to_topic(NotificationCommandMessage::dismiss_all());
                }
                _ => {}
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(click_gesture);

        if self.status_receiver.is_some() {
            let broadcaster = self.get_broadcaster();
            self.start_status_listener(notification_list, count_label, dnd_badge, broadcaster);
        }

        button.clone().upcast::<Widget>()
    }
}
