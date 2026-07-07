/// Information about a VPN connection profile registered in NetworkManager.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VpnProfileInfo {
    /// Display name of the VPN profile.
    pub name: stabby::string::String,
    /// VPN type (e.g., "wireguard", "openvpn").
    pub vpn_type: stabby::string::String,
    /// Whether the VPN connection is currently active.
    pub is_active: bool,
    /// Connection UUID in NetworkManager.
    pub uuid: stabby::string::String,
}
