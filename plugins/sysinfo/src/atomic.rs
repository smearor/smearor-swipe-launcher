use crate::labels::SysinfoLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
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
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use smearor_sysinfo_model::BatteryLevel;
use smearor_sysinfo_model::BatteryStatus;
use smearor_sysinfo_model::BatteryStatusMessage;
use smearor_sysinfo_model::CpuStatusMessage;
use smearor_sysinfo_model::DisksStatusMessage;
use smearor_sysinfo_model::MemoryStatusMessage;
use smearor_sysinfo_model::NetworkStatusMessage;
use smearor_sysinfo_model::SysinfoTemperatureLevel;
use smearor_sysinfo_model::UptimeStatusMessage;
use smearor_sysinfo_model::UsageLevel;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which sysinfo metric an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysinfoAtomicView {
    /// CPU usage percentage.
    Cpu,
    /// CPU temperature.
    CpuTemperature,
    /// Memory usage percentage.
    Memory,
    /// Battery level percentage.
    Battery,
    /// Disk usage percentage (first mount or root).
    Disk,
    /// Network download throughput.
    NetworkDownload,
    /// Network upload throughput.
    NetworkUpload,
    /// System uptime.
    Uptime,
    /// 1-minute load average.
    Load,
}

impl FromStr for SysinfoAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sysinfo_cpu" => Ok(Self::Cpu),
            "sysinfo_cpu_temperature" => Ok(Self::CpuTemperature),
            "sysinfo_memory" => Ok(Self::Memory),
            "sysinfo_battery" => Ok(Self::Battery),
            "sysinfo_disk" => Ok(Self::Disk),
            "sysinfo_network_download" => Ok(Self::NetworkDownload),
            "sysinfo_network_upload" => Ok(Self::NetworkUpload),
            "sysinfo_uptime" => Ok(Self::Uptime),
            "sysinfo_load" => Ok(Self::Load),
            _ => Err(format!("Unknown sysinfo atomic view: {s}")),
        }
    }
}

impl SysinfoAtomicView {
    /// Returns the default nerd font icon name for this view.
    ///
    /// Note: `Battery` has a dynamic icon based on charging state, resolved in `render` instead.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Cpu => "nf-fae-chip",
            Self::CpuTemperature => "nf-md-thermometer",
            Self::Memory => "nf-md-memory",
            Self::Battery => "nf-md-battery",
            Self::Disk => "nf-md-harddisk",
            Self::NetworkDownload => "nf-md-download",
            Self::NetworkUpload => "nf-md-upload",
            Self::Uptime => "nf-md-clock_outline",
            Self::Load => "nf-md-chart_line",
        }
    }

    /// Renders this view's display data from the latest status messages and personalization override.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
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
        let view_data = match self {
            Self::Cpu => {
                let status = match cpu {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let usage = status.cpu_usage.clamp(0.0, 100.0);
                let label = SysinfoLabel::Cpu.localized_label(locale);
                let color = UsageLevel::from_percent(usage).get_icon_color();
                ViewData::with_color(self.icon_name().to_string(), format!("{:.0}%", usage), label.to_string(), color)
            }
            Self::CpuTemperature => {
                let status = match cpu {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let temp: Option<f32> = status.cpu_temperature.as_ref().copied().into();
                let temp = match temp {
                    Some(t) => t,
                    None => {
                        return ViewData::new(self.icon_name().to_string(), "--".to_string(), SysinfoLabel::Temperature.localized_label(locale).to_string());
                    }
                };
                let formatted = override_data.format_temperature(temp);
                let label = SysinfoLabel::Temperature.localized_label(locale);
                let color = SysinfoTemperatureLevel::from_celsius(temp).get_icon_color();
                ViewData::with_color(self.icon_name().to_string(), formatted, label.to_string(), color)
            }
            Self::Memory => {
                let status = match memory {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let usage = status.memory_usage.clamp(0.0, 100.0);
                let label = SysinfoLabel::Memory.localized_label(locale);
                let color = UsageLevel::from_percent(usage).get_icon_color();
                ViewData::with_color(self.icon_name().to_string(), format!("{:.0}%", usage), label.to_string(), color)
            }
            Self::Battery => {
                let status = match battery {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
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
            Self::Disk => {
                let status = match disks {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let usage = status.mounts.iter().next().map(|m| m.usage).unwrap_or(0.0);
                let label = SysinfoLabel::Disk.localized_label(locale);
                let color = UsageLevel::from_percent(usage).get_icon_color();
                ViewData::with_color(self.icon_name().to_string(), format!("{:.0}%", usage), label.to_string(), color)
            }
            Self::NetworkDownload => {
                let status = match network {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let formatted = override_data.format_data_rate(status.received_bytes_per_second);
                let label = SysinfoLabel::Download.localized_label(locale);
                ViewData::new(self.icon_name().to_string(), formatted, label.to_string())
            }
            Self::NetworkUpload => {
                let status = match network {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let formatted = override_data.format_data_rate(status.transmitted_bytes_per_second);
                let label = SysinfoLabel::Upload.localized_label(locale);
                ViewData::new(self.icon_name().to_string(), formatted, label.to_string())
            }
            Self::Uptime => {
                let status = match uptime {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
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
                ViewData::new(self.icon_name().to_string(), formatted, label.to_string())
            }
            Self::Load => {
                let status = match uptime {
                    Some(s) => s,
                    None => return ViewData::error(self.icon_name().to_string(), "Loading...".to_string()),
                };
                let formatted = format!("{:.2}", status.load_average_1_minute);
                let label = SysinfoLabel::Load.localized_label(locale);
                ViewData::new(self.icon_name().to_string(), formatted, label.to_string())
            }
        };
        view_data.with_text_colors(text_colors)
    }
}

/// Atomic sysinfo widget that renders a single system metric.
///
/// Subscribes to all sysinfo status topics and renders only the view specified
/// at construction time. No view switching — each atomic widget is a
/// single-purpose display.
pub struct SysinfoAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: SysinfoAtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_cpu: Rc<RefCell<Option<CpuStatusMessage>>>,
    pub latest_memory: Rc<RefCell<Option<MemoryStatusMessage>>>,
    pub latest_battery: Rc<RefCell<Option<BatteryStatusMessage>>>,
    pub latest_disks: Rc<RefCell<Option<DisksStatusMessage>>>,
    pub latest_network: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_uptime: Rc<RefCell<Option<UptimeStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl SysinfoAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = SysinfoAtomicView::from_str(widget_name).unwrap_or(SysinfoAtomicView::Cpu);

        let widget = SysinfoAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
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

    fn update_ui(&self) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_network.borrow(),
            &self.latest_uptime.borrow(),
            &override_data,
            &self.config.text_colors,
        );
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f2db}');
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, view_data.main_text_color);
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, view_data.info_text_color);
        }
    }

    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(
            &self.latest_cpu.borrow(),
            &self.latest_memory.borrow(),
            &self.latest_battery.borrow(),
            &self.latest_disks.borrow(),
            &self.latest_network.borrow(),
            &self.latest_uptime.borrow(),
            &override_data,
            &self.config.text_colors,
        );

        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f2db}');
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.is_error = view_data.is_error;
        data.icon_color = view_data.icon_color.map(|c| c.to_rgba());
        data.main_text_color = view_data.main_text_color.map(|c| c.to_rgba());
        data.info_text_color = view_data.info_text_color.map(|c| c.to_rgba());
        data
    }
}

