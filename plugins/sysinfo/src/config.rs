use serde::Deserialize;

pub const DEFAULT_ICON_SIZE: i32 = 36;

/// Visual representation for percentage-based widgets.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize)]
pub enum DisplayMode {
    /// Horizontal or vertical progress bar.
    #[default]
    Bar,
    /// Circular or semicircular gauge.
    Gauge,
}

/// Orientation of a bar display.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize)]
pub enum BarOrientation {
    /// Horizontal bar.
    #[default]
    Horizontal,
    /// Vertical bar.
    Vertical,
}

/// Display mode for the disks widget.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub enum DiskDisplayMode {
    /// Show the root mount point only.
    #[default]
    RootOnly,
    /// Show a list of configured mount points.
    List,
    /// Show a circular gauge for the first configured mount point.
    Gauge,
}

/// Configuration shared by percentage-based widgets.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PercentageWidgetConfig {
    /// Visual display mode.
    pub display_mode: DisplayMode,
    /// Orientation when display_mode is Bar.
    pub bar_orientation: BarOrientation,
    /// Whether to show the numeric value as text.
    pub show_value: bool,
    /// Whether to show an icon next to the value.
    pub show_icon: bool,
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
    /// Format string for the numeric label.
    pub value_format: String,
    /// Color threshold for warning state.
    pub warning_threshold: f32,
    /// Color threshold for critical state.
    pub critical_threshold: f32,
}

impl Default for PercentageWidgetConfig {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Bar,
            bar_orientation: BarOrientation::Horizontal,
            show_value: true,
            show_icon: true,
            width: 120,
            height: 40,
            icon: None,
            icon_size: DEFAULT_ICON_SIZE,
            value_format: String::from("{value:.0}%"),
            warning_threshold: 70.0,
            critical_threshold: 90.0,
        }
    }
}

/// Configuration for the CPU widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CpuWidgetConfig {
    /// Shared percentage widget configuration.
    #[serde(flatten)]
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the CPU temperature.
    pub show_temperature: bool,
    /// Format string for the temperature label.
    pub temperature_format: String,
    /// Optional filter to select which temperature component to display.
    ///
    /// Matched against component label and id (case-insensitive substring match).
    /// If `None`, the primary `cpu_temperature` from the service is used.
    /// Example values: `"k10temp"`, `"Tctl"`, `"thermal_zone0"`, `"hwmon0_1"`.
    pub temperature_component: Option<String>,
}

impl Default for CpuWidgetConfig {
    fn default() -> Self {
        Self {
            percentage: PercentageWidgetConfig {
                icon: Some(String::from("nf-fae-chip")),
                value_format: String::from("{cpu_usage:.0}%"),
                ..Default::default()
            },
            show_temperature: true,
            temperature_format: String::from("{cpu_temperature:.0}°C"),
            temperature_component: None,
        }
    }
}

/// Configuration for the memory widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MemoryWidgetConfig {
    /// Shared percentage widget configuration.
    #[serde(flatten)]
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the absolute used memory in bytes.
    pub show_used_bytes: bool,
    /// Whether to display the available memory in bytes.
    pub show_available_bytes: bool,
}

impl Default for MemoryWidgetConfig {
    fn default() -> Self {
        Self {
            percentage: PercentageWidgetConfig {
                icon: Some(String::from("nf-md-memory")),
                value_format: String::from("{memory_usage:.0}%"),
                ..Default::default()
            },
            show_used_bytes: false,
            show_available_bytes: false,
        }
    }
}

/// Configuration for the battery widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BatteryWidgetConfig {
    /// Shared percentage widget configuration.
    #[serde(flatten)]
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the charging status as text.
    pub show_status_text: bool,
    /// Whether to change the icon based on charging status.
    pub animate_icon: bool,
}

impl Default for BatteryWidgetConfig {
    fn default() -> Self {
        Self {
            percentage: PercentageWidgetConfig {
                icon: Some(String::from("nf-md-battery")),
                value_format: String::from("{level:.0}%"),
                ..Default::default()
            },
            show_status_text: true,
            animate_icon: false,
        }
    }
}

