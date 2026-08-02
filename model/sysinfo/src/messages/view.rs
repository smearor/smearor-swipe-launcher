use serde::Deserialize;
use serde::Serialize;

/// Available sysinfo views that the multi-view widget can display.
///
/// Each variant corresponds to a system metric rendered in the widget tile.
/// The order of variants defines the default cycling order when no explicit
/// `views` list is configured.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum SysinfoView {
    /// CPU usage percentage.
    #[default]
    Cpu,
    /// CPU temperature.
    CpuTemperature,
    /// Memory usage percentage.
    Memory,
    /// Battery level percentage and charging state.
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
