# Concept: Network Menu Service & Widget

This document describes the concept for a **Network Menu Service** and a **Network Menu Widget** in the *Smearor Swipe Launcher*. The service communicates
with **NetworkManager** via **D-Bus** using the [`zbus`](https://crates.io/crates/zbus) crate to query and control all network interfaces (WLAN, Ethernet,
Bluetooth-Tethering, VPN). The widget provides a compact, touch-optimized GTK4 menu with signal-strength visualization, WLAN scanning, QR-code sharing,
and VPN toggles.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/network`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/network`):** Singleton background service that interfaces with `org.freedesktop.NetworkManager` via D-Bus, queries device and
   connection state, performs WLAN scans, manages VPN profiles, and broadcasts status updates.
3. **Widget Crate (`plugins/network`):** Pure GTK4 UI that displays connection status, scan results, signal strength, QR codes, and VPN toggles.

---

## 1. Feature Scope

The Network Menu aggregates and controls the status of all relevant network interfaces:

| Feature                   | Description                                                                                                              |
|---------------------------|--------------------------------------------------------------------------------------------------------------------------|
| **Status Overview**       | Current connection status, WLAN SSID, signal strength, IP address (IPv4/IPv6), and interface name (e.g., `wlan0`).       |
| **WLAN Scanner**          | Lists all available access points in range, sorted by signal strength, including encryption status (open vs. WPA2/WPA3). |
| **Connection Management** | Connect to known (saved) networks or enter passwords for new networks. Disconnect from active connections.               |
| **Airplane Mode**         | Quick global kill-switch that disables all radio connections (WLAN & Bluetooth).                                         |
| **VPN Integration**       | List and quickly toggle WireGuard or OpenVPN profiles registered in NetworkManager.                                      |

---

## 2. Recommended Libraries

On Ubuntu (and most modern Linux distributions), **NetworkManager** is the standard network management daemon. It provides a powerful and well-documented
D-Bus interface.

- **`zbus`:** The standard asynchronous Rust D-Bus library. Communicates directly with `org.freedesktop.NetworkManager` on the system bus.
    - NetworkManager provides ready-made D-Bus objects for devices (`/org/freedesktop/NetworkManager/Devices/*`) and active connections.
    - It is performant and reacts in real-time via D-Bus signals (events) when, for example, an Ethernet cable is plugged in.
