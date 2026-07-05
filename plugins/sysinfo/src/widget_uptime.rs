use crate::config::BarOrientation;
use crate::config::DisplayMode;
use crate::config::PercentageWidgetConfig;
use crate::config::UptimeDisplayMode;
use crate::config::UptimeWidgetConfig;
use crate::shared::build_icon_image;
use crate::shared::build_percentage_widget;
use crate::shared::draw_gauge;
use crate::shared::format_duration;
use crate::shared::format_duration_with_format;
use crate::shared::gauge_color;
use glib::object::Cast;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
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
use smearor_sysinfo_model::TOPIC_UPTIME;
use smearor_sysinfo_model::UptimeStatusMessage;
use std::cell::RefCell;
use std::rc::Rc;

pub struct UptimeWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: UptimeWidgetConfig,
    pub current_value: Rc<RefCell<f32>>,
    pub container: Rc<RefCell<Option<GtkBox>>>,
    pub uptime_label: Rc<RefCell<Option<Label>>>,
    pub load_label: Rc<RefCell<Option<Label>>>,
    pub icon_image: Rc<RefCell<Option<Image>>>,
    pub bar: Rc<RefCell<Option<LevelBar>>>,
    pub gauge: Rc<RefCell<Option<DrawingArea>>>,
}

impl UptimeWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: UptimeWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            current_value: Rc::new(RefCell::new(0.0)),
            container: Rc::new(RefCell::new(None)),
            uptime_label: Rc::new(RefCell::new(None)),
            load_label: Rc::new(RefCell::new(None)),
            icon_image: Rc::new(RefCell::new(None)),
            bar: Rc::new(RefCell::new(None)),
            gauge: Rc::new(RefCell::new(None)),
        })
    }

    fn update_ui(&self, message: &UptimeStatusMessage) {
        if self.config.display_mode == UptimeDisplayMode::Gauge {
            self.update_gauge_ui(message);
        } else {
            self.update_info_ui(message);
        }
    }

    fn update_gauge_ui(&self, message: &UptimeStatusMessage) {
        let uptime_seconds = message.uptime_seconds;
        let value = ((uptime_seconds % 86400) as f32 / 86400.0) * 100.0;
        *self.current_value.borrow_mut() = value;

        let uptime_label = self.uptime_label.clone();
        let gauge = self.gauge.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            if let Some(ref label) = *uptime_label.borrow() {
                label.set_text(&format_duration_with_format(uptime_seconds, &config.value_format));
            }
            if let Some(ref gauge_widget) = *gauge.borrow() {
                gauge_widget.queue_draw();
            }
        });
    }

    fn update_info_ui(&self, message: &UptimeStatusMessage) {
        let uptime_label = self.uptime_label.clone();
        let load_label = self.load_label.clone();
        let config = self.config.clone();

        let message_inner = message.clone();
        MainContext::default().spawn_local(async move {
            if let Some(ref label) = *uptime_label.borrow() {
                if config.show_uptime {
                    label.set_text(&format_duration(message_inner.uptime_seconds));
                } else {
                    label.set_text("");
                }
            }
            if let Some(ref label) = *load_label.borrow() {
                let mut parts = Vec::new();
                if config.show_load_average_1_minute {
                    parts.push(format!("1m: {:.2}", message_inner.load_average_1_minute));
                }
                if config.show_load_average_5_minute {
                    parts.push(format!("5m: {:.2}", message_inner.load_average_5_minute));
                }
                if config.show_load_average_15_minute {
                    parts.push(format!("15m: {:.2}", message_inner.load_average_15_minute));
                }
                label.set_text(&parts.join(" | "));
            }
        });
    }
}

impl MessageHandler<UptimeStatusMessage> for UptimeWidget {
    fn handle_message(&self, message: UptimeStatusMessage, _sender_id: &str) {
        self.update_ui(&message);
    }
}

impl AcceptTopic<FfiEnvelope> for UptimeWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_UPTIME
    }
}

impl MessageBroadcaster for UptimeWidget {}

impl PluginMetaGetter for UptimeWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for UptimeWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for UptimeWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == UptimeStatusMessage::TYPE_ID {
                    MessageHandler::<UptimeStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for UptimeWidget {
    fn build_widget(&mut self) -> Widget {
        if self.config.display_mode == UptimeDisplayMode::Gauge {
            return self.build_gauge_widget();
        }

        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .css_classes(["sysinfo-uptime".to_string()])
            .build();

        let mut icon_image = None;
        if self.config.show_icon {
            if let Some(ref icon) = self.config.icon {
                let image = build_icon_image(icon, self.config.icon_size);
                image.add_css_class("sysinfo-icon");
                container.append(&image);
                icon_image = Some(image);
            }
        }

        let content_area = GtkBox::builder().orientation(Orientation::Vertical).spacing(4).build();
        container.append(&content_area);

        let mut uptime_label = None;
        let mut load_label = None;

        if self.config.show_uptime {
            let label = Label::builder().label("00h 00m 00s").build();
            content_area.append(&label);
            uptime_label = Some(label);
        }

        let load_parts = [
            self.config.show_load_average_1_minute,
            self.config.show_load_average_5_minute,
            self.config.show_load_average_15_minute,
        ];
        if load_parts.iter().any(|enabled| *enabled) {
            let label = Label::builder().label("1m: 0.00 | 5m: 0.00 | 15m: 0.00").build();
            content_area.append(&label);
            load_label = Some(label);
        }

        *self.container.borrow_mut() = Some(container.clone());
        *self.uptime_label.borrow_mut() = uptime_label;
        *self.load_label.borrow_mut() = load_label;
        *self.icon_image.borrow_mut() = icon_image;

        container.upcast::<Widget>()
    }
}

impl UptimeWidget {
    fn build_gauge_widget(&mut self) -> Widget {
        let percentage_config = PercentageWidgetConfig {
            display_mode: DisplayMode::Gauge,
            bar_orientation: BarOrientation::Horizontal,
            show_value: true,
            show_icon: self.config.show_icon,
            width: 120,
            height: 40,
            icon: self.config.icon.clone(),
            icon_size: self.config.icon_size,
            value_format: String::from("{value}"),
            warning_threshold: 70.0,
            critical_threshold: 90.0,
        };

        let percentage_widget = build_percentage_widget(&percentage_config);
        let container = percentage_widget.container;

        if let Some(ref gauge_widget) = percentage_widget.gauge {
            let current_value = self.current_value.clone();
            gauge_widget.set_draw_func(move |_area, context, width, height| {
                let value = *current_value.borrow();
                draw_gauge(context, width, height, value, gauge_color(value, 70.0, 90.0));
            });
        }

        if let Some(ref label) = percentage_widget.value_label {
            label.add_css_class("sysinfo-normal");
        }

        *self.container.borrow_mut() = Some(container.clone());
        *self.bar.borrow_mut() = percentage_widget.bar;
        *self.gauge.borrow_mut() = percentage_widget.gauge.clone();
        *self.uptime_label.borrow_mut() = percentage_widget.value_label;
        *self.icon_image.borrow_mut() = percentage_widget.icon_image;

        percentage_widget.outer_widget
    }
}
