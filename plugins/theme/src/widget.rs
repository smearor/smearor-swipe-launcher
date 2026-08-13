use crate::config::ThemeWidgetConfig;
use crate::labels::ThemeLabel;
use crate::labels::ThemeLabels;
use crate::personalization::PersonalizationOverride;
use crate::preview::update_preview;
use gtk4::Align;
use gtk4::Image;
use gtk4::Label;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::apply_widget_scaled_css;
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_info_label_scaled;
use smearor_swipe_launcher_plugin_api::build_main_label_scaled;
use smearor_swipe_launcher_plugin_api::build_spacer_scaled;
use smearor_swipe_launcher_plugin_api::sanitize_scale;
use smearor_theme_model::TOPIC_STATUS;
use smearor_theme_model::ThemeCommandMessage;
use smearor_theme_model::ThemeInfo;
use smearor_theme_model::ThemeMode;
use smearor_theme_model::ThemeStatusMessage;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

pub struct ThemeWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: ThemeWidgetConfig,
    pub preview_image: Rc<RefCell<Option<Image>>>,
    pub fallback_image: Rc<RefCell<Option<Image>>>,
    pub value_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<ThemeStatusMessage>>>,
    pub latest_personalization: Rc<RefCell<Option<PersonalizationStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl ThemeWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: ThemeWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget = ThemeWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            preview_image: Rc::new(RefCell::new(None)),
            fallback_image: Rc::new(RefCell::new(None)),
            value_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            latest_personalization: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_initial_status(&self) {
        self.get_broadcaster().broadcast_message_to_topic(ThemeCommandMessage::refresh());
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Broadcast a WidgetUpdateMessage so headless/Web instances re-render this widget.
    pub fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);
    }

    /// Returns the theme info for the currently selected theme index.
    fn current_theme_info(&self) -> Option<ThemeInfo> {
        let status = self.latest_status.borrow();
        status.as_ref().and_then(|s| s.themes.get(s.selected_theme_index as usize).cloned())
    }

    fn update_ui(&self) {
        let preview_image = self.preview_image.clone();
        let fallback_image = self.fallback_image.clone();
        let value_label = self.value_label.clone();
        let info_label = self.info_label.clone();
        let config = self.config.clone();
        let theme_info = self.current_theme_info();
        let status = self.latest_status.borrow().clone();
        let personalization = self.latest_personalization.borrow();
        let labels = ThemeLabels::from_personalization(personalization.as_ref());

        MainContext::default().spawn_local(async move {
            let (theme_name, preview_path, preview_icon, mode_text) = match &theme_info {
                Some(theme) => {
                    let mode_text = match theme.mode {
                        ThemeMode::Dark => &labels.dark,
                        ThemeMode::Light => &labels.light,
                        ThemeMode::System => &labels.system,
                    };
                    (
                        theme.name.to_string(),
                        theme.preview_image_path.to_string(),
                        theme.preview_icon.to_string(),
                        mode_text.clone(),
                    )
                }
                None => (ThemeLabel::NoTheme.localized_label(Locale::default()), String::new(), String::new(), String::new()),
            };

            let is_applied = status
                .as_ref()
                .and_then(|s| s.current_theme.as_ref().map(|ct| ct.to_string() == theme_name))
                .unwrap_or(false);

            update_preview(&preview_image, &fallback_image, &preview_path, &preview_icon, &config.icons.icon_no_theme);

            if let Some(ref label) = *value_label.borrow() {
                label.set_text(&theme_name);
            }
            if let Some(ref label) = *info_label.borrow() {
                let info = if is_applied { format!("{} \u{2713}", mode_text) } else { mode_text };
                label.set_text(&info);
            }
        });
    }

    /// Select the next theme (without applying) — swipe up.
    fn select_next_theme(&self) {
        let status = self.latest_status.borrow().clone();
        if let Some(ref status) = status {
            if status.themes.is_empty() {
                return;
            }
            let next_index = (status.selected_theme_index + 1) % status.themes.len() as u32;
            if let Some(theme) = status.themes.get(next_index as usize) {
                let name: String = theme.name.to_string();
                let command = ThemeCommandMessage::select_theme(&name);
                self.get_broadcaster().broadcast_message_to_topic(command);
            }
        }
    }

    /// Select the previous theme (without applying) — swipe down.
    fn select_prev_theme(&self) {
        let status = self.latest_status.borrow().clone();
        if let Some(ref status) = status {
            if status.themes.is_empty() {
                return;
            }
            let prev_index = if status.selected_theme_index == 0 {
                status.themes.len() as u32 - 1
            } else {
                status.selected_theme_index - 1
            };
            if let Some(theme) = status.themes.get(prev_index as usize) {
                let name: String = theme.name.to_string();
                let command = ThemeCommandMessage::select_theme(&name);
                self.get_broadcaster().broadcast_message_to_topic(command);
            }
        }
    }

    /// Apply the currently selected theme — click.
    fn apply_selected_theme(&self) {
        self.get_broadcaster().broadcast_message_to_topic(ThemeCommandMessage::apply_selected());
    }
}