- **Alternative:** The [`networkmanager`](https://crates.io/crates/networkmanager) crate provides Rust bindings. However, directly addressing the required
  properties via `zbus` is often more flexible and leaner for the specific model architecture of this project.

---

## 3. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Network Menu Widget      |                 | Network Service            |
| (subscribed to           |                 | (Singleton)                |
|  service.network.status) |                 |                            |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Command Message                         |
             |  (connect, disconnect, toggle radio, VPN)   |
             |===========================================> |
             |  Topic: "service.network.command"           |
             |                                             |
             |                                             |  2. zbus D-Bus call
             |                                             |     org.freedesktop.NetworkManager
             |                                             |     .Devices / .ActiveConnections
             |                                             |     .AccessPoints / .VPN.Connections
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.network.status"
             |                                             |     Payload: NetworkStatusMessage { ... }
+--------------------------+                 +----------------------------+
             |                                             |
             |                                             |  4. WLAN Scan
             |                                             |     org.freedesktop.NetworkManager
             |                                             |     .Device.Wireless.GetAccessPoints
             |                                             |
             |                                             |  5. VPN Query
             |                                             |     org.freedesktop.NetworkManager
             |                                             |     .GetConnections (filter by type)
+--------------------------+                 +----------------------------+
```

The service also registers **MCP resources** and **MCP tools** so that AI clients can query network state and trigger network actions.

---

## 4. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path                | Responsibility                                                                     |
|-------------|---------------------|------------------------------------------------------------------------------------|
| **Model**   | `model/network/`    | Shared structs, enums, topics, and message formats                                 |
| **Service** | `services/network/` | D-Bus communication, device/connection queries, WLAN scanning, VPN management, MCP |
| **Widget**  | `plugins/network/`  | GTK4 menu UI, signal visualization, QR code, scan list, VPN toggles                |

---

## 5. Model Crate (`model/network`)

### 5.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.network.command";
pub const TOPIC_STATUS: &str = "service.network.status";
pub const TOPIC_SCAN_RESULTS: &str = "service.network.scan_results";
pub const TOPIC_VPN_PROFILES: &str = "service.network.vpn_profiles";
```

### 5.2 Network Interface Type Enum

```rust
/// Type of a network interface as reported by NetworkManager.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
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
```

### 5.3 Connection State Enum

```rust
/// Connection state of a network device as reported by NetworkManager.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum NetworkConnectionState {
    /// Device is disconnected.
    #[default]
    Disconnected,
    /// Device is connecting.
    Connecting,
    /// Device is fully connected.
    Connected,
    /// Device is in a failed state.
    Failed,
    /// Device is unavailable.
    Unavailable,
}
```

### 5.4 Security / Encryption Enum

```rust
/// Encryption type of a WLAN access point.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum WifiSecurity {
    /// Open network, no encryption.
    #[default]
    Open,
    /// WEP encryption (legacy).
    Wep,
    /// WPA/WPA2 encryption.
    Wpa,
    /// WPA3 encryption.
    Wpa3,
    /// Unknown or mixed encryption.
    Unknown,
}
```

### 5.5 Access Point Info

```rust
/// Information about a single WLAN access point found during a scan.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
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
```

### 5.6 VPN Profile Info

```rust
/// Information about a VPN connection profile registered in NetworkManager.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
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
```

### 5.7 Interface Status

```rust
/// Status of a single network interface.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
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
```

### 5.8 Network Status Message (Service -> Widget)

```rust
/// Complete network status message broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct NetworkStatusMessage {
    /// Primary active interface (the one with internet access, or the first connected).
    pub primary_interface: InterfaceStatus,
    /// All active interfaces.
    pub interfaces: stabby::vec::Vec<InterfaceStatus>,
    /// Whether WLAN radio is enabled.
    pub wifi_enabled: bool,
    /// Whether WWAN (mobile broadband) radio is enabled.
    pub wwan_enabled: bool,
    /// Whether airplane mode is active (all radios off).
    pub airplane_mode: bool,
    /// Aggregate inbound throughput in bytes per second.
    pub received_bytes_per_second: u64,
    /// Aggregate outbound throughput in bytes per second.
    pub transmitted_bytes_per_second: u64,
    /// Timestamp of the last status refresh as ISO-8601 string.
    pub last_updated: stabby::string::String,
}
```

### 5.9 Scan Results Message (Service -> Widget)

```rust
/// WLAN scan results message broadcast by the service after a scan request.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ScanResultsMessage {
    /// List of access points found, sorted by signal strength (strongest first).
    pub access_points: stabby::vec::Vec<AccessPointInfo>,
    /// Timestamp of the scan as ISO-8601 string.
    pub scan_time: stabby::string::String,
}
```

### 5.10 VPN Profiles Message (Service -> Widget)

```rust
/// VPN profiles message broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct VpnProfilesMessage {
    /// List of all VPN connection profiles registered in NetworkManager.
    pub profiles: stabby::vec::Vec<VpnProfileInfo>,
    /// Timestamp of the last refresh as ISO-8601 string.
    pub last_updated: stabby::string::String,
}
```

### 5.11 Command Message (Widget -> Service)

```rust
/// Actions the network service can perform on request.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
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
```

### 5.12 Nerd Font Icon Mapping

Each network state and action maps to a Material Design Nerd Font icon for consistent GTK4 rendering.

| State / Action          | Icon | Unicode     | Nerd Font Name            |
|-------------------------|------|-------------|---------------------------|
| WLAN connected (100%)   | 󰤨   | `\u{f0928}` | `nf-md-wifi`              |
| WLAN medium (50%)       | 󰤢   | `\u{f0922}` | `nf-md-wifi_strength_2`   |
| WLAN disconnected / Off | 󰤭   | `\u{f092d}` | `nf-md-wifi_off`          |
| Ethernet (wired)        | 󰈀   | `\u{f0200}` | `nf-md-ethernet`          |
| Airplane mode active    | 󰀝   | `\u{f001d}` | `nf-md-airplane`          |
| VPN active              | 󰦝   | `\u{f099d}` | `nf-md-vpn`               |
| Secure network (WPA)    | 󰌾   | `\u{f033e}` | `nf-md-lock`              |
| Open network            | 󰟵   | `\u{f07f5}` | `nf-md-lock_open_outline` |

The mapping is defined in the model crate as utility functions:

```rust
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
```

### 5.13 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::access_point::AccessPointInfo;
pub use messages::command::NetworkCommandAction;
pub use messages::command::NetworkCommandMessage;
pub use messages::icon::network_interface_icon;
pub use messages::icon::wifi_security_icon;
pub use messages::icon::wifi_signal_icon;
pub use messages::interface_status::InterfaceStatus;
pub use messages::scan_results::ScanResultsMessage;
pub use messages::state::NetworkConnectionState;
pub use messages::status::NetworkStatusMessage;
pub use messages::type_enum::NetworkInterfaceType;
pub use messages::vpn_profiles::VpnProfileInfo;
pub use messages::vpn_profiles::VpnProfilesMessage;
pub use messages::security::WifiSecurity;
```

---

## 6. Service Crate (`services/network`)

### 6.1 File Structure

- `service.rs` - `NetworkService` struct and trait implementations
- `config.rs` - `NetworkServiceConfig` struct and parsing
- `dbus.rs` - D-Bus proxy definitions and communication logic
- `scanner.rs` - WLAN scan logic
- `vpn.rs` - VPN profile query and management logic
- `throughput.rs` - Network throughput calculation
- `lib.rs` - `service_plugin!` macro invocation

### 6.2 Service Implementation

```rust
pub struct NetworkService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NetworkServiceConfig,
    pub state: Arc<RwLock<NetworkStatusMessage>>,
    pub scan_state: Arc<RwLock<ScanResultsMessage>>,
    pub vpn_state: Arc<RwLock<VpnProfilesMessage>>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<NetworkCommand>,
}

/// Internal command union for the service event loop.
pub enum NetworkCommand {
    /// Connect to a WLAN access point with optional password.
    ConnectWifi(String, Option<String>),
    /// Disconnect from the active connection on a device.
    Disconnect(String),
    /// Toggle a radio technology (wifi, wwan, or all).
    ToggleRadio(String, bool),
    /// Request a WLAN scan.
    ScanWifi,
    /// Toggle a VPN connection by profile name or UUID.
    ToggleVpn(String, bool),
    /// Refresh all status information from NetworkManager.
    Refresh,
    /// Query the public IP address via the HTTP service.
    GetPublicIp,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<NetworkCommandMessage>>` - Processes commands from widgets and MCP clients
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 6.3 Configuration

```rust
/// Configuration for the network service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NetworkServiceConfig {
    /// Interval in milliseconds for refreshing device and connection status.
    pub refresh_interval_ms: u64,
    /// Whether WLAN scanning should be enabled.
    pub enable_wifi_scan: bool,
    /// Whether VPN profile management should be enabled.
    pub enable_vpn: bool,
    /// Whether throughput metrics should be collected.
    pub enable_throughput: bool,
    /// Whether airplane mode (kill-switch) should be available.
    pub enable_airplane_mode: bool,
    /// Interval in milliseconds for throughput sampling.
    pub throughput_interval_ms: u64,
    /// Number of throughput history samples to keep for sparkline rendering.
    pub throughput_history_length: usize,
}

