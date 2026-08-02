use serde::Deserialize;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// Configuration for the workspace switcher widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WorkspaceSwitcherConfig {
    /// Widget dimensions (width, height, max_width) for GTK layout.
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
    pub(crate) mode: WidgetMode,

    /// Whether to show the workspace label (name or number).
    #[builder(default = true)]
    pub(crate) show_label: bool,

    /// Whether to show the workspace scrollbar indicator (position in workspace list).
    #[builder(default = true)]
    pub(crate) show_scrollbar: bool,

    /// Map of workspace IDs (as strings) to Nerd Font icon class names.
    /// Example: `{ "1" = "nf-md-numeric-1", "2" = "nf-md-numeric-2" }`
    #[builder(default)]
    pub(crate) icon_map: HashMap<String, String>,

    /// Default icon class name for workspaces not in `icon_map`.
    #[builder(default = "nf-md-monitor".to_string())]
    pub(crate) default_icon: String,

    /// Human-readable description of what the workspace switcher does.
    #[serde(default)]
    pub description: Option<String>,

    /// Action bindings for all input triggers.
    #[serde(flatten)]
    #[builder(default)]
    pub actions: ActionBindings,
}

impl Default for WorkspaceSwitcherConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            show_label: true,
            show_scrollbar: true,
            icon_map: HashMap::new(),
            default_icon: "nf-md-monitor".to_string(),
            description: None,
            actions: ActionBindings::default(),
        }
    }
}

impl WorkspaceSwitcherConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

/// Configuration for workspace atomic widget variants.
///
/// Extends `AtomicWidgetConfig` with workspace-specific fields like
/// `workspace_index` (for `WorkspaceSelect`), `icon`, `icon_map`, and
/// `default_icon`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WorkspaceAtomicConfig {
    /// Base atomic widget config (action bindings, description, render mode).
    #[serde(flatten)]
    pub atomic: AtomicWidgetConfig,

    /// For `WorkspaceSelect`: 0-based index of the workspace to switch to on click.
    pub workspace_index: Option<usize>,

    /// Custom icon (Nerd Font class name) for this widget.
    /// Overrides the default icon for the view.
    pub icon: Option<String>,

    /// Map of workspace IDs (as strings) to Nerd Font icon class names.
    /// Example: `{ "1" = "nf-md-numeric-1", "2" = "nf-md-numeric-2" }`
    pub icon_map: HashMap<String, String>,

    /// Default icon class name for workspaces not in `icon_map`.
    pub default_icon: Option<String>,

    /// Whether to show the workspace label (name or number).
    pub show_label: Option<bool>,
}

impl Default for WorkspaceAtomicConfig {
    fn default() -> Self {
        Self {
            atomic: AtomicWidgetConfig::default(),
            workspace_index: None,
            icon: None,
            icon_map: HashMap::new(),
            default_icon: None,
            show_label: None,
        }
    }
}
