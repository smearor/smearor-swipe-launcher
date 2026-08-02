pub(crate) mod atomic;
pub mod battery;
pub mod config;
pub mod cpu;
pub mod disks;
pub mod graphic;
pub mod html;
pub mod labels;
pub mod memory;
pub mod multi_widget;
pub mod network;
pub mod personalization;
pub mod shared;
pub mod temperature;
pub mod uptime;

use crate::atomic::SysinfoAtomicWidget;
use crate::battery::BatteryWidget;
use crate::cpu::CpuWidget;
use crate::disks::DisksWidget;
use crate::memory::MemoryWidget;
use crate::multi_widget::SysinfoMultiWidget;
use crate::network::NetworkWidget;
use crate::temperature::TemperatureWidget;
use crate::uptime::UptimeWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "sysinfo" => sysinfo_multi_widget => SysinfoMultiWidget => html,
    "cpu" => cpu_widget => CpuWidget => html,
    "memory" => memory_widget => MemoryWidget => html,
    "battery" => battery_widget => BatteryWidget => html,
    "disks" => disks_widget => DisksWidget => html,
    "network" => network_widget => NetworkWidget => html,
    "temperature" => temperature_widget => TemperatureWidget => html,
    "uptime" => uptime_widget => UptimeWidget => html,
    "sysinfo_cpu" => sysinfo_cpu_widget => SysinfoAtomicWidget,
    "sysinfo_cpu_temperature" => sysinfo_cpu_temperature_widget => SysinfoAtomicWidget,
    "sysinfo_memory" => sysinfo_memory_widget => SysinfoAtomicWidget,
    "sysinfo_battery" => sysinfo_battery_widget => SysinfoAtomicWidget,
    "sysinfo_disk" => sysinfo_disk_widget => SysinfoAtomicWidget,
    "sysinfo_network_download" => sysinfo_network_download_widget => SysinfoAtomicWidget,
    "sysinfo_network_upload" => sysinfo_network_upload_widget => SysinfoAtomicWidget,
    "sysinfo_uptime" => sysinfo_uptime_widget => SysinfoAtomicWidget,
    "sysinfo_load" => sysinfo_load_widget => SysinfoAtomicWidget,
}
