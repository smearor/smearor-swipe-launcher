use crate::config::PowerWidgetConfig;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::GestureLongPress;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
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
type SharedButton = Rc<RefCell<Option<Button>>>;
type SharedBox = Rc<RefCell<Option<GtkBox>>>;

pub struct PowerWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PowerWidgetConfig,
    pub status_sender: tokio::sync::mpsc::UnboundedSender<PowerStatusMessage>,
    pub status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<PowerStatusMessage>>,
    pub inhibitor_label: SharedLabel,
    pub countdown_label: SharedLabel,
    pub scheduled_label: SharedLabel,
    pub action_button: SharedButton,
    pub button_inner: SharedBox,
    pub current_view: Rc<RefCell<usize>>,
    pub enabled_actions: Rc<RefCell<Vec<PowerAction>>>,
    pub last_status: Rc<RefCell<Option<PowerStatusMessage>>>,
}

impl PowerWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: PowerWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel::<PowerStatusMessage>();

        let enabled_actions = build_enabled_actions(&widget_config);

        Ok(PowerWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            status_sender,
            status_receiver: Some(status_receiver),
            inhibitor_label: Rc::new(RefCell::new(None)),
            countdown_label: Rc::new(RefCell::new(None)),
            scheduled_label: Rc::new(RefCell::new(None)),
            action_button: Rc::new(RefCell::new(None)),
            button_inner: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            enabled_actions: Rc::new(RefCell::new(enabled_actions)),
            last_status: Rc::new(RefCell::new(None)),
        })
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let inhibitor_label = self.inhibitor_label.clone();
            let countdown_label = self.countdown_label.clone();
            let scheduled_label = self.scheduled_label.clone();
            let button_inner = self.button_inner.clone();
            let show_inhibitors = self.config.show_inhibitor_warnings;
            let show_countdown = self.config.show_countdown_overlay;
            let show_scheduled = self.config.show_scheduled_status;
            let current_view = self.current_view.clone();
            let enabled_actions = self.enabled_actions.clone();
            let last_status = self.last_status.clone();

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    *last_status.borrow_mut() = Some(status.clone());
                    if show_inhibitors {
                        if let Some(ref label) = *inhibitor_label.borrow() {
                            if let Some(ref button_inner) = *button_inner.borrow() {
                                let action = enabled_actions.borrow().get(*current_view.borrow()).cloned();
                                update_inhibitor_warning(label, button_inner, &status, action.as_ref());
                            }
                        }
                    }
                    if show_countdown {
                        if let Some(ref label) = *countdown_label.borrow() {
                            if let Some(ref button_inner) = *button_inner.borrow() {
                                update_countdown_overlay(label, button_inner, &status);
                            }
                        }
                    }
                    if show_scheduled {
                        if let Some(ref label) = *scheduled_label.borrow() {
                            if let Some(ref button_inner) = *button_inner.borrow() {
                                update_scheduled_status(label, button_inner, &status);
                            }
                        }
                    }
                }
            });
        }
    }

    fn next_view(&self) {
        self.cycle_view(1);
    }

    fn prev_view(&self) {
        self.cycle_view(-1);
    }

    fn cycle_view(&self, direction: i32) {
        let actions = self.enabled_actions.borrow().clone();
        if actions.len() <= 1 {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        let len = actions.len() as i32;
        *idx = ((*idx as i32 + direction + len) as usize) % len as usize;
        let action = actions[*idx].clone();
        drop(idx);

        let icon = power_action_icon_unicode(&action);
        if let Some(ref button) = *self.action_button.borrow() {
            if let Some(child) = button.child() {
                if let Some(box_widget) = child.dynamic_cast_ref::<gtk4::Box>() {
                    let first = box_widget.first_child();
                    if let Some(w) = first {
                        if let Some(label) = w.dynamic_cast_ref::<gtk4::Label>() {
                            label.set_text(icon);
                        }
                    }
                }
            }
        }

        if let Some(ref label) = *self.inhibitor_label.borrow() {
            if let Some(ref status) = *self.last_status.borrow() {
                if let Some(ref button_inner) = *self.button_inner.borrow() {
                    update_inhibitor_warning(label, button_inner, status, Some(&action));
                }
            }
        }
    }
}

