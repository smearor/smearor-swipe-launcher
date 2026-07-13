use crate::config::TemperatureWidgetConfig;
use crate::shared::build_icon_image;
use crate::shared::draw_temperature_gauge;
use crate::shared::value_class;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::glib::prelude::Cast;
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
use smearor_sysinfo_model::CpuStatusMessage;
use smearor_sysinfo_model::TOPIC_CPU;
use smearor_sysinfo_model::TemperatureComponent;
use std::cell::RefCell;
use std::rc::Rc;

/// Per-component gauge data used by the draw function.
struct ComponentGaugeData {
    current: f32,
    max: Option<f32>,
    critical: Option<f32>,
}

/// One gauge entry: overlay widget + drawing area + label + value label + data.
struct GaugeEntry {
    overlay: Widget,
    drawing_area: DrawingArea,
    label: Option<Label>,
    value_label: Label,
    data: Rc<RefCell<ComponentGaugeData>>,
}

pub struct TemperatureWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: TemperatureWidgetConfig,
    pub container: Rc<RefCell<Option<GtkBox>>>,
    pub entries: Rc<RefCell<Vec<GaugeEntry>>>,
}

impl TemperatureWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: TemperatureWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            container: Rc::new(RefCell::new(None)),
            entries: Rc::new(RefCell::new(Vec::new())),
        })
    }

    fn filter_components(&self, components: &[TemperatureComponent]) -> Vec<TemperatureComponent> {
        if self.config.components.is_empty() {
            return components.iter().filter(|c| c.temperature.is_some()).cloned().collect();
        }

        self.config
            .components
            .iter()
            .filter_map(|filter| {
                let filter_lower = filter.to_lowercase();
                components.iter().find(|c| {
                    (c.label.to_string().to_lowercase().contains(&filter_lower) || c.id.to_string().to_lowercase().contains(&filter_lower))
                        && c.temperature.is_some()
                })
            })
            .cloned()
            .collect()
    }

    fn update_ui(&self, message: &CpuStatusMessage) {
        let filtered = self.filter_components(&message.temperature_components);
        let container = self.container.clone();
        let entries = self.entries.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            let container_ref = match container.borrow().clone() {
                Some(ref c) => c.clone(),
                None => return,
            };

            let prev_count = entries.borrow().len();
            if filtered.len() != prev_count {
                while container_ref.first_child().is_some() {
                    if let Some(child) = container_ref.first_child() {
                        container_ref.remove(&child);
                    }
                }
                entries.borrow_mut().clear();

                for comp in &filtered {
                    let entry = build_gauge_entry(comp, &config);
                    container_ref.append(&entry.overlay);
                    entries.borrow_mut().push(entry);
                }
            }

            for (entry, comp) in entries.borrow().iter().zip(filtered.iter()) {
                let temperature: Option<f32> = comp.temperature.as_ref().copied().into();
                let temperature = temperature.unwrap_or(0.0);
                let max_temp: Option<f32> = comp.max_temperature.as_ref().copied().into();
                let critical_temp: Option<f32> = comp.critical_temperature.as_ref().copied().into();

                *entry.data.borrow_mut() = ComponentGaugeData {
                    current: temperature,
                    max: max_temp,
                    critical: critical_temp,
                };

                entry.drawing_area.queue_draw();

                let text = format_temperature(&config.format, temperature);
                entry.value_label.set_text(&text);

                let scale_max = critical_temp.unwrap_or(100.0).max(1.0);
                let ratio = (temperature / scale_max).clamp(0.0, 1.0);
                let css_class = value_class(ratio * 100.0, 70.0, 90.0);

                let classes = entry.value_label.css_classes();
                let kept: Vec<String> = classes
                    .iter()
                    .map(|c| c.to_string())
                    .filter(|c| c != "sysinfo-value" && c != "sysinfo-normal" && c != "sysinfo-warning" && c != "sysinfo-critical")
                    .collect();
                let mut new_classes = kept;
                new_classes.push("sysinfo-value".to_string());
                new_classes.push(css_class.to_string());
                entry.value_label.set_css_classes(&new_classes.iter().map(|s| s.as_str()).collect::<Vec<_>>());

                if let Some(ref label) = entry.label {
                    label.set_text(&comp.label.to_string());
                }
            }

            let has_content = !filtered.is_empty();
            container_ref.set_visible(has_content);
        });
    }
}

