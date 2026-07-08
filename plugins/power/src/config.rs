use serde::Deserialize;
use serde_json::Value;
use typed_builder::TypedBuilder;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_ICON_SIZE: i32 = 36;

pub const DEFAULT_BUTTON_SIZE: i32 = 48;

/// Configuration for the power menu widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct PowerWidgetConfig {
    /// Width of the widget in pixels.
    #[serde(default = "default_width")]
    pub(crate) width: i32,
    /// Spacing between buttons.
    #[builder(default = 8)]
    pub(crate) spacing: i32,
    /// Whether to show the shutdown button.
    #[builder(default = true)]
    pub(crate) show_shutdown: bool,
    /// Whether to show the reboot button.
    #[builder(default = true)]
    pub(crate) show_reboot: bool,
    /// Whether to show the suspend button.
    #[builder(default = true)]
    pub(crate) show_suspend: bool,
    /// Whether to show the hibernate button.
    #[builder(default = true)]
    pub(crate) show_hibernate: bool,
    /// Whether to show the lock screen button.
    #[builder(default = true)]
    pub(crate) show_lock: bool,
    /// Whether to show the logout button.
    #[builder(default = true)]
    pub(crate) show_logout: bool,
    /// Whether to show the reboot-to-firmware button.
    #[builder(default = true)]
    pub(crate) show_reboot_to_firmware: bool,
    /// Whether to show inhibitor warnings.
    #[builder(default = true)]
    pub(crate) show_inhibitor_warnings: bool,
    /// Whether to show the countdown overlay.
    #[builder(default = true)]
    pub(crate) show_countdown_overlay: bool,
    /// Whether to show the scheduled action status.
    #[builder(default = true)]
    pub(crate) show_scheduled_status: bool,
    /// Button size in pixels.
    #[serde(default = "default_icon_size")]
    pub(crate) button_size: i32,
    /// Icon size in pixels.
    #[serde(default = "default_icon_size")]
    pub(crate) icon_size: i32,
    /// Which power action to select on startup.
    /// One of: "shutdown", "reboot", "suspend", "hibernate", "lock", "logout", "reboot_to_firmware".
    /// Defaults to the first enabled action.
    #[serde(default)]
    pub default_action: Option<String>,
    /// Message topic for single-click (opens the power menu area).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
    /// Target instance for single-click message
    #[serde(default)]
    pub click_instance: Option<String>,
    /// Message topic for long-press
    #[serde(default)]
    pub longpress_topic: Option<String>,
    /// Message payload for long-press (JSON/TOML)
    #[serde(default)]
    pub longpress_payload: Option<Value>,
    /// Target instance for long-press message
    #[serde(default)]
    pub longpress_instance: Option<String>,
}

impl Default for PowerWidgetConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            spacing: 0,
            show_shutdown: true,
            show_reboot: true,
            show_suspend: true,
            show_hibernate: true,
            show_lock: true,
            show_logout: true,
            show_reboot_to_firmware: true,
            show_inhibitor_warnings: true,
            show_countdown_overlay: true,
            show_scheduled_status: true,
            button_size: DEFAULT_BUTTON_SIZE,
            icon_size: DEFAULT_ICON_SIZE,
            click_topic: None,
            click_payload: None,
            click_instance: None,
            longpress_topic: None,
            longpress_payload: None,
            default_action: None,
            longpress_instance: None,
        }
    }
}

fn default_width() -> i32 {
    DEFAULT_WIDTH
}

fn default_icon_size() -> i32 {
    DEFAULT_ICON_SIZE
}

fn default_button_size() -> i32 {
    DEFAULT_BUTTON_SIZE
}
