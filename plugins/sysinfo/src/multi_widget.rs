use crate::config::SysinfoMultiWidgetConfig;
use crate::labels::SysinfoLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::Color;
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
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use smearor_sysinfo_model::BatteryLevel;
use smearor_sysinfo_model::BatteryStatus;
use smearor_sysinfo_model::BatteryStatusMessage;
use smearor_sysinfo_model::CpuStatusMessage;
use smearor_sysinfo_model::DisksStatusMessage;
use smearor_sysinfo_model::MemoryStatusMessage;
use smearor_sysinfo_model::NetworkStatusMessage;
use smearor_sysinfo_model::SysinfoTemperatureLevel;
use smearor_sysinfo_model::SysinfoView;
use smearor_sysinfo_model::TOPIC_BATTERY;
use smearor_sysinfo_model::TOPIC_CPU;
use smearor_sysinfo_model::TOPIC_DISKS;
use smearor_sysinfo_model::TOPIC_MEMORY;
use smearor_sysinfo_model::TOPIC_NETWORK;
use smearor_sysinfo_model::TOPIC_UPTIME;
use smearor_sysinfo_model::UptimeStatusMessage;
use smearor_sysinfo_model::UsageLevel;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

type SharedImage = Rc<RefCell<Option<Image>>>;
type SharedLabel = Rc<RefCell<Option<Label>>>;
type SharedLevelBar = Rc<RefCell<Option<LevelBar>>>;

/// Multi-view sysinfo widget that cycles through system metrics.
///
/// Subscribes to all sysinfo status topics and renders the current view.
/// Swipe Up / Swipe Down cycles through configured views.
pub struct SysinfoMultiWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: SysinfoMultiWidgetConfig,
    pub icon_image: SharedImage,
    pub value_label: SharedLabel,
    pub info_label: SharedLabel,
    pub level_bar: SharedLevelBar,
    pub spacer_label: SharedLabel,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_cpu: Rc<RefCell<Option<CpuStatusMessage>>>,
    pub latest_memory: Rc<RefCell<Option<MemoryStatusMessage>>>,
    pub latest_battery: Rc<RefCell<Option<BatteryStatusMessage>>>,
    pub latest_disks: Rc<RefCell<Option<DisksStatusMessage>>>,
    pub latest_network: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_uptime: Rc<RefCell<Option<UptimeStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl SysinfoMultiWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: SysinfoMultiWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget = Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            icon_image: Rc::new(RefCell::new(None)),
            value_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            level_bar: Rc::new(RefCell::new(None)),
            spacer_label: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            latest_cpu: Rc::new(RefCell::new(None)),
            latest_memory: Rc::new(RefCell::new(None)),
            latest_battery: Rc::new(RefCell::new(None)),
            latest_disks: Rc::new(RefCell::new(None)),
            latest_network: Rc::new(RefCell::new(None)),
            latest_uptime: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Broadcast a WidgetUpdateMessage so headless/Web instances re-render this widget.
    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(msg);
    }

    fn update_ui(&self) {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(SysinfoView::Cpu);

        let override_data = self.personalization.borrow().clone();
        let view_data = render_view(
            view,
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_network.borrow(),
            &self.latest_uptime.borrow(),
            &override_data,
            &self.config.text_colors,
        );

        if let Some(ref img) = *self.icon_image.borrow() {
            update_icon_display(img, &view_data, self.config.icon_config.icon_size(), self.config.icon_config.icon_color());
        }
        if let Some(ref label) = *self.value_label.borrow() {
            label.set_text(&view_data.main_text);
            apply_text_color(label, view_data.main_text_color.or(self.config.text_colors.main_text_color()));
        }
        if let Some(ref label) = *self.info_label.borrow() {
            label.set_text(&view_data.info_text);
            apply_text_color(label, view_data.info_text_color.or(self.config.text_colors.info_text_color()));
        }

        update_bar_visibility(
            &self.level_bar,
            &self.spacer_label,
            view,
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_uptime.borrow(),
        );
        self.broadcast_widget_update();
    }

    fn next_view(&self) {
        self.cycle_view(1);
    }

    fn prev_view(&self) {
        self.cycle_view(-1);
    }

    fn cycle_view(&self, direction: i32) {
        if self.config.views.is_empty() {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        let len = self.config.views.len() as i32;
        let new_idx = (*idx as i32 + direction).rem_euclid(len) as usize;
        *idx = new_idx;
        let view = self.config.views[*idx];
        drop(idx);

        let override_data = self.personalization.borrow().clone();
        let view_data = render_view(
            view,
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_network.borrow(),
            &self.latest_uptime.borrow(),
            &override_data,
            &self.config.text_colors,
        );

        if let Some(ref img) = *self.icon_image.borrow() {
            update_icon_display(img, &view_data, self.config.icon_config.icon_size(), self.config.icon_config.icon_color());
        }
        if let Some(ref label) = *self.value_label.borrow() {
            label.set_text(&view_data.main_text);
            apply_text_color(label, view_data.main_text_color.or(self.config.text_colors.main_text_color()));
        }
        if let Some(ref label) = *self.info_label.borrow() {
            label.set_text(&view_data.info_text);
            apply_text_color(label, view_data.info_text_color.or(self.config.text_colors.info_text_color()));
        }

        update_bar_visibility(
            &self.level_bar,
            &self.spacer_label,
            view,
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_uptime.borrow(),
        );
        self.broadcast_widget_update();
    }
}

