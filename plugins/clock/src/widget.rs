use crate::clock::Clock;
use crate::config::ClockConfig;
use crate::labels::ClockLabels;
use gtk4::Align;
use gtk4::Label;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use serde_json;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS;
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
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::apply_widget_scaled_css;
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_spacer_scaled;
use smearor_swipe_launcher_plugin_api::sanitize_scale;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use tracing::trace;

use crate::clock::PersonalizationOverride;

pub(crate) struct ClockWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ClockConfig,
    pub(crate) clock: Arc<Clock>,
    pub(crate) labels: Arc<RwLock<Option<ClockLabels>>>,
    pub(crate) time_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
}

impl ClockWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let clock_config: ClockConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let widget = ClockWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: clock_config.clone(),
            clock: Arc::new(Clock::new(clock_config)),
            labels: Arc::new(RwLock::new(None)),
            time_receiver: None,
        };
        widget.register_mcp_capabilities();
        Ok(widget)
    }

    fn update_labels(labels: &ClockLabels, clock: &Clock) {
        labels.time_label.set_text(&clock.get_time_string());
        labels.date_label.set_text(&clock.get_date_string());
        labels.weekday_label.set_text(clock.get_weekday_name());
    }

    pub(crate) fn start_time_update(&mut self) {
        let (time_sender, time_receiver) = tokio::sync::mpsc::unbounded_channel::<()>();
        self.time_receiver = Some(time_receiver);

        thread::spawn(move || {
            loop {
                if time_sender.send(()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        if let Some(mut rx) = self.time_receiver.take() {
            let labels = self.labels.clone();
            let clock = self.clock.clone();
            MainContext::default().spawn_local(async move {
                while rx.recv().await.is_some() {
                    if let Ok(guard) = labels.read() {
                        if let Some(ref lbls) = guard.as_ref() {
                            Self::update_labels(lbls, &clock);
                        }
                    }
                }
            });
        }
    }
}

impl DefaultFallback for ClockWidget {
    fn default_fallback(&self, _kind: &ActionKind, _broadcaster: &MessageBroadcasterInner) {}
}

impl MessageHandler<FfiEnvelope> for ClockWidget {
    fn handle_message(&self, _message: FfiEnvelope, _sender_id: &str) {}
}

impl AcceptTopic<FfiEnvelope> for ClockWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_MCP_INVOKE_TOOL || topic == TOPIC_MCP_INVOKE_RESOURCE
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for ClockWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("clock: received personalization status");
        let status = message.0;
        let timezone = status.timezone.as_ref().map(|t| t.to_string());
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            timezone,
            locale,
            time_format: Some(status.time_format),
            date_format: Some(status.date_format),
        };
        self.clock.update_personalization(override_data);
        if let Ok(guard) = self.labels.read()
            && let Some(lbls) = guard.as_ref()
        {
            Self::update_labels(lbls, &self.clock);
        }
    }
}

impl MessageBroadcaster for ClockWidget {}

impl PluginMetaGetter for ClockWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for ClockWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for ClockWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for ClockWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let show_labels = !config.icon_config.icon_only();
        let scale = sanitize_scale(config.dimensions.scale.unwrap_or(1.0));

        let content_box = build_content_box(config.layout.spacing_scaled(scale), &["menu_button_inner"]);

        // Line 0: Time display (replaces icon line)
        let time_label = Label::builder()
            .label("")
            .css_classes(["clock-time"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        time_label.set_height_request(config.icon_config.icon_size_scaled(scale));
        content_box.append(&time_label);

        // Line 1: Date (main text)
        let date_label = Label::builder()
            .label(if show_labels { "" } else { "" })
            .css_classes(["widget-main-text"])
            .halign(Align::Center)
            .build();
        date_label.set_height_request((20.0 * scale).round() as i32);
        apply_text_color(&date_label, config.text_colors.main_text_color());
        content_box.append(&date_label);

        // Line 2: Weekday (info text)
        let weekday_label = Label::builder()
            .label(if show_labels { "" } else { "" })
            .css_classes(["widget-info-text"])
            .halign(Align::Center)
            .build();
        weekday_label.set_height_request((16.0 * scale).round() as i32);
        apply_text_color(&weekday_label, config.text_colors.info_text_color());
        content_box.append(&weekday_label);

        // Line 3: Spacer
        let spacer = build_spacer_scaled(16, scale);
        content_box.append(&spacer);

        let labels = ClockLabels {
            time_label: time_label.clone(),
            date_label: date_label.clone(),
            weekday_label: weekday_label.clone(),
        };

        Self::update_labels(&labels, &self.clock);

        *self.labels.write().unwrap() = Some(labels);

        let button = config.dimensions.build_button_scaled(config.mode, &content_box, "max-width-", scale);

        self.start_time_update();

        let widget_self = std::rc::Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            clock: Arc::clone(&self.clock),
            labels: Arc::clone(&self.labels),
            time_receiver: self.time_receiver.take(),
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