fn build_enabled_actions(config: &PowerWidgetConfig) -> Vec<PowerAction> {
    let mut actions = Vec::new();
    if config.show_shutdown {
        actions.push(PowerAction::Shutdown);
    }
    if config.show_reboot {
        actions.push(PowerAction::Reboot);
    }
    if config.show_suspend {
        actions.push(PowerAction::Suspend);
    }
    if config.show_hibernate {
        actions.push(PowerAction::Hibernate);
    }
    if config.show_lock {
        actions.push(PowerAction::Lock);
    }
    if config.show_logout {
        actions.push(PowerAction::Logout);
    }
    if config.show_reboot_to_firmware {
        actions.push(PowerAction::RebootToFirmware);
    }
    actions
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

        let actions = self.enabled_actions.borrow().clone();
        if let Some(ref default_action) = config.default_action {
            let target = PowerAction::from_str(default_action);
            if let Some(idx) = actions.iter().position(|a| *a == target) {
                *self.current_view.borrow_mut() = idx;
            }
        }
        let current_action = actions.get(*self.current_view.borrow()).cloned().unwrap_or(PowerAction::Shutdown);
        let icon = power_action_icon_unicode(&current_action);

        let button_inner = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.spacing)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["menu_button_inner"])
            .build();

        let action_label = Label::builder().css_classes(["nerd-icon"]).label(icon).build();
        button_inner.append(&action_label);

        *self.button_inner.borrow_mut() = Some(button_inner.clone());

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(config.width)
            .child(&button_inner)
            .build();

        let broadcaster_clone = broadcaster.clone();
        let enabled_actions = self.enabled_actions.clone();
        let current_view = self.current_view.clone();
        button.connect_clicked(move |_| {
            let actions = enabled_actions.borrow();
            let idx = *current_view.borrow();
            if let Some(action) = actions.get(idx) {
                let command = PowerCommandMessage::execute(action.clone());
                broadcaster_clone.broadcast_message_to_topic(command);
            }
        });

        *self.action_button.borrow_mut() = Some(button.clone());

        if config.show_inhibitor_warnings {
            let label = Label::builder().css_classes(["power-inhibitor-warning"]).halign(Align::Center).build();
            label.set_visible(false);
            *self.inhibitor_label.borrow_mut() = Some(label);
        }

        if config.show_countdown_overlay {
            let label = Label::builder()
                .css_classes(["power-countdown"])
                .halign(Align::Center)
                .valign(Align::Center)
                .build();
            label.set_visible(false);
            *self.countdown_label.borrow_mut() = Some(label);
        }

        if config.show_scheduled_status {
            let label = Label::builder().css_classes(["power-scheduled"]).halign(Align::Center).build();
            label.set_visible(false);
            *self.scheduled_label.borrow_mut() = Some(label);
        }

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: None,
            inhibitor_label: self.inhibitor_label.clone(),
            countdown_label: self.countdown_label.clone(),
            scheduled_label: self.scheduled_label.clone(),
            action_button: self.action_button.clone(),
            button_inner: self.button_inner.clone(),
            current_view: self.current_view.clone(),
            enabled_actions: self.enabled_actions.clone(),
            last_status: self.last_status.clone(),
        });

        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let widget_for_drag = widget_self.clone();
        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
            const SWIPE_THRESHOLD: f64 = 50.0;
            if offset_y.abs() > offset_x.abs() && offset_y.abs() > SWIPE_THRESHOLD {
                gesture.set_state(EventSequenceState::Claimed);
                if offset_y < 0.0 {
                    widget_for_drag.next_view();
                } else {
                    widget_for_drag.prev_view();
                }
            }
        });
        button.add_controller(drag_gesture);

        let click_topic = config.click_topic.clone();
        let click_payload = config.click_payload.clone();
        let click_instance = config.click_instance.clone();
        let click_broadcaster = broadcaster.clone();
        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Bubble).build();
        click_gesture.connect_released(move |gesture, _n_press, _x, _y| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = click_instance.clone() {
                    click_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    click_broadcaster.broadcast_string(&topic, &payload_str);
                }
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        button.add_controller(click_gesture);

        let longpress_topic = config.longpress_topic.clone();
        let longpress_payload = config.longpress_payload.clone();
        let longpress_instance = config.longpress_instance.clone();
        let longpress_broadcaster = broadcaster.clone();
        let button_weak = button.downgrade();
        let longpress_gesture = GestureLongPress::new();
        longpress_gesture.connect_pressed(move |gesture, _, _| {
            if let Some(btn) = button_weak.upgrade() {
                btn.add_css_class("longpress-active");
            }
            if let (Some(topic), Some(payload)) = (longpress_topic.clone(), longpress_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = longpress_instance.clone() {
                    longpress_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    longpress_broadcaster.broadcast_string(&topic, &payload_str);
                }
                gesture.set_state(EventSequenceState::Claimed);
            }
        });
        let button_weak = button.downgrade();
        longpress_gesture.connect_cancelled(move |_gesture| {
            if let Some(btn) = button_weak.upgrade() {
                btn.remove_css_class("longpress-active");
            }
        });
        button.add_controller(longpress_gesture);

        self.start_status_listener();

        button.upcast::<Widget>()
    }
}

