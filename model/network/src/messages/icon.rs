use crate::NetworkInterfaceType;
use crate::WifiSecurity;

/// Returns the Nerd Font icon name for the WLAN signal strength.
pub fn wifi_signal_icon(signal: u8) -> &'static str {
    match signal {
        0..=20 => "nf-md-wifi_strength_outline",
        21..=40 => "nf-md-wifi_strength_1",
        41..=60 => "nf-md-wifi_strength_2",
        61..=80 => "nf-md-wifi_strength_3",
        _ => "nf-md-wifi",
    }
}

/// Returns the Nerd Font Unicode codepoint for the WLAN signal strength.
pub fn wifi_signal_icon_unicode(signal: u8) -> &'static str {
    match signal {
        0..=20 => "\u{f0929}",
        21..=40 => "\u{f0921}",
        41..=60 => "\u{f0922}",
        61..=80 => "\u{f0923}",
        _ => "\u{f0928}",
    }
}

/// Returns the Nerd Font icon name for a network interface type.
pub fn network_interface_icon(interface_type: &NetworkInterfaceType) -> &'static str {
    match interface_type {
        NetworkInterfaceType::Ethernet => "nf-md-ethernet",
        NetworkInterfaceType::Wifi => "nf-md-wifi",
        NetworkInterfaceType::Bluetooth => "nf-md-bluetooth",
        NetworkInterfaceType::Vpn => "nf-md-vpn",
        NetworkInterfaceType::Other => "nf-md-network",
    }
}

/// Returns the Nerd Font Unicode codepoint for a network interface type.
pub fn network_interface_icon_unicode(interface_type: &NetworkInterfaceType) -> &'static str {
    match interface_type {
        NetworkInterfaceType::Ethernet => "\u{f0200}",
        NetworkInterfaceType::Wifi => "\u{f0928}",
        NetworkInterfaceType::Bluetooth => "\u{f00af}",
        NetworkInterfaceType::Vpn => "\u{f099d}",
        NetworkInterfaceType::Other => "\u{f06fe}",
    }
}

/// Returns the Nerd Font icon name for a WLAN security type.
pub fn wifi_security_icon(security: &WifiSecurity) -> &'static str {
    match security {
        WifiSecurity::Open => "nf-md-lock_open_outline",
        WifiSecurity::Wep => "nf-md-lock",
        WifiSecurity::Wpa => "nf-md-lock",
        WifiSecurity::Wpa3 => "nf-md-lock",
        WifiSecurity::Unknown => "nf-md-lock_question",
    }
}

/// Returns the Nerd Font Unicode codepoint for a WLAN security type.
pub fn wifi_security_icon_unicode(security: &WifiSecurity) -> &'static str {
    match security {
        WifiSecurity::Open => "\u{f07f5}",
        WifiSecurity::Wep => "\u{f033e}",
        WifiSecurity::Wpa => "\u{f033e}",
        WifiSecurity::Wpa3 => "\u{f033e}",
        WifiSecurity::Unknown => "\u{f09ad}",
    }
}
