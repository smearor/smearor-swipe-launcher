use crate::NetworkConnectionState;
use crate::NetworkInterfaceType;

/// Status of a single network interface.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InterfaceStatus {
    /// Interface type (Wifi, Ethernet, Bluetooth, Vpn, Other).
    pub interface_type: NetworkInterfaceType,
    /// Interface name (e.g., "wlan0", "eth0").
    pub interface_name: stabby::string::String,
    /// Current connection state.
    pub state: NetworkConnectionState,
    /// SSID of the connected WLAN (only for Wifi type).
    pub ssid: stabby::option::Option<stabby::string::String>,
    /// Signal strength in percent (0 - 100, only for Wifi type).
    pub signal: stabby::option::Option<u8>,
    /// IPv4 address of the interface.
    pub ipv4_address: stabby::option::Option<stabby::string::String>,
    /// IPv6 address of the interface.
    pub ipv6_address: stabby::option::Option<stabby::string::String>,
    /// Whether the interface has internet access.
    pub internet_accessible: bool,
}