impl MessageHandler<CpuStatusMessage> for SysinfoMultiWidget {
    fn handle_message(&self, message: CpuStatusMessage, _sender_id: &str) {
        *self.latest_cpu.borrow_mut() = Some(message);
        self.update_ui();
    }
}

impl MessageHandler<MemoryStatusMessage> for SysinfoMultiWidget {
    fn handle_message(&self, message: MemoryStatusMessage, _sender_id: &str) {
        *self.latest_memory.borrow_mut() = Some(message);
        self.update_ui();
    }
}

impl MessageHandler<BatteryStatusMessage> for SysinfoMultiWidget {
    fn handle_message(&self, message: BatteryStatusMessage, _sender_id: &str) {
        *self.latest_battery.borrow_mut() = Some(message);
        self.update_ui();
    }
}

impl MessageHandler<DisksStatusMessage> for SysinfoMultiWidget {
    fn handle_message(&self, message: DisksStatusMessage, _sender_id: &str) {
        *self.latest_disks.borrow_mut() = Some(message);
        self.update_ui();
    }
}

impl MessageHandler<NetworkStatusMessage> for SysinfoMultiWidget {
    fn handle_message(&self, message: NetworkStatusMessage, _sender_id: &str) {
        *self.latest_network.borrow_mut() = Some(message);
        self.update_ui();
    }
}

impl MessageHandler<UptimeStatusMessage> for SysinfoMultiWidget {
    fn handle_message(&self, message: UptimeStatusMessage, _sender_id: &str) {
        *self.latest_uptime.borrow_mut() = Some(message);
        self.update_ui();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for SysinfoMultiWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            temperature_unit: Some(status.temperature_unit),
            measurement_system: Some(status.measurement_system),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        self.update_ui();
    }
}

impl AcceptTopic<FfiEnvelope> for SysinfoMultiWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_CPU
            || topic == TOPIC_MEMORY
            || topic == TOPIC_BATTERY
            || topic == TOPIC_DISKS
            || topic == TOPIC_NETWORK
            || topic == TOPIC_UPTIME
            || topic == <FfiEnvelopePayload<PersonalizationStatusMessage> as MessageTopic>::topic()
            || topic == <FfiEnvelopePayload<InvokeToolMessage> as MessageTopic>::topic()
    }
}

