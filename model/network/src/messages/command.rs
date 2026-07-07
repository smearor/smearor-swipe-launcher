use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_COMMAND: &str = "service.network.command";

/// Actions the network service can perform on request.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum NetworkCommandAction {
    /// Connect to a WLAN access point.
    #[default]
    ConnectWifi,
    /// Disconnect from the active connection on a device.
    Disconnect,
    /// Toggle a radio technology (WLAN, WWAN, or all).
    ToggleRadio,
    /// Request a WLAN scan.
    ScanWifi,
    /// Toggle a VPN connection.
    ToggleVpn,
    /// Refresh all status information from NetworkManager.
    Refresh,
    /// Query the public IP address via the HTTP service.
    GetPublicIp,
}

/// Command message sent by widgets or MCP clients to the network service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct NetworkCommandMessage {
    /// The action to execute.
    pub action: NetworkCommandAction,
    /// SSID for WLAN connection (used with `ConnectWifi`).
    pub ssid: stabby::option::Option<stabby::string::String>,
    /// Password for WLAN connection (used with `ConnectWifi`, optional for known networks).
    pub password: stabby::option::Option<stabby::string::String>,
    /// Radio technology to toggle (used with `ToggleRadio`): "wifi", "wwan", or "all".
    pub technology: stabby::option::Option<stabby::string::String>,
    /// Whether the radio should be enabled (used with `ToggleRadio`).
    pub enabled: bool,
    /// VPN profile name or UUID (used with `ToggleVpn`).
    pub profile_name: stabby::option::Option<stabby::string::String>,
    /// Whether the VPN should be active (used with `ToggleVpn`).
    pub active: bool,
}

impl NetworkCommandMessage {
    /// Creates a connect-WiFi command.
    pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Self {
        Self {
            action: NetworkCommandAction::ConnectWifi,
            ssid: stabby::option::Option::Some(stabby::string::String::from(ssid)),
            password: password
                .map(|p| stabby::option::Option::Some(stabby::string::String::from(p)))
                .unwrap_or(stabby::option::Option::None()),
            ..Default::default()
        }
    }

    /// Creates a disconnect command.
    pub fn disconnect() -> Self {
        Self {
            action: NetworkCommandAction::Disconnect,
            ..Default::default()
        }
    }

    /// Creates a toggle-radio command.
    pub fn toggle_radio(technology: &str, enabled: bool) -> Self {
        Self {
            action: NetworkCommandAction::ToggleRadio,
            technology: stabby::option::Option::Some(stabby::string::String::from(technology)),
            enabled,
            ..Default::default()
        }
    }

    /// Creates a scan-WiFi command.
    pub fn scan_wifi() -> Self {
        Self {
            action: NetworkCommandAction::ScanWifi,
            ..Default::default()
        }
    }

    /// Creates a toggle-VPN command.
    pub fn toggle_vpn(profile_name: &str, active: bool) -> Self {
        Self {
            action: NetworkCommandAction::ToggleVpn,
            profile_name: stabby::option::Option::Some(stabby::string::String::from(profile_name)),
            active,
            ..Default::default()
        }
    }

    /// Creates a refresh command.
    pub fn refresh() -> Self {
        Self {
            action: NetworkCommandAction::Refresh,
            ..Default::default()
        }
    }

    /// Creates a get-public-IP command.
    pub fn get_public_ip() -> Self {
        Self {
            action: NetworkCommandAction::GetPublicIp,
            ..Default::default()
        }
    }
}

impl TypedMessage for NetworkCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_network_model::NetworkCommandMessage");
}

impl MessageTopic for NetworkCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for NetworkCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
