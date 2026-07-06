use crate::config::PowerWidgetConfig;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Button;
use gtk4::GestureClick;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerCommandMessage;
use smearor_power_model::PowerStatusMessage;
use smearor_power_model::TOPIC_STATUS;
use smearor_power_model::power_action_icon_unicode;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

type SharedLabel = Rc<RefCell<Option<Label>>>;

pub struct PowerWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PowerWidgetConfig,
    pub status_sender: tokio::sync::mpsc::UnboundedSender<PowerStatusMessage>,
    pub status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<PowerStatusMessage>>,
    pub inhibitor_label: SharedLabel,
    pub countdown_label: SharedLabel,
    pub scheduled_label: SharedLabel,
}

impl PowerWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: PowerWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel::<PowerStatusMessage>();

        Ok(PowerWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            status_sender,
            status_receiver: Some(status_receiver),
            inhibitor_label: Rc::new(RefCell::new(None)),
            countdown_label: Rc::new(RefCell::new(None)),
            scheduled_label: Rc::new(RefCell::new(None)),
        })
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let inhibitor_label = self.inhibitor_label.clone();
            let countdown_label = self.countdown_label.clone();
            let scheduled_label = self.scheduled_label.clone();
            let show_inhibitors = self.config.show_inhibitor_warnings;
            let show_countdown = self.config.show_countdown_overlay;
            let show_scheduled = self.config.show_scheduled_status;

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    if show_inhibitors {
                        if let Some(ref label) = *inhibitor_label.borrow() {
                            update_inhibitor_warning(label, &status);
                        }
                    }
                    if show_countdown {
                        if let Some(ref label) = *countdown_label.borrow() {
                            update_countdown_overlay(label, &status);
                        }
                    }
                    if show_scheduled {
                        if let Some(ref label) = *scheduled_label.borrow() {
                            update_scheduled_status(label, &status);
                        }
                    }
                }
            });
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PowerStatusMessage>> for PowerWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PowerStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            debug!("Power Widget: failed to forward status to UI thread: {e}");
        }
    }
}

impl AcceptTopic<FfiEnvelope> for PowerWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS
    }
}

impl MessageBroadcaster for PowerWidget {}

impl PluginMetaGetter for PowerWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for PowerWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for PowerWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == PowerStatusMessage::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PowerStatusMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for PowerWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let broadcaster = self.get_broadcaster();

        let main_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.spacing)
            .css_classes(["power-widget".to_string()])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        main_box.set_width_request(config.width);
        main_box.set_height_request(config.height);

        if config.show_inhibitor_warnings {
            let label = Label::builder()
                .css_classes(["power-inhibitor-warning".to_string()])
                .halign(Align::Center)
                .build();
            label.set_visible(false);
            main_box.append(&label);
            *self.inhibitor_label.borrow_mut() = Some(label);
        }

        let buttons_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.spacing)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        let row1 = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(config.spacing)
            .halign(Align::Center)
            .build();

        let row2 = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(config.spacing)
            .halign(Align::Center)
            .build();

        if config.show_shutdown {
            row1.append(&build_power_button(PowerAction::Shutdown, &config, &broadcaster));
        }
        if config.show_reboot {
            row1.append(&build_power_button(PowerAction::Reboot, &config, &broadcaster));
        }
        if config.show_suspend {
            row1.append(&build_power_button(PowerAction::Suspend, &config, &broadcaster));
        }
        if config.show_hibernate {
            row1.append(&build_power_button(PowerAction::Hibernate, &config, &broadcaster));
        }

        if config.show_lock {
            row2.append(&build_power_button(PowerAction::Lock, &config, &broadcaster));
        }
        if config.show_logout {
            row2.append(&build_power_button(PowerAction::Logout, &config, &broadcaster));
        }
        if config.show_reboot_to_firmware {
            row2.append(&build_power_button(PowerAction::RebootToFirmware, &config, &broadcaster));
        }

        buttons_box.append(&row1);
        buttons_box.append(&row2);
        main_box.append(&buttons_box);

        if config.show_countdown_overlay {
            let label = Label::builder()
                .css_classes(["power-countdown".to_string()])
                .halign(Align::Center)
                .valign(Align::Center)
                .build();
            label.set_visible(false);
            main_box.append(&label);
            *self.countdown_label.borrow_mut() = Some(label);
        }

        if config.show_scheduled_status {
            let label = Label::builder().css_classes(["power-scheduled".to_string()]).halign(Align::Center).build();
            label.set_visible(false);
            main_box.append(&label);
            *self.scheduled_label.borrow_mut() = Some(label);
        }

        let click_topic = config.click_topic.clone();
        let click_payload = config.click_payload.clone();
        let click_broadcaster = broadcaster.clone();
        let click_gesture = GestureClick::new();
        click_gesture.connect_released(move |_gesture, _n_press, _, _| {
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                click_broadcaster.broadcast_string(&topic, &payload_str);
            }
        });
        main_box.add_controller(click_gesture);

        self.start_status_listener();

        main_box.upcast::<Widget>()
    }
}

fn build_power_button(action: PowerAction, config: &PowerWidgetConfig, broadcaster: &smearor_swipe_launcher_plugin_api::MessageBroadcasterInner) -> Button {
    let icon = power_action_icon_unicode(&action);
    let button = Button::builder()
        .label(icon)
        .css_classes(["power-button".to_string()])
        .width_request(config.button_size)
        .height_request(config.button_size)
        .build();

    let broadcaster_clone = broadcaster.clone();
    button.connect_clicked(move |_| {
        let command = PowerCommandMessage::execute(action.clone());
        broadcaster_clone.broadcast_message_to_topic(command);
    });

    button
}

fn update_inhibitor_warning(label: &Label, status: &PowerStatusMessage) {
    if status.inhibitors.is_empty() {
        label.set_visible(false);
    } else {
        let descriptions: Vec<String> = status
            .inhibitors
            .iter()
            .map(|inh| format!("{}: {}", inh.who.to_string(), inh.reason.to_string()))
            .collect();
        let text = format!("\u{f0027} {}", descriptions.join(", "));
        label.set_text(&text);
        label.set_visible(true);
    }
}

fn update_countdown_overlay(label: &Label, status: &PowerStatusMessage) {
    if status.countdown_active {
        let icon = power_action_icon_unicode(&status.countdown_action);
        let text = format!("{} in {}...", icon, status.countdown_remaining_seconds);
        label.set_text(&text);
        label.set_visible(true);
    } else {
        label.set_visible(false);
    }
}

fn update_scheduled_status(label: &Label, status: &PowerStatusMessage) {
    match status.scheduled_action.as_ref() {
        Some(sched) => {
            let icon = power_action_icon_unicode(&sched.action);
            let minutes = sched.remaining_seconds / 60;
            let seconds = sched.remaining_seconds % 60;
            let text = format!("{} in {:02}:{:02}", icon, minutes, seconds);
            label.set_text(&text);
            label.set_visible(true);
        }
        None => {
            label.set_visible(false);
        }
    }
}