atomic_widget_impl! {
    widget: SysinfoAtomicWidget,
    debug_tag: "sysinfo-atomic",
    mcp_description: "Sysinfo atomic widget",
    css_prefix: "sysinfo",
    default_icon: '\u{f2db}',
    default_main: "--",
    default_info: "Loading...",
    extra_message_types: [
        CpuStatusMessage,
        MemoryStatusMessage,
        BatteryStatusMessage,
        DisksStatusMessage,
        NetworkStatusMessage,
        UptimeStatusMessage,
        FfiEnvelopePayload<PersonalizationStatusMessage>
    ]
}

impl MessageHandler<CpuStatusMessage> for SysinfoAtomicWidget {
    fn handle_message(&self, message: CpuStatusMessage, _sender_id: &str) {
        trace!("sysinfo atomic widget: received cpu status");
        *self.latest_cpu.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<MemoryStatusMessage> for SysinfoAtomicWidget {
    fn handle_message(&self, message: MemoryStatusMessage, _sender_id: &str) {
        trace!("sysinfo atomic widget: received memory status");
        *self.latest_memory.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<BatteryStatusMessage> for SysinfoAtomicWidget {
    fn handle_message(&self, message: BatteryStatusMessage, _sender_id: &str) {
        trace!("sysinfo atomic widget: received battery status");
        *self.latest_battery.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<DisksStatusMessage> for SysinfoAtomicWidget {
    fn handle_message(&self, message: DisksStatusMessage, _sender_id: &str) {
        trace!("sysinfo atomic widget: received disks status");
        *self.latest_disks.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<NetworkStatusMessage> for SysinfoAtomicWidget {
    fn handle_message(&self, message: NetworkStatusMessage, _sender_id: &str) {
        trace!("sysinfo atomic widget: received network status");
        *self.latest_network.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<UptimeStatusMessage> for SysinfoAtomicWidget {
    fn handle_message(&self, message: UptimeStatusMessage, _sender_id: &str) {
        trace!("sysinfo atomic widget: received uptime status");
        *self.latest_uptime.borrow_mut() = Some(message);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for SysinfoAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("sysinfo atomic widget: received personalization status");
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
