use serde::Deserialize;
use serde_json::Value;
use smearor_network_model::NetworkView;
use typed_builder::TypedBuilder;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_HEIGHT: i32 = 100;

pub const DEFAULT_SPACING: i32 = 0;

pub const DEFAULT_BUTTON_SIZE: i32 = 48;

pub const DEFAULT_ICON_SIZE: i32 = 36;

pub const DEFAULT_ICON_WIFI_STRENGTH_4: &str = "nf-md-wifi_strength_4";

pub const DEFAULT_ICON_WIFI_STRENGTH_3: &str = "nf-md-wifi_strength_3";

pub const DEFAULT_ICON_WIFI_STRENGTH_2: &str = "nf-md-wifi_strength_2";

pub const DEFAULT_ICON_WIFI_STRENGTH_1: &str = "nf-md-wifi_strength_1";

pub const DEFAULT_ICON_WIFI_STRENGTH_OFF: &str = "nf-md-wifi_strength_off";

pub const DEFAULT_ICON_ETHERNET_ON: &str = "nf-md-network_outline";

pub const DEFAULT_ICON_ETHERNET_OFF: &str = "nf-md-network_off";

pub const DEFAULT_ICON_VPN_ON: &str = "nf-md-shield_key";

pub const DEFAULT_ICON_VPN_OFF: &str = "nf-md-shield_off";

pub const DEFAULT_ICON_AIRPLANE_ON: &str = "nf-md-airplane";

pub const DEFAULT_ICON_AIRPLANE_OFF: &str = "nf-md-airplane_off";

pub const DEFAULT_ICON_THROUGHPUT: &str = "nf-md-swap_vertical";

pub const DEFAULT_ICON_WIFI_SCAN: &str = "nf-md-wifi_strength_4";

pub const DEFAULT_ICON_QR_CODE: &str = "nf-md-qrcode";

