use serde::Deserialize;
use serde::Serialize;

/// Available network views that the widget can display.
/// Each variant corresponds to a data category rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum NetworkView {
    /// WiFi status: SSID, signal strength, IP address.
    #[default]
    WifiStatus,
    /// Ethernet status: connection state, IP address.
    EthernetStatus,
    /// Aggregate throughput: download and upload rates.
    Throughput,
    /// WiFi scan summary: network count and strongest signal.
    WifiScan,
    /// VPN status: first profile name and active state.
    Vpn,
    /// Airplane mode: on/off state.
    Airplane,
    /// QR code for sharing WiFi credentials.
    QrCode,
}