fn format_temperature(format: &str, temperature: f32) -> String {
    let mut text = format.replace("{temperature:.0}", &format!("{:.0}", temperature));
    text = text.replace("{temperature:.1}", &format!("{:.1}", temperature));
    text = text.replace("{temperature:.2}", &format!("{:.2}", temperature));
    text = text.replace("{temperature}", &format!("{:.1}", temperature));
    text
}

fn build_gauge_entry(comp: &TemperatureComponent, config: &TemperatureWidgetConfig) -> GaugeEntry {
    let temperature: Option<f32> = comp.temperature.as_ref().copied().into();
    let temperature = temperature.unwrap_or(0.0);
    let max_temp: Option<f32> = comp.max_temperature.as_ref().copied().into();
    let critical_temp: Option<f32> = comp.critical_temperature.as_ref().copied().into();

    let data = Rc::new(RefCell::new(ComponentGaugeData {
        current: temperature,
        max: max_temp,
        critical: critical_temp,
    }));

    let drawing_area = DrawingArea::builder()
        .content_width(config.gauge_size)
        .content_height(config.gauge_size)
        .css_classes(["sysinfo-gauge".to_string()])
        .build();

    let data_clone = data.clone();
    drawing_area.set_draw_func(move |_area, context, width, height| {
        let d = data_clone.borrow();
        draw_temperature_gauge(context, width, height, d.current, d.max, d.critical);
    });

    let content_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let label = if config.show_label {
        let label = Label::builder().css_classes(["sysinfo-temperature-label".to_string()]).build();
        label.set_text(&comp.label.to_string());
        label.set_max_width_chars(20);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        content_box.append(&label);
        Some(label)
    } else {
        None
    };

    let value_label = Label::builder()
        .css_classes(["sysinfo-value".to_string(), "sysinfo-normal".to_string()])
        .build();
    value_label.set_text(&format_temperature(&config.format, temperature));
    content_box.append(&value_label);

    let overlay = Overlay::builder().build();
    overlay.set_child(Some(&drawing_area));
    overlay.add_overlay(&content_box);
    overlay.set_margin_start(4);
    overlay.set_margin_end(4);

    GaugeEntry {
        overlay: overlay.upcast::<Widget>(),
        drawing_area,
        label,
        value_label,
        data,
    }
}

impl MessageHandler<CpuStatusMessage> for TemperatureWidget {
    fn handle_message(&self, message: CpuStatusMessage, _sender_id: &str) {
        self.update_ui(&message);
    }
}

impl AcceptTopic<FfiEnvelope> for TemperatureWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_CPU
    }
}

impl MessageBroadcaster for TemperatureWidget {}

impl PluginMetaGetter for TemperatureWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for TemperatureWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for TemperatureWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == CpuStatusMessage::TYPE_ID {
                    MessageHandler::<CpuStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for TemperatureWidget {
    fn build_widget(&mut self) -> Widget {
        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(Align::Center)
            .valign(Align::Center)
            .css_classes(["sysinfo-temperature-widget".to_string()])
            .build();

        if self.config.show_icon {
            if let Some(ref icon) = self.config.icon {
                let image = build_icon_image(icon, self.config.icon_size);
                image.add_css_class("sysinfo-icon");
                container.append(&image);
            }
        }

        *self.container.borrow_mut() = Some(container.clone());
        container.upcast::<Widget>()
    }
}
