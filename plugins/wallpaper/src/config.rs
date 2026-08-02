use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use typed_builder::TypedBuilder;

pub const DEFAULT_PREVIEW_WIDTH: i32 = 100;
pub const DEFAULT_PREVIEW_HEIGHT: i32 = 100;
pub const DEFAULT_FALLBACK_ICON: &str = "nf-md-wallpaper";

/// Configuration for the wallpaper widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WallpaperWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,

    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,

    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon_config: WidgetIcon,

    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,

    /// Widget layout mode (compact or wide).
    #[builder(default)]
    #[serde(default)]
    pub(crate) mode: WidgetMode,

    /// Whether to show the theme name as a label overlay.
    #[builder(default)]
    pub(crate) show_theme_name: bool,

    /// Whether to show the wallpaper type icon.
    #[builder(default)]
    pub(crate) show_type_icon: bool,

    /// Whether to show the running/stopped status indicator.
    #[builder(default)]
    pub(crate) show_status_indicator: bool,

    /// Preview image width in pixels.
    #[builder(default, setter(into))]
    pub(crate) preview_width: Option<i32>,

    /// Preview image height in pixels.
    #[builder(default, setter(into))]
    pub(crate) preview_height: Option<i32>,

    /// Fallback icon when no preview image is available.
    #[builder(default, setter(into))]
    #[serde(default = "default_fallback_icon")]
    pub(crate) fallback_icon: String,

    /// Action bindings for all input triggers.
    #[serde(flatten)]
    #[builder(default)]
    pub actions: ActionBindings,
}

impl WallpaperWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for WallpaperWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            show_theme_name: true,
            show_type_icon: true,
            show_status_indicator: true,
            preview_width: Some(DEFAULT_PREVIEW_WIDTH),
            preview_height: Some(DEFAULT_PREVIEW_HEIGHT),
            fallback_icon: DEFAULT_FALLBACK_ICON.to_string(),
            actions: ActionBindings::default(),
        }
    }
}

fn default_fallback_icon() -> String {
    DEFAULT_FALLBACK_ICON.to_string()
}
