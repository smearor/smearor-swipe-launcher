use crate::battery::widget::BatteryWidget;
use crate::cpu::widget::CpuWidget;
use crate::disks::widget::DisksWidget;
use crate::labels::SysinfoLabel;
use crate::memory::widget::MemoryWidget;
use crate::multi_widget::SysinfoMultiWidget;
use crate::multi_widget::render_view as render_multi_view;
use crate::network::widget::NetworkWidget;
use crate::temperature::widget::TemperatureWidget;
use crate::uptime::widget::UptimeWidget;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WebRenderer;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_sysinfo_model::BatteryLevel;
use smearor_sysinfo_model::BatteryStatus;
use smearor_sysinfo_model::SysinfoTemperatureLevel;
use smearor_sysinfo_model::UsageLevel;

/// Renders view data as an HTML fragment.
fn render_view_data_to_html(plugin_id: &str, widget_class: &str, view_data: ViewData) -> String {
    let mut html = html_expanded_open(plugin_id, widget_class);
    let icon_class = if view_data.icon_name.starts_with("nf-") {
        format!("nerd-icon nerd-{}", view_data.icon_name)
    } else {
        format!("icon icon-{}", view_data.icon_name)
    };
    let color_style = if let Some(color) = view_data.icon_color {
        format!(
            r#" style="color: rgba({}, {}, {}, {});""#,
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            color.a
        )
    } else {
        String::new()
    };
    html.push_str(&format!(
        r#"<div class="smearor-{}-icon"><span class="{}"{}></span></div>"#,
        widget_class, icon_class, color_style
    ));
    let main_color_style = if let Some(color) = view_data.main_text_color {
        format!(
            r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            color.a
        )
    } else {
        String::new()
    };
    let info_color_style = if let Some(color) = view_data.info_text_color {
        format!(
            r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            color.a
        )
    } else {
        String::new()
    };
    html.push_str(&format!(r#"<div class="smearor-{}-main"{}>{}</div>"#, widget_class, main_color_style, view_data.main_text));
    let marquee_class = if view_data.info_text.len() > 20 { " marquee" } else { "" };
    html.push_str(&format!(
        r#"<div class="smearor-{}-info{}"{}>{}</div>"#,
        widget_class, marquee_class, info_color_style, view_data.info_text
    ));
    html.push_str(html_expanded_close());
    html
}

fn loading_view_data(icon: &str) -> ViewData {
    ViewData::error(icon.to_string(), "Loading...".to_string())
}

impl WebRenderer for CpuWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-fae-chip"),
            Some(s) => {
                let usage = s.cpu_usage.clamp(0.0, 100.0);
                let label = SysinfoLabel::Cpu.localized_label(locale);
                let color = UsageLevel::from_percent(usage).get_icon_color();
                ViewData::with_color("nf-fae-chip".to_string(), format!("{:.0}%", usage), label.to_string(), color)
            }
        }
        .with_text_colors(&self.config.percentage.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-cpu", view_data)
    }
}

impl WebRenderer for MemoryWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-md-memory"),
            Some(s) => {
                let usage = s.memory_usage.clamp(0.0, 100.0);
                let label = SysinfoLabel::Memory.localized_label(locale);
                let color = UsageLevel::from_percent(usage).get_icon_color();
                ViewData::with_color("nf-md-memory".to_string(), format!("{:.0}%", usage), label.to_string(), color)
            }
        }
        .with_text_colors(&self.config.percentage.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-memory", view_data)
    }
}

impl WebRenderer for BatteryWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-md-battery"),
            Some(s) => {
                let level = s.level.clamp(0.0, 100.0);
                let icon = match s.status {
                    BatteryStatus::Charging => "nf-md-battery_charging",
                    BatteryStatus::Full => "nf-md-battery",
                    BatteryStatus::Discharging => "nf-md-battery_alert",
                    BatteryStatus::Unknown => "nf-md-battery",
                };
                let label = SysinfoLabel::Battery.localized_label(locale);
                let color = BatteryLevel::from_status(level, s.status).get_icon_color();
                ViewData::with_color(icon.to_string(), format!("{:.0}%", level), label.to_string(), color)
            }
        }
        .with_text_colors(&self.config.percentage.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-battery", view_data)
    }
}

impl WebRenderer for DisksWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-md-harddisk"),
            Some(s) => {
                let usage = s.mounts.iter().next().map(|m| m.usage).unwrap_or(0.0);
                let label = SysinfoLabel::Disk.localized_label(locale);
                let color = UsageLevel::from_percent(usage).get_icon_color();
                ViewData::with_color("nf-md-harddisk".to_string(), format!("{:.0}%", usage), label.to_string(), color)
            }
        }
        .with_text_colors(&self.config.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-disks", view_data)
    }
}

impl WebRenderer for NetworkWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-md-network"),
            Some(s) => {
                let down = override_data.format_data_rate(s.received_bytes_per_second);
                let up = override_data.format_data_rate(s.transmitted_bytes_per_second);
                let label = SysinfoLabel::Network.localized_label(locale);
                ViewData::new("nf-md-network".to_string(), format!("{}\u{2193} {}\u{2191}", down, up), label.to_string())
            }
        }
        .with_text_colors(&self.config.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-network", view_data)
    }
}

impl WebRenderer for TemperatureWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-md-thermometer"),
            Some(s) => {
                let temp: Option<f32> = s.cpu_temperature.as_ref().copied().into();
                let temp = match temp {
                    Some(t) => t,
                    None => {
                        return render_view_data_to_html(
                            plugin_id,
                            "sysinfo-temperature",
                            ViewData::new(
                                "nf-md-thermometer".to_string(),
                                "--".to_string(),
                                SysinfoLabel::Temperature.localized_label(locale).to_string(),
                            ),
                        );
                    }
                };
                let formatted = override_data.format_temperature(temp);
                let label = SysinfoLabel::Temperature.localized_label(locale);
                let color = SysinfoTemperatureLevel::from_celsius(temp).get_icon_color();
                ViewData::with_color("nf-md-thermometer".to_string(), formatted, label.to_string(), color)
            }
        }
        .with_text_colors(&self.config.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-temperature", view_data)
    }
}

impl WebRenderer for UptimeWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let status = self.latest_status.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        let view_data = match status.as_ref() {
            None => loading_view_data("nf-md-clock_outline"),
            Some(s) => {
                let seconds = s.uptime_seconds;
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
        }
        .with_text_colors(&self.config.text_colors);

        render_view_data_to_html(plugin_id, "sysinfo-uptime", view_data)
    }
}

impl WebRenderer for SysinfoMultiWidget {
    fn render_html(&self, _instance_id: &str, plugin_id: &str) -> String {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(smearor_sysinfo_model::SysinfoView::Cpu);
        let override_data = self.personalization.borrow().clone();

        let view_data = render_multi_view(
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

        render_view_data_to_html(plugin_id, "sysinfo-multi", view_data)
    }
}
