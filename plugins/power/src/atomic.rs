use crate::labels::PowerLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerCommandMessage;
use smearor_power_model::PowerStatusMessage;
use smearor_power_model::TOPIC_STATUS;
use smearor_power_model::power_action_icon;
use smearor_power_model::power_action_icon_unicode;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which power action an atomic widget represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicView {
    /// Standby (mapped to Suspend).
    Standby,
    /// Hibernate the system.
    Hibernate,
    /// Lock the screen.
    Lock,
    /// Reboot the system.
    Reboot,
    /// Shut down the system.
    Shutdown,
    /// Log out of the session.
    Logout,
    /// Suspend the system.
    Suspend,
}

impl FromStr for AtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "power_standby" => Ok(Self::Standby),
            "power_hibernate" => Ok(Self::Hibernate),
            "power_lock" => Ok(Self::Lock),
            "power_reboot" => Ok(Self::Reboot),
            "power_shutdown" => Ok(Self::Shutdown),
            "power_logout" => Ok(Self::Logout),
            "power_suspend" => Ok(Self::Suspend),
            _ => Err(format!("Unknown power atomic view: {s}")),
        }
    }
}

impl AtomicView {
    /// Returns the `PowerAction` this atomic view triggers.
    pub fn to_power_action(&self) -> PowerAction {
        match self {
            Self::Standby => PowerAction::Suspend,
            Self::Hibernate => PowerAction::Hibernate,
            Self::Lock => PowerAction::Lock,
            Self::Reboot => PowerAction::Reboot,
            Self::Shutdown => PowerAction::Shutdown,
            Self::Logout => PowerAction::Logout,
            Self::Suspend => PowerAction::Suspend,
        }
    }

    /// Returns the label key for this view.
    fn label(&self) -> PowerLabel {
        match self {
            Self::Standby => PowerLabel::Standby,
            Self::Hibernate => PowerLabel::Hibernate,
            Self::Lock => PowerLabel::Lock,
            Self::Reboot => PowerLabel::Reboot,
            Self::Shutdown => PowerLabel::Shutdown,
            Self::Logout => PowerLabel::Logout,
            Self::Suspend => PowerLabel::Suspend,
        }
    }

    /// Renders this view's display data.
    pub fn render(&self, status: &PowerStatusMessage, override_data: &PersonalizationOverride) -> ViewData {
        let action = self.to_power_action();
        let icon = power_action_icon(&action).to_string();
        let label = self.label().localized_label(override_data.locale);
        let main_text = label.clone();
        let info_text = if status.countdown_active && status.countdown_action == action {
            let countdown_label = PowerLabel::countdown_label(status.countdown_action, override_data.locale);
            PowerLabel::format_with_seconds(&countdown_label, status.countdown_remaining_seconds)
        } else if let Some(sched) = status.scheduled_action.as_ref() {
            if sched.action == action {
                override_data.format_countdown(sched.remaining_seconds)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        ViewData::new(icon, main_text, info_text)
    }
}

/// Atomic power widget that renders a single power action.
///
/// Subscribes to `service.power.status` and renders only the action specified
/// at construction time. Click triggers the action, longpress triggers cancel.
pub struct PowerAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: AtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<PowerStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl PowerAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();
        let view = AtomicView::from_str(widget_name).unwrap_or(AtomicView::Shutdown);

        let widget = PowerAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn update_ui(&self, status: &PowerStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data);
        let action = self.view.to_power_action();
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or(power_action_icon_unicode(&action).chars().next().unwrap_or('\u{f0425}'));
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, self.config.text_colors.main_text_color());
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, self.config.text_colors.info_text_color());
        }
    }

    /// Extract graphic rendering data from the latest status.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let action = self.view.to_power_action();
        let Some(status) = status.as_ref() else {
            let icon_char =
                resolve_icon_codepoint(power_action_icon(&action)).unwrap_or(power_action_icon_unicode(&action).chars().next().unwrap_or('\u{f0425}'));
            return AtomicGraphicData::new(icon_char, self.view.label().localized_label(Locale::default()), "Loading...".to_string());
        };

        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or(power_action_icon_unicode(&action).chars().next().unwrap_or('\u{f0425}'));
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.main_text_color = self.config.text_colors.main_text_color().map(|c| c.to_rgba());
        data.info_text_color = self.config.text_colors.info_text_color().map(|c| c.to_rgba());
        data
    }
}

atomic_widget_impl! {
    widget: PowerAtomicWidget,
    status: PowerStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "power-atomic",
    mcp_description: "Power atomic widget",
    css_prefix: "power",
    default_icon: '\u{f0425}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: PowerCommandMessage::refresh(),
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for PowerAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("power atomic widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            time_format: Some(status.time_format),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}
