use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMetadata;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use typed_builder::TypedBuilder;

pub const DEFAULT_ICON_THEME: &str = "nf-md-palette";
pub const DEFAULT_ICON_THEME_DARK: &str = "nf-md-weather_night";
pub const DEFAULT_ICON_THEME_LIGHT: &str = "nf-md-weather_sunny";
pub const DEFAULT_ICON_THEME_SYSTEM: &str = "nf-md-theme_light_dark";
pub const DEFAULT_ICON_NO_THEME: &str = "nf-md-palette_outline";

fn default_icon_theme() -> String {
    DEFAULT_ICON_THEME.to_string()
}

fn default_icon_theme_dark() -> String {
    DEFAULT_ICON_THEME_DARK.to_string()
}

fn default_icon_theme_light() -> String {
    DEFAULT_ICON_THEME_LIGHT.to_string()
}

fn default_icon_theme_system() -> String {
    DEFAULT_ICON_THEME_SYSTEM.to_string()
}

fn default_icon_no_theme() -> String {
    DEFAULT_ICON_NO_THEME.to_string()
}

/// Theme-specific icon configuration.
/// All Nerd Font icon names used by the Theme widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
#[allow(dead_code)]
pub struct ThemeIcons {
    /// Icon for a generic theme.
    #[builder(default = DEFAULT_ICON_THEME.to_string())]
    #[serde(default = "default_icon_theme")]
    pub(crate) icon_theme: String,

    /// Icon for dark mode theme.
    #[builder(default = DEFAULT_ICON_THEME_DARK.to_string())]
    #[serde(default = "default_icon_theme_dark")]
    pub(crate) icon_theme_dark: String,

    /// Icon for light mode theme.
    #[builder(default = DEFAULT_ICON_THEME_LIGHT.to_string())]
    #[serde(default = "default_icon_theme_light")]
    pub(crate) icon_theme_light: String,

    /// Icon for system mode theme.
    #[builder(default = DEFAULT_ICON_THEME_SYSTEM.to_string())]
    #[serde(default = "default_icon_theme_system")]
    pub(crate) icon_theme_system: String,

    /// Icon when no theme is applied.
    #[builder(default = DEFAULT_ICON_NO_THEME.to_string())]
    #[serde(default = "default_icon_no_theme")]
    pub(crate) icon_no_theme: String,
}

impl Default for ThemeIcons {
    fn default() -> Self {
        Self {
            icon_theme: DEFAULT_ICON_THEME.to_string(),
            icon_theme_dark: DEFAULT_ICON_THEME_DARK.to_string(),
            icon_theme_light: DEFAULT_ICON_THEME_LIGHT.to_string(),
            icon_theme_system: DEFAULT_ICON_THEME_SYSTEM.to_string(),
            icon_no_theme: DEFAULT_ICON_NO_THEME.to_string(),
        }
    }
}

/// Configuration for the Theme widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct ThemeWidgetConfig {
    /// Shared widget dimensions (width, height, max_width, scale).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,

    /// Shared widget layout (spacing, css_classes).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,

    /// Shared widget icon settings (icon_size, icon_only, icon_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon: WidgetIcon,

    /// Shared widget text colors (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,

    /// Shared widget mode (compact, wide).
    #[serde(default)]
    #[builder(default)]
    pub(crate) mode: WidgetMode,

    /// Action bindings for click, longpress, drag, and scroll gestures.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) actions: ActionBindings,

    /// Widget metadata (description for MCP tool registration).
    #[serde(flatten)]
    #[builder(default)]
    pub metadata: WidgetMetadata,

    /// Theme-specific Nerd Font icons.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icons: ThemeIcons,
}

impl ThemeWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for ThemeWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            actions: ActionBindings::default(),
            metadata: WidgetMetadata::default(),
            icons: ThemeIcons::default(),
        }
    }
}
