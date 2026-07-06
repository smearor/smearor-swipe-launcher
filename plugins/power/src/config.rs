use serde::Deserialize;
use serde_json::Value;
use typed_builder::TypedBuilder;

/// Configuration for the power menu widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct PowerWidgetConfig {
    /// Width of the widget in pixels.
    #[builder(default = 200)]
    pub(crate) width: i32,
    /// Height of the widget in pixels.
    #[builder(default = 240)]
    pub(crate) height: i32,
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
    #[builder(default = 48)]
    pub(crate) button_size: i32,
    /// Icon size in pixels.
    #[builder(default = 24)]
    pub(crate) icon_size: i32,
    /// Message topic for single-click (opens the power menu area).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
}

impl Default for PowerWidgetConfig {
    fn default() -> Self {
        Self {
            width: 200,
            height: 240,
            spacing: 8,
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
            button_size: 48,
            icon_size: 24,
            click_topic: None,
            click_payload: None,
        }
    }
}
