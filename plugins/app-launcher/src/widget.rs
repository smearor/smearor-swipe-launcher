use crate::config::AppLauncherConfig;
use crate::personalization::PersonalizationOverride;
use adw::gdk::pango::EllipsizeMode;
use freedesktop_entry_parser::Entry;
use gtk4::Align;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::WidgetExt;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::DesktopFileStatus;
use smearor_app_launcher_model::DesktopFileStatusMessage;
use smearor_app_launcher_model::TOPIC_STATUS as TOPIC_APP_LAUNCHER_STATUS;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
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
use smearor_swipe_launcher_plugin_api::PluginMetaRaw;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::apply_widget_scaled_css;
use smearor_swipe_launcher_plugin_api::build_spacer_scaled;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use smearor_swipe_launcher_plugin_api::sanitize_scale;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::trace;

pub struct AppLauncherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AppLauncherConfig,
    pub desktop_entry: Entry,
    pub app_name: String,
    pub icon_name: String,
    pub led_indicator: Arc<RwLock<Option<gtk4::Box>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl AppLauncherWidget {
    pub fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!("AppLauncherWidget config: {config:?}");
        let meta_raw = PluginMetaRaw::try_from(&config)?;
        let config = AppLauncherConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let mut app_name = meta_raw.display_name.to_string();
        let mut icon_name = meta_raw.icon_name.unwrap_or_default().to_string();

        // Parse `.desktop` file
        let desktop_entry = match Entry::parse_file(&config.desktop_file_path) {
            Ok(entry) => entry,
            Err(e) => {
                return Err(PluginConstructionErrorWrapper::new(
                    PluginConstructionError::Custom,
                    format!("AppLauncher Service: Failed to parse desktop file {}: {e}", config.desktop_file_path).into(),
                ));
            }
        };
        if let Some(name) = desktop_entry.get("Desktop Entry", "Name").and_then(|names| names.first()) {
            app_name = name.clone();
        }
        if let Some(config_icon) = &config.icon {
            icon_name = config_icon.clone();
        } else {
            match desktop_entry.get("Desktop Entry", "Icon").and_then(|names| names.first()) {
                Some(icon) => icon_name = icon.clone(),
                None => {
                    if icon_name.is_empty() {
                        icon_name = "system-run".to_string();
                    }
                }
            }
        }

        let widget = AppLauncherWidget {
            meta: PluginMeta::new(meta_raw.id, app_name.clone(), Some(icon_name.clone())),
            config,
            desktop_entry,
            app_name,
            icon_name,
            core_context,
            led_indicator: Arc::new(RwLock::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_personalization_status();
        Ok(widget)
    }

    /// Request personalization status from the personalization service.
    /// This is needed because the widget may be created after the initial broadcast.
    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }
}

impl DefaultFallback for AppLauncherWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::DoublePress => {
                broadcaster.broadcast_message_to_topic(DesktopFileCommandMessage::exec(
                    &self.config.desktop_file_path,
                    self.config.wrapper.clone(),
                    self.config.forked,
                    self.config.terminate_on_exit,
                ));
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(DesktopFileCommandMessage::terminate(&self.config.desktop_file_path, self.config.wrapper.clone()));
            }
            ActionKind::SwipeUp
            | ActionKind::ScrollUp
            | ActionKind::SwipeDown
            | ActionKind::ScrollDown
            | ActionKind::MiddleClick
            | ActionKind::Hold
            | ActionKind::CompoundLongpress
            | ActionKind::Init
            | ActionKind::Expand
            | ActionKind::Collapse
            | ActionKind::ToggleView => {}
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<DesktopFileStatusMessage>> for AppLauncherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<DesktopFileStatusMessage>, _sender_id: &str) {
        if message.desktop_file != self.config.desktop_file_path {
            return;
        }
        trace!("AppLauncher Widget {} status updated for {}: {:?}", self.meta.id, message.desktop_file, message.status);
        if let Ok(guard) = self.led_indicator.read() {
            if let Some(led) = guard.as_ref() {
                match message.status {
                    DesktopFileStatus::Running => {
                        led.remove_css_class("led-unlit");
                        led.add_css_class("led-lit");
                    }
                    DesktopFileStatus::Stopped => {
                        led.remove_css_class("led-lit");
                        led.add_css_class("led-unlit");
                    }
                }
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for AppLauncherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("app-launcher widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
    }
}

impl MessageBroadcaster for AppLauncherWidget {}

impl PluginMetaGetter for AppLauncherWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for AppLauncherWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl AcceptTopic<FfiEnvelope> for AppLauncherWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_APP_LAUNCHER_STATUS || topic == TOPIC_PERSONALIZATION_STATUS || topic == TOPIC_MCP_INVOKE_TOOL
    }
}

