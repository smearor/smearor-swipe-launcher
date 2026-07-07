use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;
use stabby::vec::Vec as StabbyVec;

use crate::AccessPointInfo;
use crate::InterfaceStatus;
use crate::NetworkCommandAction;
use crate::NetworkCommandMessage;
use crate::NetworkConnectionState;
use crate::NetworkInterfaceType;
use crate::NetworkStatusMessage;
use crate::ScanResultsMessage;
use crate::VpnProfileInfo;
use crate::VpnProfilesMessage;
use crate::WifiSecurity;

fn parse_interface_type(value: &serde_json::Value) -> NetworkInterfaceType {
    match value.as_str() {
        Some("Ethernet") => NetworkInterfaceType::Ethernet,
        Some("Wifi") => NetworkInterfaceType::Wifi,
        Some("Bluetooth") => NetworkInterfaceType::Bluetooth,
        Some("Vpn") => NetworkInterfaceType::Vpn,
        _ => NetworkInterfaceType::Other,
    }
}

fn parse_connection_state(value: &serde_json::Value) -> NetworkConnectionState {
    match value.as_str() {
        Some("Connecting") => NetworkConnectionState::Connecting,
        Some("Connected") => NetworkConnectionState::Connected,
        Some("Failed") => NetworkConnectionState::Failed,
        Some("Unavailable") => NetworkConnectionState::Unavailable,
        _ => NetworkConnectionState::Disconnected,
    }
}

fn parse_wifi_security(value: &serde_json::Value) -> WifiSecurity {
    match value.as_str() {
        Some("Wep") => WifiSecurity::Wep,
        Some("Wpa") => WifiSecurity::Wpa,
        Some("Wpa3") => WifiSecurity::Wpa3,
        Some("Unknown") => WifiSecurity::Unknown,
        _ => WifiSecurity::Open,
    }
}

fn parse_command_action(value: &serde_json::Value) -> NetworkCommandAction {
    match value.as_str() {
        Some("Disconnect") => NetworkCommandAction::Disconnect,
        Some("ToggleRadio") => NetworkCommandAction::ToggleRadio,
        Some("ScanWifi") => NetworkCommandAction::ScanWifi,
        Some("ToggleVpn") => NetworkCommandAction::ToggleVpn,
        Some("Refresh") => NetworkCommandAction::Refresh,
        Some("GetPublicIp") => NetworkCommandAction::GetPublicIp,
        _ => NetworkCommandAction::ConnectWifi,
    }
}

fn parse_option_string(value: &serde_json::Value, key: &str) -> StabbyOption<stabby::string::String> {
    match value.get(key).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => StabbyOption::Some(stabby::string::String::from(s)),
        _ => StabbyOption::None(),
    }
}

fn parse_option_u8(value: &serde_json::Value, key: &str) -> StabbyOption<u8> {
    match value.get(key).and_then(|v| v.as_u64()) {
        Some(n) => StabbyOption::Some(n as u8),
        _ => StabbyOption::None(),
    }
}

fn parse_access_point(value: &serde_json::Value) -> AccessPointInfo {
    AccessPointInfo {
        ssid: stabby::string::String::from(value.get("ssid").and_then(|v| v.as_str()).unwrap_or("")),
        bssid: stabby::string::String::from(value.get("bssid").and_then(|v| v.as_str()).unwrap_or("")),
        signal: value.get("signal").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        frequency: value.get("frequency").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        security: parse_wifi_security(value.get("security").unwrap_or(&serde_json::Value::Null)),
        is_connected: value.get("is_connected").and_then(|v| v.as_bool()).unwrap_or(false),
        is_known: value.get("is_known").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn parse_access_points(value: &serde_json::Value) -> StabbyVec<AccessPointInfo> {
    let mut aps = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            aps.push(parse_access_point(item));
        }
    }
    aps
}

