use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use typed_builder::TypedBuilder;

pub const DEFAULT_WIDTH: i32 = 120;

pub const DEFAULT_HEIGHT: i32 = 80;

pub const DEFAULT_ICON_SIZE: i32 = 36;

/// Configuration for the workspace switcher widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WorkspaceSwitcherConfig {
    /// The width of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) width: Option<i32>,

    /// The height of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) height: Option<i32>,

    /// Whether to show the workspace label (name or number).
    #[builder(default = true)]
    pub(crate) show_label: bool,

    /// Whether to show the dot indicator (position in workspace list).
    #[builder(default = true)]
    pub(crate) show_dot_indicator: bool,

    /// Icon size in pixels for the workspace icon.
    #[builder(default = DEFAULT_ICON_SIZE)]
    #[serde(default = "default_icon_size")]
    pub(crate) icon_size: i32,

    /// Map of workspace IDs (as strings) to Nerd Font icon class names.
    /// Example: `{ "1" = "nf-md-numeric-1", "2" = "nf-md-numeric-2" }`
    #[builder(default)]
    pub(crate) icon_map: HashMap<String, String>,

    /// Default icon class name for workspaces not in `icon_map`.
    #[builder(default = "nf-md-monitor".to_string())]
    pub(crate) default_icon: String,

    /// Message topic for single-click action.
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click action (JSON/TOML).
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,
}

impl Default for WorkspaceSwitcherConfig {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDTH),
            height: Some(DEFAULT_HEIGHT),
            show_label: true,
            show_dot_indicator: true,
            icon_size: DEFAULT_ICON_SIZE,
            icon_map: HashMap::new(),
            default_icon: "nf-md-monitor".to_string(),
            click_topic: None,
            click_payload: None,
            longpress_topic: None,
            longpress_payload: None,
        }
    }
}

fn default_icon_size() -> i32 {
    DEFAULT_ICON_SIZE
}