impl Default for NetworkServiceConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 2000,
            enable_wifi_scan: true,
            enable_vpn: true,
            enable_throughput: true,
            enable_airplane_mode: true,
            throughput_interval_ms: 1000,
            throughput_history_length: 60,
        }
    }
}
```

### 6.4 D-Bus Communication

The service uses `zbus` to communicate with `org.freedesktop.NetworkManager`. The D-Bus proxy interface is defined in `dbus.rs`:

```rust
/// D-Bus proxy for `org.freedesktop.NetworkManager`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    fn get_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    fn get_active_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    fn get_all_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    fn activate_connection(
        &self,
        connection: &zbus::zvariant::ObjectPath,
        device: &zbus::zvariant::ObjectPath,
        specific_object: &zbus::zvariant::ObjectPath,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn deactivate_connection(&self, active_connection: &zbus::zvariant::ObjectPath) -> zbus::Result<()>;
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn wwan_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_wwan_enabled(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Device`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait NetworkDevice {
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn ip4_config(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    #[zbus(property)]
    fn ip6_config(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn disconnect(&self) -> zbus::Result<()>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Device.Wireless`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
trait WirelessDevice {
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    fn request_scan(&self, options: std::collections::HashMap<&str, zbus::zvariant::Value>) -> zbus::Result<()>;
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.AccessPoint`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;
    #[zbus(property)]
    fn bssid(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn frequency(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn rsn_flags(&self) -> zbus::Result<u32>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Connection.Active`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait ActiveConnection {
    #[zbus(property)]
    fn connection(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn type ( & self ) -> zbus::Result<String>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Settings.Connection`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings.Connection",
    default_service = "org.freedesktop.NetworkManager"
)]
trait SettingsConnection {
    fn get_settings(&self) -> zbus::Result<std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::Value>>>;
}
```

**Device type mapping:**

| NM Device Type | `NetworkInterfaceType` |
|----------------|------------------------|
| 1 (Ethernet)   | `Ethernet`             |
| 2 (Wifi)       | `Wifi`                 |
| 5 (Bluetooth)  | `Bluetooth`            |
| 12 (VPN)       | `Vpn`                  |
| Other          | `Other`                |

**Connection state mapping:**

| NM State | `NetworkConnectionState` |
|----------|--------------------------|
| 30       | `Disconnected`           |
| 40       | `Connecting`             |
| 100      | `Connected`              |
| 120      | `Failed`                 |
| 20       | `Unavailable`            |

### 6.5 WLAN Scan

When a `ScanWifi` command is received, the service calls `RequestScan` on the wireless device and then queries all access points. The results are sorted by
signal strength (strongest first) and broadcast on `TOPIC_SCAN_RESULTS`.

```rust
async fn perform_wifi_scan(
    connection: &zbus::Connection,
    device_path: &zbus::zvariant::OwnedObjectPath,
) -> Vec<AccessPointInfo> {
    let wireless = WirelessDeviceProxy::new(connection, device_path).await;
    match wireless {
        Ok(proxy) => {
            let _ = proxy.request_scan(std::collections::HashMap::new()).await;
            let ap_paths = proxy.get_access_points().await.unwrap_or_default();
            let mut access_points = Vec::new();
            for ap_path in ap_paths {
                if let Ok(ap) = AccessPointProxy::new(connection, &ap_path).await {
                    let ssid_bytes = ap.ssid().await.unwrap_or_default();
                    let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                    let bssid = ap.bssid().await.unwrap_or_default();
                    let strength = ap.strength().await.unwrap_or_default();
                    let frequency = ap.frequency().await.unwrap_or_default();
                    let flags = ap.flags().await.unwrap_or(0);
                    let rsn_flags = ap.rsn_flags().await.unwrap_or(0);
                    let security = determine_security(flags, rsn_flags);
                    access_points.push(AccessPointInfo {
                        ssid: stabby::string::String::from(ssid),
                        bssid: stabby::string::String::from(bssid),
                        signal: strength,
                        frequency,
                        security,
                        is_connected: false,
                        is_known: false,
                    });
                }
            }
            access_points.sort_by(|a, b| b.signal.cmp(&a.signal));
            access_points
        }
        Err(_) => Vec::new(),
    }
}
```

### 6.6 VPN Management

The service queries all connections from NetworkManager and filters for VPN types (`vpn` and `wireguard`). For each VPN profile, it checks whether an
active connection exists.

```rust
async fn refresh_vpn_profiles(connection: &zbus::Connection) -> Vec<VpnProfileInfo> {
    let manager = NetworkManagerProxy::new(connection).await;
    match manager {
        Ok(proxy) => {
            let all_connections = proxy.get_all_connections().await.unwrap_or_default();
            let active_connections = proxy.get_active_connections().await.unwrap_or_default();
            let mut profiles = Vec::new();
            for conn_path in all_connections {
                if let Ok(settings) = SettingsConnectionProxy::new(connection, &conn_path).await {
                    let settings_map = settings.get_settings().await.unwrap_or_default();
                    if let Some(connection_section) = settings_map.get("connection") {
                        if let Some(conn_type) = connection_section.get("type") {
                            let type_str = conn_type.to_string();
                            if type_str == "vpn" || type_str == "wireguard" {
                                let name = connection_section.get("id")
                                    .map(|v| v.to_string())
                                    .unwrap_or_default();
                                let uuid = connection_section.get("uuid")
                                    .map(|v| v.to_string())
                                    .unwrap_or_default();
                                let is_active = active_connections.iter().any(|active| {
                                    if let Ok(active_conn) = ActiveConnectionProxy::new(connection, active).await {
                                        active_conn.connection().map(|c| c.as_ref() == conn_path.as_ref()).unwrap_or(false)
                                    } else {
                                        false
                                    }
                                });
                                profiles.push(VpnProfileInfo {
                                    name: stabby::string::String::from(name),
                                    vpn_type: stabby::string::String::from(type_str),
                                    is_active,
                                    uuid: stabby::string::String::from(uuid),
                                });
                            }
                        }
                    }
                }
            }
            profiles
        }
        Err(_) => Vec::new(),
    }
}
```

### 6.7 Throughput Calculation

The service samples `/proc/net/dev` at the configured `throughput_interval_ms` and computes the delta between two reads. The aggregate inbound and
outbound throughput across all interfaces (excluding loopback) is broadcast as part of `NetworkStatusMessage`.

```rust
async fn collect_throughput() -> (u64, u64) {
    let content = tokio::fs::read_to_string("/proc/net/dev").await.unwrap_or_default();
    let mut total_received: u64 = 0;
    let mut total_transmitted: u64 = 0;
    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            let interface = parts[0].trim_end_matches(':');
            if interface != "lo" {
                total_received += parts[1].parse::<u64>().unwrap_or(0);
                total_transmitted += parts[9].parse::<u64>().unwrap_or(0);
            }
        }
    }
    (total_received, total_transmitted)
}
```

### 6.8 Background Update Loop

On initialization, the service spawns a dedicated OS thread with a single-threaded Tokio runtime. The runtime runs an update loop that refreshes device
and connection status at the configured interval, samples throughput, and processes incoming commands.

```rust
async fn run_update_loop(
    config: NetworkServiceConfig,
    state: Arc<RwLock<NetworkStatusMessage>>,
    scan_state: Arc<RwLock<ScanResultsMessage>>,
    vpn_state: Arc<RwLock<VpnProfilesMessage>>,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<NetworkCommand>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    let connection = zbus::Connection::system().await;
    let mut interval = tokio::time::interval(Duration::from_millis(config.refresh_interval_ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut prev_received: u64 = 0;
    let mut prev_transmitted: u64 = 0;
    let mut throughput_timer = tokio::time::interval(Duration::from_millis(config.throughput_interval_ms));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Ok(ref conn) = connection {
                    let status = refresh_full_status(conn, &config).await;
                    let mut current = state.write().await;
                    *current = status;
                    let status_clone = current.clone();
                    drop(current);
                    broadcaster.broadcast_topic(TOPIC_STATUS, status_clone);

                    if config.enable_vpn {
                        let vpn_profiles = refresh_vpn_profiles(conn).await;
                        let mut vpn_current = vpn_state.write().await;
                        *vpn_current = VpnProfilesMessage {
                            profiles: vpn_profiles.into(),
                            last_updated: stabby::string::String::from(current_iso8601()),
                        };
                        let vpn_clone = vpn_current.clone();
                        drop(vpn_current);
                        broadcaster.broadcast_topic(TOPIC_VPN_PROFILES, vpn_clone);
                    }
                }
            }
            _ = throughput_timer.tick() => {
                if config.enable_throughput {
                    let (received, transmitted) = collect_throughput().await;
                    let received_bps = received.saturating_sub(prev_received);
                    let transmitted_bps = transmitted.saturating_sub(prev_transmitted);
                    prev_received = received;
                    prev_transmitted = transmitted;
                    let mut current = state.write().await;
                    current.received_bytes_per_second = received_bps;
                    current.transmitted_bytes_per_second = transmitted_bps;
                    let status_clone = current.clone();
                    drop(current);
                    broadcaster.broadcast_topic(TOPIC_STATUS, status_clone);
                }
            }
            Some(command) = command_receiver.recv() => {
                match command {
                    NetworkCommand::ConnectWifi(ssid, password) => {
                        if let Ok(ref conn) = connection {
                            connect_to_wifi(conn, &ssid, password.as_deref()).await;
                        }
                    }
                    NetworkCommand::Disconnect(interface) => {
                        if let Ok(ref conn) = connection {
                            disconnect_device(conn, &interface).await;
                        }
                    }
                    NetworkCommand::ToggleRadio(technology, enabled) => {
                        if let Ok(ref conn) = connection {
                            toggle_radio(conn, &technology, enabled).await;
                        }
                    }
                    NetworkCommand::ScanWifi => {
                        if let Ok(ref conn) = connection {
                            let results = perform_wifi_scan(conn, &get_wifi_device_path(conn).await).await;
                            let mut scan_current = scan_state.write().await;
                            *scan_current = ScanResultsMessage {
                                access_points: results.into(),
                                scan_time: stabby::string::String::from(current_iso8601()),
                            };
                            let scan_clone = scan_current.clone();
                            drop(scan_current);
                            broadcaster.broadcast_topic(TOPIC_SCAN_RESULTS, scan_clone);
                        }
                    }
                    NetworkCommand::ToggleVpn(profile_name, active) => {
                        if let Ok(ref conn) = connection {
                            toggle_vpn(conn, &profile_name, active).await;
                        }
                    }
                    NetworkCommand::Refresh => {
                        if let Ok(ref conn) = connection {
                            let status = refresh_full_status(conn, &config).await;
                            let mut current = state.write().await;
                            *current = status;
                            let status_clone = current.clone();
                            drop(current);
                            broadcaster.broadcast_topic(TOPIC_STATUS, status_clone);
                        }
                    }
                    NetworkCommand::GetPublicIp => {
                        // Delegate to the HTTP service via a message broker request.
                        // The result is published as a status update.
                    }
                }
            }
        }
    }
}
```

### 6.9 MCP Resources

The service registers the following MCP resources via the Plugin-Resource-Registry:

| URI                      | Description                                                                                          | Source type            |
|--------------------------|------------------------------------------------------------------------------------------------------|------------------------|
| `network://status`       | Structured JSON about the primary active interface (type, SSID, signal, IP, internet accessibility). | `NetworkStatusMessage` |
| `network://scan-results` | List of all WLAN access points in range, including signal strength and encryption.                   | `ScanResultsMessage`   |
| `network://vpn-profiles` | List of all VPN connections registered in NetworkManager and their current state (active/inactive).  | `VpnProfilesMessage`   |

Example `network://status` response:

```json
{
  "type": "wifi",
  "ssid": "MeinHeimNetz",
  "signal": 84,
  "ip": "192.168.1.42",
  "internet_accessible": true
}
```

Example `network://scan-results` response:

```json
{
  "access_points": [
    {
      "ssid": "MeinHeimNetz",
      "bssid": "AA:BB:CC:DD:EE:FF",
      "signal": 84,
      "frequency": 2412,
      "security": "Wpa",
      "is_connected": true,
      "is_known": true
    },
    {
      "ssid": "GuestNetwork",
      "bssid": "11:22:33:44:55:66",
      "signal": 42,
      "frequency": 2437,
      "security": "Open",
      "is_connected": false,
      "is_known": false
    }
  ],
  "scan_time": "2025-07-07T12:40:00Z"
}
```

Example `network://vpn-profiles` response:

```json
{
  "profiles": [
    {
      "name": "Firmen-VPN",
      "vpn_type": "wireguard",
      "is_active": false,
      "uuid": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    }
  ],
  "last_updated": "2025-07-07T12:40:00Z"
}
```

### 6.10 MCP Tools

The service registers the following MCP tools via the Plugin-Tool-Registry:

| Tool                    | Description                                                                       | Parameters                                                       |
|-------------------------|-----------------------------------------------------------------------------------|------------------------------------------------------------------|
| `network_toggle_radio`  | Toggles WLAN or airplane mode on/off.                                             | `technology: string` ("wifi", "wwan", "all"), `enabled: boolean` |
| `network_connect_wifi`  | Connects the system to a specific access point.                                   | `ssid: string`, `password: string` (optional)                    |
| `network_toggle_vpn`    | Starts or stops a specific VPN connection.                                        | `profile_name: string`, `active: boolean`                        |
| `network_get_public_ip` | Triggers the internal HTTP service to query the external IP and provider (GeoIP). | -                                                                |

> **MCP tool naming convention:** Tool names use `snake_case` with underscores, never dots. Dots in tool names cause schema validation failures in LLM
> gateways (Windsurf, Claude, etc.). This is consistent with existing tools like `sysinfo_refresh` and `get_current_time`.

**Example JSON schema for `network_toggle_radio`:**

```json
{
  "name": "network_toggle_radio",
  "description": "Toggles WLAN or airplane mode on/off. Useful for prompts like: 'Schalte das WLAN aus, ich will offline arbeiten.'",
  "inputSchema": {
    "type": "object",
    "properties": {
      "technology": {
        "type": "string",
        "enum": [
          "wifi",
          "wwan",
          "all"
        ],
        "description": "The radio technology to toggle"
      },
      "enabled": {
        "type": "boolean",
        "description": "Whether the radio should be enabled"
      }
    },
    "required": [
      "technology",
      "enabled"
    ]
  }
}
```

**Example JSON schema for `network_connect_wifi`:**

```json
{
  "name": "network_connect_wifi",
  "description": "Connects the system to a specific WLAN access point. Useful for prompts like: 'Verbinde mich mit dem WLAN MeinHeimNetz.'",
  "inputSchema": {
    "type": "object",
    "properties": {
      "ssid": {
        "type": "string",
        "description": "The SSID of the access point to connect to"
      },
      "password": {
        "type": "string",
        "description": "The password for the WLAN (optional for known networks)"
      }
    },
    "required": [
      "ssid"
    ]
  }
}
```

**Example JSON schema for `network_toggle_vpn`:**

```json
{
  "name": "network_toggle_vpn",
  "description": "Starts or stops a specific VPN connection. Useful for prompts like: 'Verbinde mich mit dem Firmen-VPN.'",
  "inputSchema": {
    "type": "object",
    "properties": {
      "profile_name": {
        "type": "string",
        "description": "The name or UUID of the VPN profile"
      },
      "active": {
        "type": "boolean",
        "description": "Whether the VPN should be active"
      }
    },
    "required": [
      "profile_name",
      "active"
    ]
  }
}
```

**Example JSON schema for `network_get_public_ip`:**

```json
{
  "name": "network_get_public_ip",
  "description": "Triggers the internal HTTP service to query the external IP address and provider (GeoIP). Useful for prompts like: 'Bin ich gerade sicher im VPN getunnelt?'",
  "inputSchema": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

---

## 7. Widget Crate (`plugins/network`)

### 7.1 Overview

The Network Menu Widget is a GTK4 menu that displays the current network status, a WLAN scan list, VPN toggles, and an airplane mode button. It subscribes
to `service.network.status`, `service.network.scan_results`, and `service.network.vpn_profiles` and updates its display based on incoming messages. The
widget is optimized for 32-inch touch/swipe displays.

### 7.2 File Structure

- `widget.rs` - `NetworkWidget` struct and trait implementations
- `config.rs` - `NetworkWidgetConfig` struct and parsing
- `status_view.rs` - Status overview rendering (interface, SSID, signal, IP)
- `scan_list.rs` - WLAN scan results list rendering
- `vpn_list.rs` - VPN profile list and toggle rendering
- `qr_code.rs` - QR code generation and rendering for WLAN sharing
- `sparkline.rs` - Mini throughput sparkline rendering
- `lib.rs` - `widget_plugin!` macro invocation

### 7.3 Widget Configuration

```rust
/// Configuration for the network menu widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Spacing between elements.
    pub spacing: i32,
    /// Whether to show the status overview section.
    pub show_status: bool,
    /// Whether to show the WLAN scan list.
    pub show_scan_list: bool,
    /// Whether to show the airplane mode toggle.
    pub show_airplane_mode: bool,
    /// Whether to show the VPN toggle list.
    pub show_vpn: bool,
    /// Whether to show the throughput sparkline.
    pub show_throughput_sparkline: bool,
    /// Whether to show the QR code generator button.
    pub show_qr_code: bool,
    /// Maximum number of access points to display in the scan list.
    pub max_scan_results: usize,
    /// Whether to show signal strength as a dot-matrix or bars.
    pub signal_display_mode: SignalDisplayMode,
    /// Button size in pixels.
    pub button_size: i32,
    /// Icon size in pixels.
    pub icon_size: i32,
    /// Background color of the widget.
    pub background_color: Option<String>,
    /// Message topic for single-click (opens the network menu area).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
}

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            width: 280,
            height: 400,
            spacing: 8,
            show_status: true,
            show_scan_list: true,
            show_airplane_mode: true,
            show_vpn: true,
            show_throughput_sparkline: true,
            show_qr_code: true,
            max_scan_results: 10,
            signal_display_mode: SignalDisplayMode::Bars,
            button_size: 48,
            icon_size: 24,
            background_color: None,
            click_topic: None,
            click_payload: None,
        }
    }
}
```

### 7.4 Signal Display Mode

```rust
/// Visual representation for WLAN signal strength.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum SignalDisplayMode {
    /// Vertical bars (decreasing height for weaker signal).
    #[default]
    Bars,
    /// Dot matrix (filled dots for signal strength).
    Dots,
}
```

### 7.5 Widget Implementation

```rust
pub struct NetworkWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NetworkWidgetConfig,
    pub current_status: Option<NetworkStatusMessage>,
    pub current_scan_results: Option<ScanResultsMessage>,
    pub current_vpn_profiles: Option<VpnProfilesMessage>,
}
```

> **GTK widget references:** GTK4 widgets (`gtk4::Box`, `gtk4::Button`, `gtk4::Label`) are **not** `Send` or `Sync`. They must not be stored in
> `Arc<RwLock<...>>` inside the plugin struct. Instead, widget references are captured inside `glib::clone!` closures or passed directly to
> `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state (`config`, `current_status`, `current_scan_results`,
> `current_vpn_profiles`).

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<NetworkStatusMessage>>` - Receives status updates from the service
- `MessageHandler<FfiEnvelopePayload<ScanResultsMessage>>` - Receives scan results from the service
- `MessageHandler<FfiEnvelopePayload<VpnProfilesMessage>>` - Receives VPN profile updates from the service
- `MessageBroadcaster` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 7.6 Status Overview Layout

The status overview section displays the primary active interface with its type icon, SSID (for WLAN), signal strength visualization, and IP address.

```
+------------------------------+
|  󰤨 MeinHeimNetz  󰤢 84%     |
|  192.168.1.42                |
|  ↓ 1.2 MB/s  ↑ 256 KB/s     |
+------------------------------+
```

- The interface type icon is selected via `network_interface_icon`.
- The WLAN signal icon is selected via `wifi_signal_icon` and changes color (green/yellow/red) based on signal quality.
- The throughput sparkline (analog to the Sysinfo widget) shows a mini-graph of recent download/upload rates.

### 7.7 WLAN Scan List

The scan list displays all access points found during the last scan, sorted by signal strength. Each entry shows the SSID, signal strength, and encryption
icon.

```
+------------------------------+
|  Available Networks          |
|------------------------------|
|  󰤨 MeinHeimNetz   󰌾 84%   |
|  󰤢 GuestNetwork   󰟵 42%   |
|  󰤡 NeighborWifi   󰌾 31%   |
|  ...                         |
+------------------------------+
```

- Connected networks show a checkmark or highlighted background.
- Known (saved) networks show a filled icon; unknown networks show an outline.
- Tapping a known network connects immediately.
- Tapping an unknown network opens a password entry dialog.
- The encryption icon is selected via `wifi_security_icon`.

### 7.8 Signal Strength Visualization

Instead of bulky text lines, the widget uses a compact signal design:

- **Bars mode:** Vertical bars with decreasing height for weaker signal. Bars change color from green (strong) to yellow (medium) to red (weak).
- **Dots mode:** A dot matrix where filled dots represent signal strength. Same color scheme applies.

```
Bars:    █ █ █ █   (100%)       Dots:    ● ● ● ●   (100%)
         █ █ █ ░   (75%)                 ● ● ● ○   (75%)
         █ █ ░ ░   (50%)                 ● ● ○ ○   (50%)
         █ ░ ░ ░   (25%)                 ● ○ ○ ○   (25%)