/// Configuration for the disks widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DisksWidgetConfig {
    /// Maximum number of mount points to display.
    pub max_mount_points: usize,
    /// Mount points to display (empty means all).
    pub include_mount_points: Vec<String>,
    /// Whether to show read/write throughput.
    pub show_throughput: bool,
    /// Whether to display the widget as a list or a single bar for the root mount.
    pub display_mode: DiskDisplayMode,
    /// Whether to show an icon.
    pub show_icon: bool,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
}

impl Default for DisksWidgetConfig {
    fn default() -> Self {
        Self {
            max_mount_points: 5,
            include_mount_points: Vec::new(),
            show_throughput: false,
            display_mode: DiskDisplayMode::RootOnly,
            show_icon: true,
            icon: Some(String::from("nf-md-harddisk")),
            icon_size: DEFAULT_ICON_SIZE,
        }
    }
}

/// Display mode for the network widget.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub enum NetworkDisplayMode {
    /// Show textual up/down values.
    #[default]
    Info,
    /// Show a circular gauge with two semicircles for up/down.
    Gauge,
}

/// Configuration for the network widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Visual display mode.
    pub display_mode: NetworkDisplayMode,
    /// Whether to show received bytes per second.
    pub show_received: bool,
    /// Whether to show transmitted bytes per second.
    pub show_transmitted: bool,
    /// Maximum expected received bytes per second for the gauge (0 means auto).
    pub max_download: f64,
    /// Maximum expected transmitted bytes per second for the gauge (0 means auto).
    pub max_upload: f64,
    /// Whether to show a small sparkline history.
    pub show_history: bool,
    /// Number of history samples to keep.
    pub history_length: usize,
    /// Whether to show an icon.
    pub show_icon: bool,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
}

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            display_mode: NetworkDisplayMode::Info,
            show_received: true,
            show_transmitted: true,
            max_download: 0.0,
            max_upload: 0.0,
            show_history: false,
            history_length: 30,
            show_icon: true,
            icon: Some(String::from("nf-md-network")),
            icon_size: DEFAULT_ICON_SIZE,
        }
    }
}

/// Configuration for the temperature widget.
///
/// Displays one or more temperature components as circular gauges.
/// Each gauge shows current temperature, max marker, and critical threshold.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TemperatureWidgetConfig {
    /// Filter list for which temperature components to display.
    ///
    /// Each entry is matched against component label and id (case-insensitive substring).
    /// If empty, all available components are shown.
    pub components: Vec<String>,
    /// Format string for the temperature label.
    pub format: String,
    /// Whether to show the component label text.
    pub show_label: bool,
    /// Gauge diameter in pixels.
    pub gauge_size: i32,
    /// Whether to show an icon.
    pub show_icon: bool,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
}

impl Default for TemperatureWidgetConfig {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            format: String::from("{temperature:.0}°C"),
            show_label: true,
            gauge_size: 120,
            show_icon: true,
            icon: Some(String::from("nf-md-thermometer")),
            icon_size: DEFAULT_ICON_SIZE,
        }
    }
}

/// Display mode for the uptime widget.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub enum UptimeDisplayMode {
    /// Show textual uptime and load average.
    #[default]
    Info,
    /// Show a circular gauge with the uptime in the center.
    Gauge,
}

/// Configuration for the uptime widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct UptimeWidgetConfig {
    /// Visual display mode.
    pub display_mode: UptimeDisplayMode,
    /// Format string for the uptime label.
    pub value_format: String,
    /// Whether to show the uptime as a human-readable duration.
    pub show_uptime: bool,
    /// Whether to show the 1-minute load average.
    pub show_load_average_1_minute: bool,
    /// Whether to show the 5-minute load average.
    pub show_load_average_5_minute: bool,
    /// Whether to show the 15-minute load average.
    pub show_load_average_15_minute: bool,
    /// Whether to show an icon.
    pub show_icon: bool,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
}

impl Default for UptimeWidgetConfig {
    fn default() -> Self {
        Self {
            display_mode: UptimeDisplayMode::Info,
            value_format: String::from("{value}"),
            show_uptime: true,
            show_load_average_1_minute: true,
            show_load_average_5_minute: true,
            show_load_average_15_minute: true,
            show_icon: true,
            icon: Some(String::from("nf-md-clock_start")),
            icon_size: DEFAULT_ICON_SIZE,
        }
    }
}
