use crate::labels::NotificationLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_notifications_model::NotificationCommandMessage;
use smearor_notifications_model::NotificationStatusMessage;
use smearor_notifications_model::TOPIC_STATUS;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::AtomicWidgetConfig;
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
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which notifications view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationAtomicView {
    /// Bell icon + notification count.
    Count,
    /// Bell icon + latest notification text (truncated).
    Latest,
    /// Do-not-disturb toggle icon.
    Dnd,
}

impl NotificationAtomicView {
    /// Returns the default nerd font icon name for this view.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Count => "nf-fa-bell",
            Self::Latest => "nf-fa-bell",
            Self::Dnd => "nf-md-bell_off",
        }
    }
}

impl FromStr for NotificationAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "notifications_count" => Ok(Self::Count),
            "notifications_latest" => Ok(Self::Latest),
            "notifications_dnd" => Ok(Self::Dnd),
            _ => Err(format!("Unknown notifications atomic view: {s}")),
        }
    }
}

impl NotificationAtomicView {
    /// Renders this view's display data from the current notifications status and personalization override.
    pub fn render(&self, status: &NotificationStatusMessage, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.effective_locale();
        match self {
            Self::Count => {
                if status.do_not_disturb {
                    ViewData::new(
                        "nf-md-bell_off".to_string(),
                        NotificationLabel::Dnd.localized_label(locale).to_string(),
                        format!("{}", status.unread_count),
                    )
                } else {
                    ViewData::new(
                        "nf-fa-bell".to_string(),
                        format!("{}", status.unread_count),
                        NotificationLabel::Notifications.localized_label(locale).to_string(),
                    )
                }
            }
            Self::Latest => {
                if status.do_not_disturb {
                    ViewData::new("nf-md-bell_off".to_string(), NotificationLabel::Dnd.localized_label(locale).to_string(), String::new())
                } else if let Some(latest) = status.notifications.first() {
                    ViewData::new("nf-fa-bell".to_string(), latest.summary.as_str().to_string(), latest.app_name.as_str().to_string())
                } else {
                    ViewData::new(
                        "nf-fa-bell".to_string(),
                        NotificationLabel::NoNotifications.localized_label(locale).to_string(),
                        String::new(),
                    )
                }
            }
            Self::Dnd => {
                if status.do_not_disturb {
                    ViewData::new(
                        "nf-md-bell_off".to_string(),
                        NotificationLabel::DoNotDisturb.localized_label(locale).to_string(),
                        "ON".to_string(),
                    )
                } else {
                    ViewData::new("nf-fa-bell".to_string(), NotificationLabel::Dnd.localized_label(locale).to_string(), "OFF".to_string())
                }
            }
        }
    }
}

/// Atomic notifications widget that renders a single notifications view.
///
/// Subscribes to `service.notifications.status` and renders only the view specified
/// at construction time. No view switching — each atomic widget is a
/// single-purpose display.
pub struct NotificationAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: NotificationAtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<NotificationStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl NotificationAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = NotificationAtomicView::from_str(widget_name).unwrap_or(NotificationAtomicView::Count);

        let widget = NotificationAtomicWidget {
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

    fn update_ui(&self, status: &NotificationStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f0f3}');
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
    }

    /// Extract graphic rendering data from the latest status.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            return AtomicGraphicData::error('\u{f0f3}', "Loading...".to_string());
        };

        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f0f3}');
        AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text)
    }
}

atomic_widget_impl! {
    widget: NotificationAtomicWidget,
    status: NotificationStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "notifications-atomic",
    mcp_description: "Notifications atomic widget",
    css_prefix: "notifications",
    default_icon: '\u{f0f3}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: NotificationCommandMessage::refresh(),
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for NotificationAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("notifications atomic widget: received personalization status");
        let status = message.0;
        let locale = status
            .locale
            .as_ref()
            .map(|l| Locale::from_str(l.as_str()).unwrap_or_default())
            .unwrap_or_default();
        let override_data = PersonalizationOverride {
            time_format: status.time_format,
            date_format: status.date_format,
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}
