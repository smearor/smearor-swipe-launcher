use serde::Deserialize;
use serde_json::Value;
use typed_builder::TypedBuilder;

/// Configuration for the network menu widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Width of the widget in pixels.
    #[builder(default = 300)]
    pub(crate) width: i32,
    /// Height of the widget in pixels.
    #[builder(default = 400)]
    pub(crate) height: i32,
    /// Spacing between elements.
    #[builder(default = 8)]
    pub(crate) spacing: i32,
    /// Whether to show the status overview section.
    #[builder(default = true)]
    pub(crate) show_status: bool,
    /// Whether to show the WLAN scan list.
    #[builder(default = true)]
    pub(crate) show_scan_list: bool,
    /// Whether to show the airplane mode toggle.
    #[builder(default = true)]
    pub(crate) show_airplane_toggle: bool,
    /// Whether to show the VPN toggle list.
    #[builder(default = true)]
    pub(crate) show_vpn_toggles: bool,
    /// Whether to show the throughput sparkline.
    #[builder(default = true)]
    pub(crate) show_throughput: bool,
    /// Whether to show the QR code generator button.
    #[builder(default = true)]
    pub(crate) show_qr_code: bool,
    /// Button size in pixels.
    #[builder(default = 48)]
    pub(crate) button_size: i32,
    /// Icon size in pixels.
    #[builder(default = 24)]
    pub(crate) icon_size: i32,
    /// Maximum number of access points to display in the scan list.
    #[builder(default = 10)]
    pub(crate) max_access_points: usize,
    /// Message topic for single-click (opens the network menu area).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
}

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            width: 300,
            height: 400,
            spacing: 8,
            show_status: true,
            show_scan_list: true,
            show_airplane_toggle: true,
            show_vpn_toggles: true,
            show_throughput: true,
            show_qr_code: true,
            button_size: 48,
            icon_size: 24,
            max_access_points: 10,
            click_topic: None,
            click_payload: None,
        }
    }
}