```

### 7.9 QR Code Generator

When the user clicks the QR code button (visible only when connected to a WLAN), the widget generates a QR code encoding the WLAN credentials
(`WIFI:S:<SSID>;T:<security>;P:<password>;;` format). The QR code is displayed as an overlay that visitors can scan with their phone cameras.

```
+------------------------------+
|                              |
|     ┌──────────────┐         |
|     │ ▓▓ ▓▓ ▓▓ ▓▓ │         |
|     │ ▓▓ ▓▓ ▓▓ ▓▓ │         |
|     │ ▓▓ ▓▓ ▓▓ ▓▓ │         |
|     │ ▓▓ ▓▓ ▓▓ ▓▓ │         |
|     └──────────────┘         |
|     Scan to connect          |
|     to MeinHeimNetz          |
|                              |
|     [ 󰅖 Close ]             |
+------------------------------+
```

The QR code is rendered using a `gtk4::DrawingArea` with a custom draw function, or via an image generated from a QR code library (e.g., `qrcode` crate)
and displayed as a `gtk4::Picture`.

### 7.10 VPN Toggle List

The VPN list displays all VPN profiles registered in NetworkManager with a toggle switch.

```
+------------------------------+
|  VPN Profiles                |
|------------------------------|
|  󰦝 Firmen-VPN     [ ON ]    |
|  󰦝 Home-WG        [OFF ]    |
+------------------------------+
```

- Tapping a toggle sends a `ToggleVpn` command to the service.
- Active VPNs show a highlighted toggle and the `nf-md-vpn` icon in color.
- Inactive VPNs show a grayed-out toggle.

### 7.11 Airplane Mode

The airplane mode button is a prominent toggle at the top or bottom of the widget.

```
+------------------------------+
|  󰀝 Airplane Mode  [OFF ]    |
+------------------------------+
```

- When activated, the service disables all radios (WLAN, WWAN, Bluetooth).
- The icon changes color to indicate active state.
- Tapping sends a `ToggleRadio` command with `technology = "all"` and `enabled = false/true`.

### 7.12 State Synchronization

The widget subscribes to three topics:

1. `service.network.status` - Updates the status overview, throughput sparkline, and airplane mode toggle.
2. `service.network.scan_results` - Updates the WLAN scan list.
3. `service.network.vpn_profiles` - Updates the VPN toggle list.

When a new message arrives:

1. The message is deserialized and stored in the corresponding state field.
2. The affected section is re-rendered.
3. All GTK updates happen via `glib::MainContext::spawn_local`.

---

## 8. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
| Network Widget    |<--------|                   |-------->| Network Service   |
| (menu in area)    |  Status |   Event Broker    | Command | (Singleton)       |
+---------+---------+ Broadcast +-------------------+ Broadcast +-------------------+
          |                                                 |
          | Click: send NetworkCommandMessage               | zbus D-Bus
          | (connect, disconnect, toggle radio, VPN)        | org.freedesktop
          v                                                 |   .NetworkManager
+-------------------+                               +-------------------+
| Scan list /       |                               | NetworkManager    |
| QR code overlay   |                               | /Devices/*        |
| (local state)     |                               | /ActiveConnections|
+-------------------+                               +-------------------+
```

