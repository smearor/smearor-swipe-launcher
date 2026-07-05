pub mod config;
pub mod shared;
pub mod widget_battery;
pub mod widget_cpu;
pub mod widget_disks;
pub mod widget_memory;
pub mod widget_network;
pub mod widget_uptime;

use crate::widget_battery::BatteryWidget;
use crate::widget_cpu::CpuWidget;
use crate::widget_disks::DisksWidget;
use crate::widget_memory::MemoryWidget;
use crate::widget_network::NetworkWidget;
use crate::widget_uptime::UptimeWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin;

widget_factory_plugin! {
    "cpu" => CpuWidget,
    "memory" => MemoryWidget,
    "battery" => BatteryWidget,
    "disks" => DisksWidget,
    "network" => NetworkWidget,
    "uptime" => UptimeWidget,
}