fn update_inhibitor_warning(label: &Label, button_inner: &GtkBox, status: &PowerStatusMessage, current_action: Option<&PowerAction>) {
    let what_filter = current_action.map(action_to_inhibitor_what).unwrap_or("");
    let relevant: Vec<String> = status
        .inhibitors
        .iter()
        .filter(|inh| what_filter.is_empty() || inh.what.to_lowercase().contains(what_filter))
        .map(|inh| format!("{}: {}", inh.who.to_string(), inh.reason.to_string()))
        .collect();
    if relevant.is_empty() {
        set_label_visibility(label, button_inner, false);
    } else {
        let text = format!("\u{f0027} {}", relevant.join(", "));
        let truncated = if text.chars().count() > 40 {
            let truncated: String = text.chars().take(37).collect();
            format!("{truncated}...")
        } else {
            text
        };
        label.set_text(&truncated);
        set_label_visibility(label, button_inner, true);
    }
}

fn action_to_inhibitor_what(action: &PowerAction) -> &'static str {
    match action {
        PowerAction::Shutdown => "shutdown",
        PowerAction::Reboot | PowerAction::RebootToFirmware => "reboot",
        PowerAction::Suspend | PowerAction::Hibernate => "sleep",
        PowerAction::Lock | PowerAction::Logout => "",
        PowerAction::Cancel => "",
    }
}

fn set_label_visibility(label: &Label, button_inner: &GtkBox, visible: bool) {
    if visible {
        if !label.parent().is_some() {
            button_inner.append(label);
        }
        label.set_visible(true);
    } else {
        label.set_visible(false);
        if let Some(parent) = label.parent() {
            if parent.downcast_ref::<GtkBox>().is_some() {
                button_inner.remove(label);
            }
        }
    }
}

fn update_countdown_overlay(label: &Label, button_inner: &GtkBox, status: &PowerStatusMessage) {
    if status.countdown_active {
        let icon = power_action_icon_unicode(&status.countdown_action);
        let text = format!("{} in {}...", icon, status.countdown_remaining_seconds);
        label.set_text(&text);
        set_label_visibility(label, button_inner, true);
    } else {
        set_label_visibility(label, button_inner, false);
    }
}

fn update_scheduled_status(label: &Label, button_inner: &GtkBox, status: &PowerStatusMessage) {
    match status.scheduled_action.as_ref() {
        Some(sched) => {
            let icon = power_action_icon_unicode(&sched.action);
            let minutes = sched.remaining_seconds / 60;
            let seconds = sched.remaining_seconds % 60;
            let text = format!("{} in {:02}:{:02}", icon, minutes, seconds);
            label.set_text(&text);
            set_label_visibility(label, button_inner, true);
        }
        None => {
            set_label_visibility(label, button_inner, false);
        }
    }
}