---

## 9. Configuration Example

### 9.1 Service Registration in `services.toml`

```toml
[[services]]
id = "network"
path = "target/release/libsmearor_network_service.so"

[network]
refresh_interval_ms = 2000
enable_wifi_scan = true
enable_vpn = true
enable_throughput = true
enable_airplane_mode = true
throughput_interval_ms = 1000
throughput_history_length = 60
```

### 9.2 Widget Configuration in `config.toml`

```toml
[[scroll_band.plugins]]
id = "network_widget"
path = "target/release/libsmearor_network_widget.so"

[network_widget]
width = 280
height = 400
show_status = true
show_scan_list = true
show_airplane_mode = true
show_vpn = true
show_throughput_sparkline = true
show_qr_code = true
max_scan_results = 10
signal_display_mode = "Bars"
button_size = 48
icon_size = 24

# Click opens the network menu area
click_topic = "area.open"
click_payload = { area_id = "network_area" }
```

### 9.3 Minimal Widget Configuration (status + airplane mode only)

```toml
[[scroll_band.plugins]]
id = "network_widget"
path = "target/release/libsmearor_network_widget.so"

[network_widget]
show_status = true
show_scan_list = false
show_airplane_mode = true
show_vpn = false
show_throughput_sparkline = true
show_qr_code = false
```

