use serde::Deserialize;

/// Configuration for the sysinfo service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SysinfoServiceConfig {
    /// Query interval for metrics in milliseconds.
    pub(crate) update_interval_ms: u64,
    /// Whether CPU temperature should be collected.
    pub(crate) enable_cpu_temperature: bool,
    /// Optional source for CPU temperature.
    ///
    /// Supported values:
    /// - `None` or `"thermal_zone"`: scan `/sys/class/thermal/thermal_zone*/temp`.
    /// - `"hwmon"`: scan `/sys/class/hwmon/hwmon*/temp*_input`.
    /// - Any absolute path (e.g. `"/sys/class/hwmon/hwmon0/temp1_input"`) is read directly.
    pub(crate) cpu_temperature_source: Option<String>,
    /// Whether battery metrics should be collected.
    pub(crate) enable_battery: bool,
    /// Whether disk metrics should be collected.
    pub(crate) enable_disks: bool,
    /// Whether network metrics should be collected.
    pub(crate) enable_network: bool,
}

impl Default for SysinfoServiceConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 5000,
            enable_cpu_temperature: true,
            cpu_temperature_source: None,
            enable_battery: true,
            enable_disks: true,
            enable_network: true,
        }
    }
}