impl MessageBroadcaster for SysinfoMultiWidget {}

impl McpCapabilitiesRegistrator for SysinfoMultiWidget {
    fn register_mcp_capabilities(&self) {
        if self.config.description.is_some() {
            let tool = RegisterToolMessage::new(
                &format!("button_{}", self.meta.id),
                self.config.description.as_deref().unwrap_or("Sysinfo multi-view widget"),
                r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "hold_start", "hold_stop", "double_press", "compound_longpress"], "description": "The action to trigger" } }, "required": ["action"] }"#,
            );
            MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
        }
    }
}

impl PluginMetaGetter for SysinfoMultiWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for SysinfoMultiWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for SysinfoMultiWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                let type_id = envelope.type_id;
                trace!("SysinfoMultiWidget: on_message topic={} type_id={}", topic, type_id);
                if envelope.type_id == CpuStatusMessage::TYPE_ID {
                    MessageHandler::<CpuStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == MemoryStatusMessage::TYPE_ID {
                    MessageHandler::<MemoryStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == BatteryStatusMessage::TYPE_ID {
                    MessageHandler::<BatteryStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == DisksStatusMessage::TYPE_ID {
                    MessageHandler::<DisksStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == NetworkStatusMessage::TYPE_ID {
                    MessageHandler::<NetworkStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == UptimeStatusMessage::TYPE_ID {
                    MessageHandler::<UptimeStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<InvokeToolMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for SysinfoMultiWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let broadcaster = self.get_broadcaster();
        let show_labels = !config.icon_config.icon_only();

        let content_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.layout.spacing_or_default())
            .css_classes(["menu_button_inner"])
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(true)
            .build();

        let icon_image = Image::new();
        icon_image.set_pixel_size(config.icon_config.icon_size());
        icon_image.add_css_class("nerd-icon");
        if let Some(color) = config.icon_config.icon_color() {
            apply_icon_color(&icon_image, color);
        }
        content_box.append(&icon_image);
        *self.icon_image.borrow_mut() = Some(icon_image);

        let value_label = Label::builder()
            .label(if show_labels { "Loading..." } else { "" })
            .css_classes(["widget-main-text"])
            .build();
        value_label.set_height_request(20);
        apply_text_color(&value_label, config.text_colors.main_text_color());
        content_box.append(&value_label);
        *self.value_label.borrow_mut() = Some(value_label);

        let info_label = Label::builder()
            .label(if show_labels { "" } else { "" })
            .css_classes(["widget-info-text"])
            .build();
        info_label.set_height_request(16);
        apply_text_color(&info_label, config.text_colors.info_text_color());
        content_box.append(&info_label);
        *self.info_label.borrow_mut() = Some(info_label);

        let effective_width = config.dimensions.width_or_default().min(config.dimensions.max_width_or_default(config.mode));
        let bar = LevelBar::builder()
            .min_value(0.0)
            .max_value(100.0)
            .orientation(Orientation::Horizontal)
            .width_request(effective_width)
            .height_request(16)
            .css_classes(["sysinfo-bar", "sysinfo-normal"])
            .visible(false)
            .build();
        content_box.append(&bar);
        *self.level_bar.borrow_mut() = Some(bar);

        let spacer = Label::new(Some(""));
        spacer.set_height_request(16);
        content_box.append(&spacer);
        *self.spacer_label.borrow_mut() = Some(spacer);

        let mut button_builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(effective_width)
            .child(&content_box);
        if let Some(max_w) = config.dimensions.max_width {
            button_builder = button_builder.hexpand(false).halign(Align::Start);
            let css_class = format!("max-width-{}", max_w);
            button_builder = button_builder.css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".max-width-{} {{ max-width: {}px; }}", max_w, max_w);
            if let Some(display) = gtk4::gdk::Display::default() {
                let provider = CssProvider::new();
                provider.load_from_string(&css);
                gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            }
        }
        let button = button_builder.build();

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            icon_image: self.icon_image.clone(),
            value_label: self.value_label.clone(),
            info_label: self.info_label.clone(),
            level_bar: self.level_bar.clone(),
            spacer_label: self.spacer_label.clone(),
            current_view: self.current_view.clone(),
            latest_cpu: self.latest_cpu.clone(),
            latest_memory: self.latest_memory.clone(),
            latest_battery: self.latest_battery.clone(),
            latest_disks: self.latest_disks.clone(),
            latest_network: self.latest_network.clone(),
            latest_uptime: self.latest_uptime.clone(),
            personalization: self.personalization.clone(),
        });

        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        widget_self.attach_gesture_handlers(&button_widget, &config.actions, &broadcaster, &GestureHandlersConfiguration::default());

        button_widget
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for SysinfoMultiWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let own_button_name = format!("button_{}", self.meta.id);
        debug!(
            "SysinfoMultiWidget: InvokeToolMessage name={} own_button_name={} meta_id={}",
            tool_name, own_button_name, self.meta.id
        );
        if tool_name != own_button_name {
            return;
        }
        let action_str = serde_json::from_str::<serde_json::Value>(&message.0.arguments)
            .ok()
            .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "click".to_string());

        let action_kind = ActionKind::from_str(&action_str).ok();
        let broadcaster = self.get_broadcaster();

        if let Some(kind) = action_kind {
            trace!("SysinfoMultiWidget: handling InvokeTool action '{}'", action_str);
            let binding = self.config.binding_for_kind(kind);
            if binding.is_configured() {
                binding.dispatch(&broadcaster);
                if binding.is_supplement() {
                    self.default_fallback(&kind, &broadcaster);
                }
            } else {
                self.default_fallback(&kind, &broadcaster);
            }
        }

        let response = InvokeToolResponse::success(&message.0.correlation_id.to_string(), &format!("{} handled", action_str));
        broadcaster.broadcast_message_to_topic(response);
    }
}