/// Configuration for the network menu widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Width of the widget in pixels.
    #[builder(default = DEFAULT_WIDTH)]
    pub(crate) width: i32,

    /// Height of the widget in pixels.
    #[builder(default = DEFAULT_HEIGHT)]
    pub(crate) height: i32,

    /// Spacing between elements (icon, value, info labels) in pixels.
    #[builder(default = DEFAULT_SPACING)]
    pub(crate) spacing: i32,

    /// Button size in pixels (used for touch target sizing).
    #[builder(default = DEFAULT_BUTTON_SIZE)]
    pub(crate) button_size: i32,

    /// Icon size in pixels (pixel size for Nerd Font icon images).
    #[builder(default = DEFAULT_ICON_SIZE)]
    pub(crate) icon_size: i32,

    /// WiFi icon: signal strength 4 (>75%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_4.to_string())]
    #[serde(default = "default_icon_wifi_strength_4")]
    pub(crate) icon_wifi_strength_4: String,

    /// WiFi icon: signal strength 3 (>50%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_3.to_string())]
    #[serde(default = "default_icon_wifi_strength_3")]
    pub(crate) icon_wifi_strength_3: String,

    /// WiFi icon: signal strength 2 (>25%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_2.to_string())]
    #[serde(default = "default_icon_wifi_strength_2")]
    pub(crate) icon_wifi_strength_2: String,

    /// WiFi icon: signal strength 1 (>0%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_1.to_string())]
    #[serde(default = "default_icon_wifi_strength_1")]
    pub(crate) icon_wifi_strength_1: String,

    /// WiFi icon: WiFi off / no signal.
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_OFF.to_string())]
    #[serde(default = "default_icon_wifi_strength_off")]
    pub(crate) icon_wifi_strength_off: String,

    /// Ethernet icon: connected.
    #[builder(default = DEFAULT_ICON_ETHERNET_ON.to_string())]
    #[serde(default = "default_icon_ethernet_on")]
    pub(crate) icon_ethernet_on: String,

    /// Ethernet icon: disconnected.
    #[builder(default = DEFAULT_ICON_ETHERNET_OFF.to_string())]
    #[serde(default = "default_icon_ethernet_off")]
    pub(crate) icon_ethernet_off: String,

    /// VPN icon: active.
    #[builder(default = DEFAULT_ICON_VPN_ON.to_string())]
    #[serde(default = "default_icon_vpn_on")]
    pub(crate) icon_vpn_on: String,

    /// VPN icon: inactive.
    #[builder(default = DEFAULT_ICON_VPN_OFF.to_string())]
    #[serde(default = "default_icon_vpn_off")]
    pub(crate) icon_vpn_off: String,

    /// Airplane icon: airplane mode on.
    #[builder(default = DEFAULT_ICON_AIRPLANE_ON.to_string())]
    #[serde(default = "default_icon_airplane_on")]
    pub(crate) icon_airplane_on: String,

    /// Airplane icon: airplane mode off.
    #[builder(default = DEFAULT_ICON_AIRPLANE_OFF.to_string())]
    #[serde(default = "default_icon_airplane_off")]
    pub(crate) icon_airplane_off: String,

    /// Throughput view icon.
    #[builder(default = DEFAULT_ICON_THROUGHPUT.to_string())]
    #[serde(default = "default_icon_throughput")]
    pub(crate) icon_throughput: String,

    /// WiFi scan view icon.
    #[builder(default = DEFAULT_ICON_WIFI_SCAN.to_string())]
    #[serde(default = "default_icon_wifi_scan")]
    pub(crate) icon_wifi_scan: String,

    /// QR code view icon.
    #[builder(default = DEFAULT_ICON_QR_CODE.to_string())]
    #[serde(default = "default_icon_qr_code")]
    pub(crate) icon_qr_code: String,

    /// Views to cycle through on swipe up/down.
    #[builder(default)]
    pub(crate) views: Vec<NetworkView>,

    /// Maximum number of access points to summarize in the WifiScan view.
    #[builder(default = 10)]
    pub(crate) max_access_points: usize,

    /// Message topic for single-click (opens the network menu area).
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Target instance for single-click message.
    #[serde(default)]
    pub click_instance: Option<String>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,

    /// Target instance for long-press message.
    #[serde(default)]
    pub longpress_instance: Option<String>,
}

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            spacing: DEFAULT_SPACING,
            button_size: DEFAULT_BUTTON_SIZE,
            icon_size: DEFAULT_ICON_SIZE,
            icon_wifi_strength_4: DEFAULT_ICON_WIFI_STRENGTH_4.to_string(),
            icon_wifi_strength_3: DEFAULT_ICON_WIFI_STRENGTH_3.to_string(),
            icon_wifi_strength_2: DEFAULT_ICON_WIFI_STRENGTH_2.to_string(),
            icon_wifi_strength_1: DEFAULT_ICON_WIFI_STRENGTH_1.to_string(),
            icon_wifi_strength_off: DEFAULT_ICON_WIFI_STRENGTH_OFF.to_string(),
            icon_ethernet_on: DEFAULT_ICON_ETHERNET_ON.to_string(),
            icon_ethernet_off: DEFAULT_ICON_ETHERNET_OFF.to_string(),
            icon_vpn_on: DEFAULT_ICON_VPN_ON.to_string(),
            icon_vpn_off: DEFAULT_ICON_VPN_OFF.to_string(),
            icon_airplane_on: DEFAULT_ICON_AIRPLANE_ON.to_string(),
            icon_airplane_off: DEFAULT_ICON_AIRPLANE_OFF.to_string(),
            icon_throughput: DEFAULT_ICON_THROUGHPUT.to_string(),
            icon_wifi_scan: DEFAULT_ICON_WIFI_SCAN.to_string(),
            icon_qr_code: DEFAULT_ICON_QR_CODE.to_string(),
            views: vec![
                NetworkView::WifiStatus,
                NetworkView::EthernetStatus,
                NetworkView::Throughput,
                NetworkView::WifiScan,
                NetworkView::Vpn,
                NetworkView::Airplane,
                NetworkView::QrCode,
            ],
            max_access_points: 10,
            click_topic: None,
            click_payload: None,
            click_instance: None,
            longpress_topic: None,
            longpress_payload: None,
            longpress_instance: None,
        }
    }
}

fn default_icon_wifi_strength_4() -> String {
    DEFAULT_ICON_WIFI_STRENGTH_4.to_string()
}

fn default_icon_wifi_strength_3() -> String {
    DEFAULT_ICON_WIFI_STRENGTH_3.to_string()
}

fn default_icon_wifi_strength_2() -> String {
    DEFAULT_ICON_WIFI_STRENGTH_2.to_string()
}

fn default_icon_wifi_strength_1() -> String {
    DEFAULT_ICON_WIFI_STRENGTH_1.to_string()
}

fn default_icon_wifi_strength_off() -> String {
    DEFAULT_ICON_WIFI_STRENGTH_OFF.to_string()
}

fn default_icon_ethernet_on() -> String {
    DEFAULT_ICON_ETHERNET_ON.to_string()
}

fn default_icon_ethernet_off() -> String {
    DEFAULT_ICON_ETHERNET_OFF.to_string()
}

fn default_icon_vpn_on() -> String {
    DEFAULT_ICON_VPN_ON.to_string()
}

fn default_icon_vpn_off() -> String {
    DEFAULT_ICON_VPN_OFF.to_string()
}

fn default_icon_airplane_on() -> String {
    DEFAULT_ICON_AIRPLANE_ON.to_string()
}

fn default_icon_airplane_off() -> String {
    DEFAULT_ICON_AIRPLANE_OFF.to_string()
}

fn default_icon_throughput() -> String {
    DEFAULT_ICON_THROUGHPUT.to_string()
}

fn default_icon_wifi_scan() -> String {
    DEFAULT_ICON_WIFI_SCAN.to_string()
}

fn default_icon_qr_code() -> String {
    DEFAULT_ICON_QR_CODE.to_string()
}
