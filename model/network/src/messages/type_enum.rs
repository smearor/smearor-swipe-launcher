/// Type of a network interface as reported by NetworkManager.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetworkInterfaceType {
    /// Ethernet (wired) connection.
    Ethernet,
    /// Wireless LAN (Wi-Fi) connection.
    #[default]
    Wifi,
    /// Bluetooth tethering connection.
    Bluetooth,
    /// VPN tunnel (WireGuard, OpenVPN, etc.).
    Vpn,
    /// Unknown or other interface type.
    Other,
}