fn parse_interface_status(value: &serde_json::Value) -> InterfaceStatus {
    InterfaceStatus {
        interface_type: parse_interface_type(value.get("interface_type").unwrap_or(&serde_json::Value::Null)),
        interface_name: stabby::string::String::from(value.get("interface_name").and_then(|v| v.as_str()).unwrap_or("")),
        state: parse_connection_state(value.get("state").unwrap_or(&serde_json::Value::Null)),
        ssid: parse_option_string(value, "ssid"),
        signal: parse_option_u8(value, "signal"),
        ipv4_address: parse_option_string(value, "ipv4_address"),
        ipv6_address: parse_option_string(value, "ipv6_address"),
        internet_accessible: value.get("internet_accessible").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn parse_interfaces(value: &serde_json::Value) -> StabbyVec<InterfaceStatus> {
    let mut interfaces = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            interfaces.push(parse_interface_status(item));
        }
    }
    interfaces
}

fn parse_vpn_profile(value: &serde_json::Value) -> VpnProfileInfo {
    VpnProfileInfo {
        name: stabby::string::String::from(value.get("name").and_then(|v| v.as_str()).unwrap_or("")),
        vpn_type: stabby::string::String::from(value.get("vpn_type").and_then(|v| v.as_str()).unwrap_or("")),
        is_active: value.get("is_active").and_then(|v| v.as_bool()).unwrap_or(false),
        uuid: stabby::string::String::from(value.get("uuid").and_then(|v| v.as_str()).unwrap_or("")),
    }
}

fn parse_vpn_profiles(value: &serde_json::Value) -> StabbyVec<VpnProfileInfo> {
    let mut profiles = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            profiles.push(parse_vpn_profile(item));
        }
    }
    profiles
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NetworkCommandMessageConverter, NetworkCommandMessage, |json: serde_json::Value| {
    let action = parse_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let ssid = parse_option_string(&json, "ssid");
    let password = parse_option_string(&json, "password");
    let technology = parse_option_string(&json, "technology");
    let enabled = json.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let profile_name = parse_option_string(&json, "profile_name");
    let active = json.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
    NetworkCommandMessage {
        action,
        ssid,
        password,
        technology,
        enabled,
        profile_name,
        active,
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NetworkStatusMessageConverter, NetworkStatusMessage, |json: serde_json::Value| {
    let primary_interface = parse_interface_status(json.get("primary_interface").unwrap_or(&serde_json::Value::Null));
    let interfaces = parse_interfaces(json.get("interfaces").unwrap_or(&serde_json::Value::Null));
    let wifi_enabled = json.get("wifi_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let wwan_enabled = json.get("wwan_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let airplane_mode = json.get("airplane_mode").and_then(|v| v.as_bool()).unwrap_or(false);
    let received_bytes_per_second = json.get("received_bytes_per_second").and_then(|v| v.as_u64()).unwrap_or(0);
    let transmitted_bytes_per_second = json.get("transmitted_bytes_per_second").and_then(|v| v.as_u64()).unwrap_or(0);
    let last_updated = stabby::string::String::from(json.get("last_updated").and_then(|v| v.as_str()).unwrap_or(""));
    NetworkStatusMessage::new(
        primary_interface,
        interfaces,
        wifi_enabled,
        wwan_enabled,
        airplane_mode,
        received_bytes_per_second,
        transmitted_bytes_per_second,
        last_updated,
    )
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ScanResultsMessageConverter, ScanResultsMessage, |json: serde_json::Value| {
    let access_points = parse_access_points(json.get("access_points").unwrap_or(&serde_json::Value::Null));
    let scan_time = stabby::string::String::from(json.get("scan_time").and_then(|v| v.as_str()).unwrap_or(""));
    ScanResultsMessage { access_points, scan_time }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(VpnProfilesMessageConverter, VpnProfilesMessage, |json: serde_json::Value| {
    let profiles = parse_vpn_profiles(json.get("profiles").unwrap_or(&serde_json::Value::Null));
    let last_updated = stabby::string::String::from(json.get("last_updated").and_then(|v| v.as_str()).unwrap_or(""));
    VpnProfilesMessage { profiles, last_updated }
});

/// Register all JSON converter implementations for network messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    NetworkCommandMessageConverter::register_in_host(context);
    NetworkStatusMessageConverter::register_in_host(context);
    ScanResultsMessageConverter::register_in_host(context);
    VpnProfilesMessageConverter::register_in_host(context);
}