---

## 10. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Network Menu feature. The order is chosen so that each
layer is built on top of already-tested foundations.

### Phase 1: Foundation — Model Crate (`model/network`)

**Goal:** Define all shared messages, topics, and configuration types.

**Order:**

1. Create the crate `model/network` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_COMMAND`, `TOPIC_STATUS`, `TOPIC_SCAN_RESULTS`, and `TOPIC_VPN_PROFILES`.
3. Create one file per message struct:
    - `src/messages/type_enum.rs` -> `NetworkInterfaceType` enum
    - `src/messages/state.rs` -> `NetworkConnectionState` enum
    - `src/messages/security.rs` -> `WifiSecurity` enum
    - `src/messages/access_point.rs` -> `AccessPointInfo`
    - `src/messages/vpn_profiles.rs` -> `VpnProfileInfo` and `VpnProfilesMessage`
    - `src/messages/interface_status.rs` -> `InterfaceStatus`
    - `src/messages/status.rs` -> `NetworkStatusMessage`
    - `src/messages/scan_results.rs` -> `ScanResultsMessage`
    - `src/messages/command.rs` -> `NetworkCommandAction` and `NetworkCommandMessage`
    - `src/messages/icon.rs` -> `wifi_signal_icon`, `network_interface_icon`, `wifi_security_icon` mapping functions
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.
- The icon mapping functions return correct icon names for all variants.

---

### Phase 2: Backend — Service Crate (`services/network`)

**Goal:** Communicate with NetworkManager via D-Bus and publish network status, scan results, and VPN profiles.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/network` with a `Cargo.toml` that depends on the `model/network` crate, the project plugin API, `zbus`, `tokio`, and `tracing`.
2. Create `src/config.rs` with `NetworkServiceConfig` and its default values.
3. Create `src/dbus.rs` and implement the D-Bus proxy definitions for `org.freedesktop.NetworkManager`, `Device`, `Device.Wireless`, `AccessPoint`,
   `Connection.Active`, and `Settings.Connection`.
