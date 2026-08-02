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
use smearor_render_utils::Color;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_sysinfo_model::BatteryLevel;
use smearor_sysinfo_model::BatteryStatus;
use smearor_sysinfo_model::SysinfoTemperatureLevel;
use smearor_sysinfo_model::UsageLevel;
use tracing::trace;

/// Background color for error/loading states (dark red).
const BG_COLOR_ERROR: Color = [40, 20, 20, 255];

/// Text color for error/loading states (muted red).
const TEXT_COLOR_ERROR: Color = [200, 100, 100, 255];

/// Renders view data onto a pixel buffer for headless graphic rendering.
fn render_view_data_to_graphic(width: u32, height: u32, view_data: ViewData, icon_size: f32) -> FfiGraphic {
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    let bg = if view_data.is_error { BG_COLOR_ERROR } else { background_color(false) };
    fill_background(&mut pixels, width, height, bg);

    let text_col = if view_data.is_error { TEXT_COLOR_ERROR } else { text_color(false) };
    let icon_col = view_data.icon_color.map(|c| c.to_rgba()).unwrap_or(text_col);
    let main_col = view_data.main_text_color.map(|c| c.to_rgba()).unwrap_or(text_col);
    let info_col = view_data.info_text_color.map(|c| c.to_rgba()).unwrap_or(text_col);

    let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f2db}');
    draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

    if !view_data.main_text.is_empty() {
        draw_text_centered(
            &mut pixels,
            width,
            height,
            &view_data.main_text,
            height as f32 * 0.72,
            (height as f32 * 0.22).min(16.0).max(10.0),
            main_col,
        );
    }

    if !view_data.info_text.is_empty() {
        draw_text_centered(
            &mut pixels,
            width,
            height,
            &view_data.info_text,
            height as f32 * 0.92,
            (height as f32 * 0.16).min(12.0).max(8.0),
            info_col,
        );
    }

    FfiGraphic::from_pixels(width, height, pixels)
}

fn loading_view_data(icon: &str) -> ViewData {
    ViewData::error(icon.to_string(), "Loading...".to_string())
}

impl GraphicRenderer for CpuWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("CpuWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for MemoryWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("MemoryWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for BatteryWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("BatteryWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for DisksWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("DisksWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for NetworkWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("NetworkWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for TemperatureWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("TemperatureWidget: render_graphic {}x{}", width, height);

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
                        return render_view_data_to_graphic(
                            width,
                            height,
                            ViewData::new(
                                "nf-md-thermometer".to_string(),
                                "--".to_string(),
                                SysinfoLabel::Temperature.localized_label(locale).to_string(),
                            ),
                            (height as f32 * 0.5).min(40.0),
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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for UptimeWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("UptimeWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}

impl GraphicRenderer for SysinfoMultiWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("SysinfoMultiWidget: render_graphic {}x{}", width, height);

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

        let icon_size = (height as f32 * 0.5).min(40.0);
        render_view_data_to_graphic(width, height, view_data, icon_size)
    }
}