impl DefaultFallback for SysinfoMultiWidget {
    fn default_fallback(&self, kind: &ActionKind, _broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::SwipeUp | ActionKind::ScrollUp | ActionKind::MiddleClick => {
                self.next_view();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.prev_view();
            }
            ActionKind::DoublePress | ActionKind::Longpress | ActionKind::RightClick | ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {
                debug!("SysinfoMultiWidget: no action for {:?}", kind);
            }
        }
    }
}

fn set_icon_image(img: &Image, icon_name: &str, icon_size: i32) {
    if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
        img.set_icon_name(Some(&gtk_icon_name));
    }
    img.set_pixel_size(icon_size);
}

fn update_icon_display(img: &Image, view_data: &ViewData, icon_size: i32, configured_color: Option<Color>) {
    set_icon_image(img, &view_data.icon_name, icon_size);
    if let Some(c) = configured_color {
        apply_icon_color(img, c);
    }
}

fn update_bar_css(bar: &LevelBar, value: f32, warning: f32, critical: f32) {
    bar.remove_css_class("sysinfo-normal");
    bar.remove_css_class("sysinfo-warning");
    bar.remove_css_class("sysinfo-critical");
    let class = if value >= critical {
        "sysinfo-critical"
    } else if value >= warning {
        "sysinfo-warning"
    } else {
        "sysinfo-normal"
    };
    bar.add_css_class(class);
}