4. Implement `refresh_full_status` that queries all devices and active connections and builds a `NetworkStatusMessage`.
5. Create `src/scanner.rs` and implement `perform_wifi_scan` that requests a scan and collects access point info.
6. Create `src/vpn.rs` and implement `refresh_vpn_profiles` and `toggle_vpn`.
7. Create `src/throughput.rs` and implement `collect_throughput` using `/proc/net/dev`.
8. Implement `connect_to_wifi`, `disconnect_device`, and `toggle_radio` helper functions.
9. Create `src/service.rs` with `NetworkService` and all required trait implementations.
10. Implement `run_update_loop` to refresh status at the configured interval, sample throughput, and process incoming commands.
11. Register MCP resources (`network://status`, `network://scan-results`, `network://vpn-profiles`) and MCP tools (`network_toggle_radio`,
    `network_connect_wifi`, `network_toggle_vpn`, `network_get_public_ip`).
12. Wire `service_plugin!` in `src/lib.rs`.
13. Add unit tests for device type mapping, connection state mapping, and security detection.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for mappings produce correct results.
- Running the service broadcasts `TOPIC_STATUS` at least once per refresh interval.
- WLAN scan returns access points sorted by signal strength.
- VPN profiles are correctly listed with active/inactive state.
- MCP resources are registered and return JSON when queried.
- The `network_toggle_radio` tool toggles WLAN/WWan/all radios.
- The `network_connect_wifi` tool connects to a specified SSID.
- The `network_toggle_vpn` tool starts/stops a VPN connection.
- The `network_get_public_ip` tool triggers the HTTP service query.

---

### Phase 3: Display — Widget Crate (`plugins/network`)