impl WidgetPlugin for AppLauncherWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<DesktopFileStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<DesktopFileStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for AppLauncherWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let config = self.config.clone();
        let show_labels = !config.icon_config.icon_only();
        let scale = sanitize_scale(config.dimensions.scale.unwrap_or(1.0));

        // Build content box based on widget mode
        let content_box = match config.mode {
            WidgetMode::Compact => {
                // Compact: vertical layout (icon on top, name below)
                let box_ = gtk4::Box::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(config.layout.spacing_scaled(scale))
                    .css_classes(["app-launcher-tile", "menu_button_inner"])
                    .halign(Align::Center)
                    .valign(Align::Center)
                    .vexpand(true)
                    .build();
                box_
            }
            WidgetMode::Wide => {
                // Wide: horizontal layout (icon on left, name on right)
                let box_ = gtk4::Box::builder()
                    .orientation(Orientation::Horizontal)
                    .spacing(config.layout.spacing_scaled(scale))
                    .css_classes(["app-launcher-tile", "menu_button_inner", "app-launcher-wide"])
                    .halign(Align::Center)
                    .valign(Align::Center)
                    .vexpand(true)
                    .build();
                box_
            }
        };

        // Line 0: Icon
        let image = if self.icon_name.starts_with("nf-") {
            if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(&self.icon_name) {
                Image::from_icon_name(&gtk_icon_name)
            } else {
                Image::from_icon_name(&self.icon_name)
            }
        } else {
            Image::from_icon_name(&self.icon_name)
        };
        let scaled_icon_size = config.icon_config.icon_size_scaled(scale);
        image.set_pixel_size(scaled_icon_size);
        image.set_height_request(scaled_icon_size);
        if let Some(color) = config.icon_config.icon_color() {
            apply_icon_color(&image, color);
        }
        content_box.append(&image);

        // For wide mode, wrap text lines in a vertical sub-box
        let text_box = gtk4::Box::builder().orientation(Orientation::Vertical).spacing(0).valign(Align::Center).build();

        // Line 1: App name (main text)
        let name_label = Label::builder()
            .label(&self.app_name)
            .ellipsize(EllipsizeMode::End)
            .max_width_chars(12)
            .css_classes(["app-launcher-label", "widget-main-text"])
            .halign(if config.mode == WidgetMode::Wide { Align::Start } else { Align::Center })
            .build();
        name_label.set_height_request((20.0 * scale).round() as i32);
        apply_text_color(&name_label, config.text_colors.main_text_color());
        if show_labels {
            text_box.append(&name_label);
        }

        // Line 2: Info text (empty for app-launcher, reserved for future use)
        let info_label = Label::builder()
            .label("")
            .css_classes(["widget-info-text"])
            .halign(if config.mode == WidgetMode::Wide { Align::Start } else { Align::Center })
            .build();
        info_label.set_height_request((16.0 * scale).round() as i32);
        apply_text_color(&info_label, config.text_colors.info_text_color());
        if show_labels {
            text_box.append(&info_label);
        }

        // Line 3: Spacer
        let spacer = build_spacer_scaled(16, scale);
        text_box.append(&spacer);

        if config.mode == WidgetMode::Wide {
            content_box.append(&text_box);
        } else {
            // In compact mode, append text_box children directly to content_box
            // to maintain the flat 4-line structure
            content_box.append(&text_box);
        }

        // LED Indicator Box to show if application is running
        let led_box = gtk4::Box::builder()
            .width_request(8)
            .height_request(8)
            .halign(Align::Center)
            .css_classes(["app-launcher-led", "led-unlit"])
            .build();

        *self.led_indicator.write().unwrap() = Some(led_box);

        let button = config
            .dimensions
            .build_button_scaled(config.mode, &content_box, "app-launcher-max-width-", scale);

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            desktop_entry: self.desktop_entry.clone(),
            app_name: self.app_name.clone(),
            icon_name: self.icon_name.clone(),
            led_indicator: Arc::clone(&self.led_indicator),
            personalization: self.personalization.clone(),
        });
        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        if scale != 1.0 {
            apply_widget_scaled_css(&button_widget, scale);
        }
        let message_broadcaster = self.get_broadcaster();
        widget_self.attach_gesture_handlers(
            &button_widget,
            &self.config.actions,
            &message_broadcaster,
            &GestureHandlersConfiguration {
                delay_factor: Some(2.0),
                longpress_css_class: Some("menu-button-longpress".to_string()),
                ..Default::default()
            },
        );

        button_widget
    }
}
