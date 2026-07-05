use crate::config::BarOrientation;
use crate::config::DiskDisplayMode;
use crate::config::DisksWidgetConfig;
use crate::config::DisplayMode;
use crate::config::PercentageWidgetConfig;
use crate::shared::build_icon_image;
use crate::shared::build_percentage_widget;
use crate::shared::draw_gauge;
use crate::shared::format_bytes;
use crate::shared::gauge_color;
use crate::shared::update_value_label;
use crate::shared::value_class;
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
use smearor_sysinfo_model::DisksStatusMessage;
use smearor_sysinfo_model::TOPIC_DISKS;
use std::cell::RefCell;
use std::rc::Rc;

pub struct DisksWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: DisksWidgetConfig,
    pub current_value: Rc<RefCell<f32>>,
    pub container: Rc<RefCell<Option<GtkBox>>>,
    pub content_area: Rc<RefCell<Option<GtkBox>>>,
    pub throughput_label: Rc<RefCell<Option<Label>>>,
    pub icon_image: Rc<RefCell<Option<Image>>>,
    pub value_label: Rc<RefCell<Option<Label>>>,
    pub bar: Rc<RefCell<Option<LevelBar>>>,
    pub gauge: Rc<RefCell<Option<DrawingArea>>>,
}

impl DisksWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: DisksWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            current_value: Rc::new(RefCell::new(0.0)),
            container: Rc::new(RefCell::new(None)),
            content_area: Rc::new(RefCell::new(None)),
            throughput_label: Rc::new(RefCell::new(None)),
            icon_image: Rc::new(RefCell::new(None)),
            value_label: Rc::new(RefCell::new(None)),
            bar: Rc::new(RefCell::new(None)),
            gauge: Rc::new(RefCell::new(None)),
        })
    }

    fn first_mount_usage(message: &DisksStatusMessage, config: &DisksWidgetConfig) -> Option<f32> {
        for mount in message.mounts.iter() {
            let mount_point = mount.mount_point.to_string();
            if !config.include_mount_points.is_empty() && !config.include_mount_points.contains(&mount_point) {
                continue;
            }
            if config.display_mode == DiskDisplayMode::RootOnly && mount_point != "/" {
                continue;
            }
            return Some(mount.usage);
        }
        None
    }

    fn update_ui(&self, message: &DisksStatusMessage) {
        if self.config.display_mode == DiskDisplayMode::Gauge {
            self.update_gauge_ui(message);
        } else {
            self.update_list_ui(message);
        }
    }

    fn update_gauge_ui(&self, message: &DisksStatusMessage) {
        let usage = Self::first_mount_usage(message, &self.config).unwrap_or(0.0);
        *self.current_value.borrow_mut() = usage;
        let value_label = self.value_label.clone();
        let bar = self.bar.clone();
        let gauge = self.gauge.clone();
        let css_class = value_class(usage, 80.0, 95.0);

        MainContext::default().spawn_local(async move {
            if let Some(ref label) = *value_label.borrow() {
                update_value_label(label, "{value:.0}%", usage, "");
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
                bar_widget.set_value(usage as f64);
                bar_widget.remove_css_class("sysinfo-normal");
                bar_widget.remove_css_class("sysinfo-warning");
                bar_widget.remove_css_class("sysinfo-critical");
                bar_widget.add_css_class(css_class);
            }
            if let Some(ref gauge_widget) = *gauge.borrow() {
                gauge_widget.queue_draw();
            }
        });
    }

    fn update_list_ui(&self, message: &DisksStatusMessage) {
        let content_area = self.content_area.clone();
        let throughput_label = self.throughput_label.clone();
        let config = self.config.clone();
        let message_inner = message.clone();

        MainContext::default().spawn_local(async move {
            if let Some(ref content) = *content_area.borrow() {
                while let Some(child) = content.first_child() {
                    content.remove(&child);
                }

                let mut count = 0;
                for mount in message_inner.mounts.iter() {
                    if count >= config.max_mount_points {
                        break;
                    }
                    let mount_point = mount.mount_point.to_string();
                    if !config.include_mount_points.is_empty() && !config.include_mount_points.contains(&mount_point) {
                        continue;
                    }
                    if config.display_mode == DiskDisplayMode::RootOnly && mount_point != "/" {
                        continue;
                    }

                    let row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(4).build();
                    let label = Label::builder().label(&format!("{}: {:.0}%", mount_point, mount.usage)).build();
                    row.append(&label);
                    let bar = LevelBar::builder()
                        .min_value(0.0)
                        .max_value(100.0)
                        .value(mount.usage as f64)
                        .width_request(80)
                        .build();
                    row.append(&bar);
                    content.append(&row);
                    count += 1;
                }

                if count == 0 {
                    let placeholder = Label::builder().label("no disk data").build();
                    content.append(&placeholder);
                }

                if config.show_throughput {
                    let throughput = Label::builder()
                        .label(&format!(
                            "R: {}/s | W: {}/s",
                            format_bytes(message_inner.read_bytes_per_second),
                            format_bytes(message_inner.write_bytes_per_second)
                        ))
                        .build();
                    content.append(&throughput);
                }
            }

            if let Some(ref label) = *throughput_label.borrow() {
                label.set_text(&format!(
                    "R: {}/s | W: {}/s",
                    format_bytes(message_inner.read_bytes_per_second),
                    format_bytes(message_inner.write_bytes_per_second)
                ));
            }
        });
    }
}