**Goal:** Provide a GTK4 network menu with status overview, scan list, VPN toggles, QR code, and airplane mode.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/network` with a `Cargo.toml` that depends on `model/network`, the project plugin API, `gtk4`, `glib`, and optionally `qrcode`.
2. Create `src/config.rs` with `NetworkWidgetConfig` including all visibility flags and `SignalDisplayMode`.
3. Create `src/status_view.rs` and implement the status overview rendering:
    - Interface type icon, SSID, signal strength visualization, IP address.
    - Throughput sparkline using a `gtk4::DrawingArea` custom draw function.
4. Create `src/scan_list.rs` and implement the WLAN scan results list:
    - Each entry shows SSID, signal icon, encryption icon.
    - Connected and known networks are visually distinguished.
    - Click on known network connects immediately.
    - Click on unknown network opens password entry dialog.
5. Create `src/vpn_list.rs` and implement the VPN profile toggle list.
6. Create `src/qr_code.rs` and implement the QR code generation and overlay rendering.
7. Create `src/sparkline.rs` and implement the throughput sparkline drawing.
8. Create `src/widget.rs` with `NetworkWidget` and all required trait implementations.
9. Implement click handling: send `NetworkCommandMessage` with the appropriate action to the service.
10. Subscribe to `TOPIC_STATUS`, `TOPIC_SCAN_RESULTS`, and `TOPIC_VPN_PROFILES` and update state + re-render on every message.
11. Wire `widget_plugin!` in `src/lib.rs`.
12. Add an integration test that verifies the widget accepts all three topics and renders correctly.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The widget displays the status overview with interface icon, SSID, signal, and IP.
- The widget displays the scan list sorted by signal strength.
- The widget shows encryption icons for each access point.
- The widget shows the VPN toggle list with active/inactive state.
- The widget shows the airplane mode toggle.
- The widget shows the throughput sparkline.
- The QR code overlay is generated when clicking the QR button.
- Clicking a known network sends a connect command.
- Clicking an unknown network opens a password dialog.
- Clicking a VPN toggle sends a toggle command.
- Clicking airplane mode sends a toggle radio command.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/network` and `services/network` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `network` in `config.toml`.
4. Add a sample widget configuration for the network widget.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The network widget receives messages and renders correctly.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that `TOPIC_STATUS` appears on the message broker.
2. Verify the widget displays the current connection status.
3. Verify the WLAN scan list populates after requesting a scan.
4. Verify clicking a known network connects successfully.
5. Verify the password dialog works for unknown networks.
6. Verify the airplane mode toggle disables/enables all radios.
7. Verify the VPN toggle list shows all profiles and toggles work.
8. Verify the throughput sparkline updates in real-time.
9. Verify the QR code overlay generates a scannable QR code.
10. Verify MCP resources return valid JSON.
11. Verify the `network_toggle_radio` tool toggles radios correctly.
12. Verify the `network_connect_wifi` tool connects to a specified SSID.
13. Verify the `network_toggle_vpn` tool starts/stops VPN connections.
14. Verify the `network_get_public_ip` tool returns the external IP.
15. Run `cargo test` for all three crates.
16. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- The widget renders correctly for all network states.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.
- D-Bus communication works without requiring sudo or password prompts.
- MCP tools return valid JSON and execute the correct actions.

---

### Summary of Order

```
Phase 1: model/network
    |
    v
Phase 2: services/network
    |
    v
Phase 3: plugins/network
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

### Rationale

- **Model first:** Message formats, enums, and icon mappings must exist before the service or widget can use them.
- **Service second:** The widget needs a running publisher to test against. D-Bus communication with NetworkManager is the core logic.
- **Widget third:** The display widget depends on the service's status, scan results, and VPN profile topics.
- **Wiring fourth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 11. Technical Notes

- **D-Bus over shell commands:** Using `zbus` to communicate with `org.freedesktop.NetworkManager` is the architecturally correct approach. It avoids
  spawning subprocesses (like `nmcli`), is faster, and provides real-time event signals when network state changes.
- **NetworkManager signals:** The service can subscribe to D-Bus signals (`PropertiesChanged`, `DeviceAdded`, `DeviceRemoved`, `AccessPointAdded`,
  `AccessPointRemoved`) for real-time updates instead of polling. This reduces latency and CPU usage.
- **WLAN scan timing:** NetworkManager requires a short delay between scan requests (typically 10-30 seconds). The service should cache scan results and
  avoid spamming scan requests.
- **QR code format:** The QR code uses the standard `WIFI:` URI scheme (`WIFI:S:<SSID>;T:<security>;P:<password>;;`) which is recognized by most mobile
  operating systems (Android, iOS).
- **Throughput sparkline:** The sparkline is rendered as a `gtk4::DrawingArea` with a custom draw function, similar to the Sysinfo widget. History samples
  are kept in a ring buffer.
- **Signal color thresholds:** Green (signal > 60%), Yellow (signal 30-60%), Red (signal < 30%). These thresholds can be made configurable in the future.
- **No polling in the widget:** The widget updates exclusively through incoming messages. Periodic polling only happens in the service.
- **GTK widget ownership:** GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget
  references are captured in `glib::clone!` closures or `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.
- **MCP tool naming:** Tool names use `snake_case` with underscores, never dots. Dots cause schema validation failures in LLM gateways. This is consistent
  with existing tools (`sysinfo_refresh`, `get_current_time`, `weather_refresh`).
- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in `model/power`, `model/notifications`, `model/audio`, and `model/app-launcher`.
- **HTTP service integration:** The `network_get_public_ip` tool delegates to the existing HTTP service to query the external IP address and GeoIP
  information. This avoids duplicating HTTP client logic in the network service.

---

## 12. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/network`, `services/network`, and `plugins/network`.
- **One struct per file:** Each message struct and each enum lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String` and optional strings use
  `stabby::option::Option<stabby::string::String>` to maintain ABI stability across compiler invocations.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `zbus`, `tokio`, and `tracing`; the widget uses `gtk4`, `glib`, and optionally
  `qrcode`.

---

*End of document.*
