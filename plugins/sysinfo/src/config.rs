use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use smearor_sysinfo_model::SysinfoView;

pub use smearor_swipe_launcher_plugin_api::DEFAULT_ICON_SIZE;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetLayout;

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
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
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
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
}

impl Default for PercentageWidgetConfig {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Bar,
            bar_orientation: BarOrientation::Horizontal,
            show_value: true,
            show_icon: true,
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon: None,
            icon_size: DEFAULT_ICON_SIZE,
            value_format: String::from("{value:.0}%"),
            warning_threshold: 70.0,
            critical_threshold: 90.0,
            text_colors: WidgetTextColors::default(),
        }
    }
}

/// Configuration for the sysinfo multi-view widget.
///
/// Cycles through configurable system metric views via swipe gestures.
/// Follows the Unified 4-Line Layout with icon, value, label, and optional progress bar.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SysinfoMultiWidgetConfig {
    /// Widget dimensions (width, height, max_width) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Icon configuration (size, color, icon_only, mode).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Layout mode: compact (vertical) or wide (horizontal).
    pub mode: WidgetMode,
    /// Views to cycle through on swipe up/down.
    pub views: Vec<SysinfoView>,
    /// Action bindings for gestures.
    #[serde(flatten)]
    pub actions: ActionBindings,
    /// Optional description for MCP tool registration.
    pub description: Option<String>,
}

impl Default for SysinfoMultiWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            views: vec![
                SysinfoView::Cpu,
                SysinfoView::CpuTemperature,
                SysinfoView::Memory,
                SysinfoView::Battery,
                SysinfoView::Disk,
                SysinfoView::NetworkDownload,
                SysinfoView::NetworkUpload,
                SysinfoView::Uptime,
                SysinfoView::Load,
            ],
            actions: ActionBindings::default(),
            description: None,
        }
    }
}

impl SysinfoMultiWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: smearor_swipe_launcher_plugin_api::ActionKind) -> &dyn smearor_swipe_launcher_plugin_api::DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