impl DefaultFallback for ThemeWidget {
    fn default_fallback(&self, kind: &ActionKind, _broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::DoublePress => {
                self.apply_selected_theme();
            }
            ActionKind::SwipeUp | ActionKind::ScrollUp | ActionKind::MiddleClick => {
                self.select_next_theme();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.select_prev_theme();
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                self.apply_selected_theme();
            }
            ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
            ActionKind::Expand => {
                self.select_next_theme();
            }
            ActionKind::Collapse => {
                self.select_prev_theme();
            }
            ActionKind::ToggleView => {
                self.select_next_theme();
            }
        }
    }
}

impl MessageHandler<ThemeStatusMessage> for ThemeWidget {
    fn handle_message(&self, message: ThemeStatusMessage, _sender_id: &str) {
        trace!("theme widget: status update current_theme={:?}", message.current_theme);
        *self.latest_status.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for ThemeWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("theme widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            color_scheme: Some(status.color_scheme.clone()),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        *self.latest_personalization.borrow_mut() = Some(status);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl AcceptTopic<FfiEnvelope> for ThemeWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_PERSONALIZATION_STATUS || topic == TOPIC_MCP_INVOKE_TOOL
    }
}

impl MessageBroadcaster for ThemeWidget {}

impl PluginMetaGetter for ThemeWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for ThemeWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for ThemeWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                trace!("theme widget: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == ThemeStatusMessage::TYPE_ID {
                    MessageHandler::<ThemeStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<InvokeToolMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for ThemeWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let show_labels = !config.icon.icon_only();
        let scale = sanitize_scale(config.dimensions.scale.unwrap_or(1.0));

        let content_box = build_content_box(config.layout.spacing_scaled(scale), &["menu_button_inner"]);

        let preview_image = Image::builder()
            .css_classes(["theme-preview"])
            .halign(Align::Center)
            .valign(Align::Center)
            .visible(false)
            .build();
        content_box.append(&preview_image);

        let fallback_image = Image::builder()
            .css_classes(["theme-icon", "nerd-icon"])
            .halign(Align::Center)
            .valign(Align::Center)
            .pixel_size(config.icon.icon_size_scaled(scale))
            .build();
        if let Some(color) = config.icon.icon_color() {
            apply_icon_color(&fallback_image, color);
        }
        content_box.append(&fallback_image);

        let value_label = build_main_label_scaled(if show_labels { "Loading..." } else { "" }, config.text_colors.main_text_color(), true, Some(12), scale);
        content_box.append(&value_label);

        let info_label = build_info_label_scaled(if show_labels { "" } else { "" }, config.text_colors.info_text_color(), false, None, scale);
        content_box.append(&info_label);

        let spacer = build_spacer_scaled(16, scale);
        content_box.append(&spacer);

        *self.preview_image.borrow_mut() = Some(preview_image);
        *self.fallback_image.borrow_mut() = Some(fallback_image);
        *self.value_label.borrow_mut() = Some(value_label);
        *self.info_label.borrow_mut() = Some(info_label);

        let button = config.dimensions.build_button_scaled(config.mode, &content_box, "max-width-", scale);

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            preview_image: self.preview_image.clone(),
            fallback_image: self.fallback_image.clone(),
            value_label: self.value_label.clone(),
            info_label: self.info_label.clone(),
            latest_status: self.latest_status.clone(),
            latest_personalization: self.latest_personalization.clone(),
            personalization: self.personalization.clone(),
        });

        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        if scale != 1.0 {
            apply_widget_scaled_css(&button_widget, scale);
        }
        let message_broadcaster = self.get_broadcaster();
        widget_self.attach_gesture_handlers(&button_widget, &self.config.actions, &message_broadcaster, &GestureHandlersConfiguration::default());

        button_widget
    }
}

/// Renders the view data for the theme widget based on the current status.
/// Used by graphic and HTML renderers.
pub fn render_view(status: Option<&ThemeStatusMessage>, config: &ThemeWidgetConfig, labels: &ThemeLabels) -> ViewData {
    let theme_info: Option<ThemeInfo> = status.and_then(|s| s.themes.get(s.selected_theme_index as usize).cloned());

    match theme_info {
        Some(theme) => {
            let is_applied = status
                .and_then(|s| s.current_theme.as_ref().map(|ct| ct.to_string() == theme.name.to_string()))
                .unwrap_or(false);

            let icon = if !theme.preview_icon.is_empty() {
                theme.preview_icon.to_string()
            } else {
                config.icons.icon_theme.clone()
            };

            let mode_text = match theme.mode {
                ThemeMode::Dark => &labels.dark,
                ThemeMode::Light => &labels.light,
                ThemeMode::System => &labels.system,
            };

            let info = if is_applied { format!("{} \u{2713}", mode_text) } else { mode_text.clone() };

            ViewData::new(icon, theme.name.to_string(), info)
        }
        None => ViewData::new(config.icons.icon_no_theme.clone(), labels.no_theme.clone(), labels.theme.clone()),
    }
}
