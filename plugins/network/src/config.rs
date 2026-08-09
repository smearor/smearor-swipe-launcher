use serde::Deserialize;
use smearor_network_model::NetworkView;
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

    /// Widget metadata (description for MCP tool registration).
    #[serde(flatten)]
    #[builder(default)]
    pub metadata: WidgetMetadata,

    /// Action bindings for all input triggers.
    #[serde(flatten)]
    #[builder(default)]
    pub actions: ActionBindings,
}

impl NetworkWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
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
            metadata: WidgetMetadata::default(),
            actions: ActionBindings::default(),
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
