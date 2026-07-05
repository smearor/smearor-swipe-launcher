use crate::config::MemoryWidgetConfig;
use crate::shared::build_percentage_widget;
use crate::shared::draw_gauge;
use crate::shared::format_bytes;
use crate::shared::gauge_color;
use crate::shared::update_value_label;
use crate::shared::value_class;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::DrawingAreaExtManual;
use gtk4::prelude::WidgetExt;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
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
use smearor_sysinfo_model::MemoryStatusMessage;
use smearor_sysinfo_model::TOPIC_MEMORY;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MemoryWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: MemoryWidgetConfig,
    pub container: Rc<RefCell<Option<GtkBox>>>,
    pub bar: Rc<RefCell<Option<LevelBar>>>,
    pub gauge: Rc<RefCell<Option<DrawingArea>>>,
    pub value_label: Rc<RefCell<Option<Label>>>,
    pub details_label: Rc<RefCell<Option<Label>>>,
    pub current_value: Rc<RefCell<f32>>,
}

impl MemoryWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: MemoryWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            container: Rc::new(RefCell::new(None)),
            bar: Rc::new(RefCell::new(None)),
            gauge: Rc::new(RefCell::new(None)),
            value_label: Rc::new(RefCell::new(None)),
            details_label: Rc::new(RefCell::new(None)),
            current_value: Rc::new(RefCell::new(0.0)),
        })
    }

    fn update_ui(&self, message: &MemoryStatusMessage) {
        let memory_usage = message.memory_usage.clamp(0.0, 100.0);
        *self.current_value.borrow_mut() = memory_usage;
        let value_label = self.value_label.clone();
        let bar = self.bar.clone();
        let gauge = self.gauge.clone();
        let details_label = self.details_label.clone();
        let config = self.config.clone();
        let css_class = value_class(memory_usage, config.percentage.warning_threshold, config.percentage.critical_threshold);

        let message_inner = message.clone();
        MainContext::default().spawn_local(async move {
            if let Some(ref label) = *value_label.borrow() {
                update_value_label(label, &config.percentage.value_format, memory_usage, "memory_usage");
                let classes = label.css_classes();
                let classes: Vec<String> = classes
                    .iter()
                    .map(|c| c.to_string())
                    .filter(|c| c != "sysinfo-value" && c != "sysinfo-normal" && c != "sysinfo-warning" && c != "sysinfo-critical")
                    .collect();
                let mut new_classes = classes;
                new_classes.push(css_class.to_string());
                label.set_css_classes(&new_classes.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            }
            if let Some(ref bar_widget) = *bar.borrow() {
                bar_widget.set_value(memory_usage as f64);
                bar_widget.remove_css_class("sysinfo-normal");
                bar_widget.remove_css_class("sysinfo-warning");
                bar_widget.remove_css_class("sysinfo-critical");
                bar_widget.add_css_class(css_class);
            }
            if let Some(ref gauge_widget) = *gauge.borrow() {
                gauge_widget.queue_draw();
            }
            if let Some(ref details) = *details_label.borrow() {
                let mut parts = Vec::new();
                if config.show_used_bytes {
                    parts.push(format!("used: {}", format_bytes(message_inner.memory_used)));
                }
                if config.show_available_bytes {
                    parts.push(format!("available: {}", format_bytes(message_inner.memory_available)));
                }
                details.set_text(&parts.join(" | "));
            }
        });
    }
}

impl MessageHandler<MemoryStatusMessage> for MemoryWidget {
    fn handle_message(&self, message: MemoryStatusMessage, _sender_id: &str) {
        self.update_ui(&message);
    }
}

impl AcceptTopic<FfiEnvelope> for MemoryWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_MEMORY
    }
}

impl MessageBroadcaster for MemoryWidget {}

impl PluginMetaGetter for MemoryWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for MemoryWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for MemoryWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == MemoryStatusMessage::TYPE_ID {
                    MessageHandler::<MemoryStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for MemoryWidget {
    fn build_widget(&mut self) -> Widget {
        let percentage_widget = build_percentage_widget(&self.config.percentage);
        let container = percentage_widget.container;

        let mut details_label = None;
        if self.config.show_used_bytes || self.config.show_available_bytes {
            let label = Label::builder().css_classes(["sysinfo-details".to_string()]).build();
            label.set_halign(Align::Center);
            container.append(&label);
            details_label = Some(label);
        }

        if let Some(ref gauge_widget) = percentage_widget.gauge {
            let current_value = self.current_value.clone();
            let warning = self.config.percentage.warning_threshold;
            let critical = self.config.percentage.critical_threshold;
            gauge_widget.set_draw_func(move |_area, context, width, height| {
                let value = *current_value.borrow();
                draw_gauge(context, width, height, value, gauge_color(value, warning, critical));
            });
        }

        if let Some(ref label) = percentage_widget.value_label {
            label.add_css_class("sysinfo-normal");
        }

        *self.container.borrow_mut() = Some(container.clone());
        *self.bar.borrow_mut() = percentage_widget.bar;
        *self.gauge.borrow_mut() = percentage_widget.gauge.clone();
        *self.value_label.borrow_mut() = percentage_widget.value_label;
        *self.details_label.borrow_mut() = details_label;

        percentage_widget.outer_widget
    }
}