fn update_bar_visibility(
    level_bar: &SharedLevelBar,
    spacer_label: &SharedLabel,
    view: SysinfoView,
    cpu: &Option<CpuStatusMessage>,
    memory: &Option<MemoryStatusMessage>,
    battery: &Option<BatteryStatusMessage>,
    disks: &Option<DisksStatusMessage>,
    uptime: &Option<UptimeStatusMessage>,
) {
    let (has_bar, value) = match view {
        SysinfoView::Cpu => cpu.as_ref().map(|s| (true, s.cpu_usage.clamp(0.0, 100.0))).unwrap_or((false, 0.0)),
        SysinfoView::Memory => memory.as_ref().map(|s| (true, s.memory_usage.clamp(0.0, 100.0))).unwrap_or((false, 0.0)),
        SysinfoView::Battery => battery.as_ref().map(|s| (true, s.level.clamp(0.0, 100.0))).unwrap_or((false, 0.0)),
        SysinfoView::Disk => disks
            .as_ref()
            .map(|s| (true, s.mounts.iter().next().map(|m| m.usage).unwrap_or(0.0)))
            .unwrap_or((false, 0.0)),
        SysinfoView::Load => uptime
            .as_ref()
            .map(|s| (true, (s.load_average_1_minute * 100.0 / available_parallelism_f32()).clamp(0.0, 100.0)))
            .unwrap_or((false, 0.0)),
        _ => (false, 0.0),
    };

    if has_bar {
        if let Some(ref bar) = *level_bar.borrow() {
            bar.set_value(value as f64);
            bar.set_visible(true);
            update_bar_css(bar, value, 70.0, 90.0);
        }
        if let Some(ref spacer) = *spacer_label.borrow() {
            spacer.set_visible(false);
        }
    } else {
        if let Some(ref bar) = *level_bar.borrow() {
            bar.set_visible(false);
        }
        if let Some(ref spacer) = *spacer_label.borrow() {
            spacer.set_visible(true);
        }
    }
}

fn available_parallelism_f32() -> f32 {
    std::thread::available_parallelism().map(|n| n.get() as f32).unwrap_or(1.0)
}