impl MessageHandler<DisksStatusMessage> for DisksWidget {
    fn handle_message(&self, message: DisksStatusMessage, _sender_id: &str) {
        self.update_ui(&message);
    }
}

impl AcceptTopic<FfiEnvelope> for DisksWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_DISKS
    }
}

impl MessageBroadcaster for DisksWidget {}

impl PluginMetaGetter for DisksWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for DisksWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for DisksWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == DisksStatusMessage::TYPE_ID {
                    MessageHandler::<DisksStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for DisksWidget {
    fn build_widget(&mut self) -> Widget {
        if self.config.display_mode == DiskDisplayMode::Gauge {
            return self.build_gauge_widget();
        }

        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .css_classes(["sysinfo-disks".to_string()])
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

        let mut throughput_label = None;
        if self.config.show_throughput {
            let label = Label::builder().label("R: 0 B/s | W: 0 B/s").build();
            content_area.append(&label);
            throughput_label = Some(label);
        }

        let placeholder = Label::builder().label("waiting for disk data...").build();
        content_area.append(&placeholder);

        *self.container.borrow_mut() = Some(container.clone());
        *self.content_area.borrow_mut() = Some(content_area);
        *self.throughput_label.borrow_mut() = throughput_label;
        *self.icon_image.borrow_mut() = icon_image;

        container.upcast::<Widget>()
    }
}

impl DisksWidget {
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
            value_format: String::from("{value:.0}%"),
            warning_threshold: 80.0,
            critical_threshold: 95.0,
        };

        let percentage_widget = build_percentage_widget(&percentage_config);
        let container = percentage_widget.container;

        if let Some(ref gauge_widget) = percentage_widget.gauge {
            let current_value = self.current_value.clone();
            gauge_widget.set_draw_func(move |_area, context, width, height| {
                let value = *current_value.borrow();
                draw_gauge(context, width, height, value, gauge_color(value, 80.0, 95.0));
            });
        }

        if let Some(ref label) = percentage_widget.value_label {
            label.add_css_class("sysinfo-normal");
        }

        *self.container.borrow_mut() = Some(container.clone());
        *self.bar.borrow_mut() = percentage_widget.bar;
        *self.gauge.borrow_mut() = percentage_widget.gauge.clone();
        *self.value_label.borrow_mut() = percentage_widget.value_label;
        *self.icon_image.borrow_mut() = percentage_widget.icon_image;

        percentage_widget.outer_widget
    }
}
