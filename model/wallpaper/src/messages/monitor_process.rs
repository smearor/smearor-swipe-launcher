/// Tracks the PID of a wallpaper process running on a specific monitor.
#[derive(Clone, Debug, Default)]
#[stabby::stabby]
pub struct MonitorProcess {
    /// The monitor name (e.g., "DP-1", "HDMI-A-1").
    pub monitor: stabby::string::String,
    /// The PID of the wallpaper process on this monitor.
    pub process_id: u32,
}
