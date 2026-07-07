use crate::WifiSecurity;

/// Information about a single WLAN access point found during a scan.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessPointInfo {
    /// SSID (network name) of the access point.
    pub ssid: stabby::string::String,
    /// BSSID (MAC address) of the access point.
    pub bssid: stabby::string::String,
    /// Signal strength in percent (0 - 100).
    pub signal: u8,
    /// Frequency in MHz.
    pub frequency: u32,
    /// Encryption type of the access point.
    pub security: WifiSecurity,
    /// Whether this is the currently connected access point.
    pub is_connected: bool,
    /// Whether this network is saved (known) in NetworkManager.
    pub is_known: bool,
}
