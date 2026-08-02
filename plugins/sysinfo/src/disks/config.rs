use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

use crate::config::DEFAULT_ICON_SIZE;

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
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
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
            layout: WidgetLayout::default(),
            text_colors: WidgetTextColors::default(),
            actions: ActionBindings::default(),
        }
    }
}

impl DisksWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