/// Renders the display data for a given sysinfo view.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_view(
    view: SysinfoView,
    cpu: &Option<CpuStatusMessage>,
    memory: &Option<MemoryStatusMessage>,
    battery: &Option<BatteryStatusMessage>,
    disks: &Option<DisksStatusMessage>,
    network: &Option<NetworkStatusMessage>,
    uptime: &Option<UptimeStatusMessage>,
    override_data: &PersonalizationOverride,
    text_colors: &WidgetTextColors,
) -> ViewData {
    let locale = override_data.locale;
    let view_data = match view {
        SysinfoView::Cpu => {
            let status = match cpu {
                Some(s) => s,
                None => return ViewData::error("nf-md-gauge_empty".to_string(), "Loading...".to_string()),
            };
            let usage = status.cpu_usage.clamp(0.0, 100.0);
            let level = UsageLevel::from_percent(usage);
            let icon = level.get_icon_name().unwrap_or_else(|| "nf-md-gauge_empty".to_string());
            let label = SysinfoLabel::Cpu.localized_label(locale);
            let color = level.get_icon_color();
            ViewData::with_color(icon, format!("{:.0}%", usage), label.to_string(), color)
        }
        SysinfoView::CpuTemperature => {
            let status = match cpu {
                Some(s) => s,
                None => return ViewData::error("nf-fa-thermometer_empty".to_string(), "Loading...".to_string()),
            };
            let temp: Option<f32> = status.cpu_temperature.as_ref().copied().into();
            let temp = match temp {
                Some(t) => t,
                None => {
                    return ViewData::new(
                        "nf-fa-thermometer_empty".to_string(),
                        "--".to_string(),
                        SysinfoLabel::Temperature.localized_label(locale).to_string(),
                    );
                }
            };
            let formatted = override_data.format_temperature(temp);
            let level = SysinfoTemperatureLevel::from_celsius(temp);
            let icon = level.get_icon_name().unwrap_or_else(|| "nf-fa-thermometer_empty".to_string());
            let label = SysinfoLabel::Temperature.localized_label(locale);
            let color = level.get_icon_color();
            ViewData::with_color(icon, formatted, label.to_string(), color)
        }
        SysinfoView::Memory => {
            let status = match memory {
                Some(s) => s,
                None => return ViewData::error("nf-md-memory".to_string(), "Loading...".to_string()),
            };
            let usage = status.memory_usage.clamp(0.0, 100.0);
            let label = SysinfoLabel::Memory.localized_label(locale);
            let color = UsageLevel::from_percent(usage).get_icon_color();
            ViewData::with_color("nf-md-memory".to_string(), format!("{:.0}%", usage), label.to_string(), color)
        }
        SysinfoView::Battery => {
            let status = match battery {
                Some(s) => s,
                None => return ViewData::error("nf-md-battery".to_string(), "Loading...".to_string()),
            };
            let level = status.level.clamp(0.0, 100.0);
            let icon = match status.status {
                BatteryStatus::Charging => "nf-md-battery_charging",
                BatteryStatus::Full => "nf-md-battery",
                BatteryStatus::Discharging => "nf-md-battery_alert",
                BatteryStatus::Unknown => "nf-md-battery",
            };
            let label = SysinfoLabel::Battery.localized_label(locale);
            let color = BatteryLevel::from_status(level, status.status).get_icon_color();
            ViewData::with_color(icon.to_string(), format!("{:.0}%", level), label.to_string(), color)
        }
        SysinfoView::Disk => {
            let status = match disks {
                Some(s) => s,
                None => return ViewData::error("nf-md-harddisk".to_string(), "Loading...".to_string()),
            };
            let usage = status.mounts.iter().next().map(|m| m.usage).unwrap_or(0.0);
            let label = SysinfoLabel::Disk.localized_label(locale);
            let color = UsageLevel::from_percent(usage).get_icon_color();
            ViewData::with_color("nf-md-harddisk".to_string(), format!("{:.0}%", usage), label.to_string(), color)
        }
        SysinfoView::NetworkDownload => {
            let status = match network {
                Some(s) => s,
                None => return ViewData::error("nf-md-download".to_string(), "Loading...".to_string()),
            };
            let formatted = override_data.format_data_rate(status.received_bytes_per_second);
            let label = SysinfoLabel::Download.localized_label(locale);
            ViewData::new("nf-md-download".to_string(), formatted, label.to_string())
        }
        SysinfoView::NetworkUpload => {
            let status = match network {
                Some(s) => s,
                None => return ViewData::error("nf-md-upload".to_string(), "Loading...".to_string()),
            };
            let formatted = override_data.format_data_rate(status.transmitted_bytes_per_second);
            let label = SysinfoLabel::Upload.localized_label(locale);
            ViewData::new("nf-md-upload".to_string(), formatted, label.to_string())
        }
        SysinfoView::Uptime => {
            let status = match uptime {
                Some(s) => s,
                None => return ViewData::error("nf-md-clock_outline".to_string(), "Loading...".to_string()),
            };
            let seconds = status.uptime_seconds;
            let days = seconds / 86400;
            let hours = (seconds % 86400) / 3600;
            let minutes = (seconds % 3600) / 60;
            let formatted = if days > 0 {
                format!("{}d {:02}h", days, hours)
            } else {
                format!("{:02}h {:02}m", hours, minutes)
            };
            let label = SysinfoLabel::Uptime.localized_label(locale);
            ViewData::new("nf-md-clock_outline".to_string(), formatted, label.to_string())
        }
        SysinfoView::Load => {
            let status = match uptime {
                Some(s) => s,
                None => return ViewData::error("nf-md-chart_line".to_string(), "Loading...".to_string()),
            };
            let load = status.load_average_1_minute;
            let label = SysinfoLabel::Load.localized_label(locale);
            let pct = (load * 100.0 / available_parallelism_f32()).clamp(0.0, 100.0);
            let color = UsageLevel::from_percent(pct).get_icon_color();
            ViewData::with_color("nf-md-chart_line".to_string(), format!("{:.2}", load), label.to_string(), color)
        }
    };
    view_data.with_text_colors(text_colors)
}
