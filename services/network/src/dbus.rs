use smearor_network_model::AccessPointInfo;
use smearor_network_model::InterfaceStatus;
use smearor_network_model::NetworkConnectionState;
use smearor_network_model::NetworkInterfaceType;
use smearor_network_model::VpnProfileInfo;
use smearor_network_model::WifiSecurity;
use tracing::debug;
use tracing::error;
use tracing::trace;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

/// Lists all saved connection profiles via the NetworkManager Settings interface.
async fn list_all_connections(connection: &Connection) -> Vec<zbus::zvariant::OwnedObjectPath> {
    match NetworkManagerSettingsProxy::new(connection).await {
        Ok(settings) => match settings.list_connections().await {
            Ok(paths) => paths,
            Err(e) => {
                error!("Network Service: failed to list connections: {e}");
                Vec::new()
            }
        },
        Err(e) => {
            error!("Network Service: failed to create NM Settings proxy: {e}");
            Vec::new()
        }
    }
}

/// D-Bus proxy for `org.freedesktop.NetworkManager`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn wwan_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_wwan_enabled(&self, value: bool) -> zbus::Result<()>;
    fn get_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    #[zbus(property)]
    fn active_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    fn activate_connection(
        &self,
        connection: &zbus::zvariant::ObjectPath<'_>,
        device: &zbus::zvariant::ObjectPath<'_>,
        specific_object: &zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn deactivate_connection(&self, active_connection: &zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;
    fn add_and_activate_connection(
        &self,
        connection: std::collections::HashMap<&str, std::collections::HashMap<&str, zbus::zvariant::Value<'_>>>,
        device: &zbus::zvariant::ObjectPath<'_>,
        specific_object: &zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<(zbus::zvariant::OwnedObjectPath, zbus::zvariant::OwnedObjectPath)>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Settings`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
trait NetworkManagerSettings {
    fn list_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Device`.
#[zbus::proxy(interface = "org.freedesktop.NetworkManager.Device", default_service = "org.freedesktop.NetworkManager")]
trait NetworkDevice {
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn ip4_config(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    #[zbus(property)]
    fn ip6_config(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    #[zbus(property)]
    fn active_connection(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Device.Wireless`.
#[zbus::proxy(interface = "org.freedesktop.NetworkManager.Device.Wireless", default_service = "org.freedesktop.NetworkManager")]
trait WirelessDevice {
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn get_all_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    fn request_scan(&self, options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>) -> zbus::Result<()>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.AccessPoint`.
#[zbus::proxy(interface = "org.freedesktop.NetworkManager.AccessPoint", default_service = "org.freedesktop.NetworkManager")]
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
    fn rsn_flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Connection.Active`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait ActiveConnection {
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn type_(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn uuid(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.IP4Config`.
#[zbus::proxy(interface = "org.freedesktop.NetworkManager.IP4Config", default_service = "org.freedesktop.NetworkManager")]
trait IP4Config {
    #[zbus(property)]
    fn addresses(&self) -> zbus::Result<Vec<String>>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.IP6Config`.
#[zbus::proxy(interface = "org.freedesktop.NetworkManager.IP6Config", default_service = "org.freedesktop.NetworkManager")]
trait IP6Config {
    #[zbus(property)]
    fn addresses(&self) -> zbus::Result<Vec<String>>;
}

/// D-Bus proxy for `org.freedesktop.NetworkManager.Settings.Connection`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings.Connection",
    default_service = "org.freedesktop.NetworkManager"
)]
trait SettingsConnection {
    fn get_settings(&self) -> zbus::Result<std::collections::HashMap<String, std::collections::HashMap<String, OwnedValue>>>;
    fn get_secrets(&self, setting_name: &str) -> zbus::Result<std::collections::HashMap<String, std::collections::HashMap<String, OwnedValue>>>;
}

const NM_DEVICE_TYPE_ETHERNET: u32 = 1;
const NM_DEVICE_TYPE_WIFI: u32 = 2;
const NM_DEVICE_TYPE_BT: u32 = 5;
const NM_DEVICE_TYPE_VPN: u32 = 13;

const NM_DEVICE_STATE_ACTIVATED: u32 = 100;
const NM_DEVICE_STATE_FAILED: u32 = 120;
const NM_DEVICE_STATE_UNAVAILABLE: u32 = 20;
const NM_DEVICE_STATE_DISCONNECTED: u32 = 30;
const NM_DEVICE_STATE_PREPARE: u32 = 40;
const NM_DEVICE_STATE_CONFIG: u32 = 50;
const NM_DEVICE_STATE_NEED_AUTH: u32 = 60;
const NM_DEVICE_STATE_IP_CONFIG: u32 = 70;
const NM_DEVICE_STATE_IP_CHECK: u32 = 80;
const NM_DEVICE_STATE_SECONDARIES: u32 = 90;

fn device_type_from_u32(value: u32) -> NetworkInterfaceType {
    match value {
        NM_DEVICE_TYPE_ETHERNET => NetworkInterfaceType::Ethernet,
        NM_DEVICE_TYPE_WIFI => NetworkInterfaceType::Wifi,
        NM_DEVICE_TYPE_BT => NetworkInterfaceType::Bluetooth,
        NM_DEVICE_TYPE_VPN => NetworkInterfaceType::Vpn,
        _ => NetworkInterfaceType::Other,
    }
}

fn device_state_from_u32(value: u32) -> NetworkConnectionState {
    match value {
        NM_DEVICE_STATE_ACTIVATED => NetworkConnectionState::Connected,
        NM_DEVICE_STATE_FAILED => NetworkConnectionState::Failed,
        NM_DEVICE_STATE_UNAVAILABLE => NetworkConnectionState::Unavailable,
        NM_DEVICE_STATE_DISCONNECTED => NetworkConnectionState::Disconnected,
        NM_DEVICE_STATE_PREPARE
        | NM_DEVICE_STATE_CONFIG
        | NM_DEVICE_STATE_NEED_AUTH
        | NM_DEVICE_STATE_IP_CONFIG
        | NM_DEVICE_STATE_IP_CHECK
        | NM_DEVICE_STATE_SECONDARIES => NetworkConnectionState::Connecting,
        _ => NetworkConnectionState::Disconnected,
    }
}

fn ssid_bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim_matches('\0').to_string()
}

fn security_from_flags(flags: u32, wpa_flags: u32, rsn_flags: u32) -> WifiSecurity {
    const NM_802_11_AP_SEC_NONE: u32 = 0;
    let _ = NM_802_11_AP_SEC_NONE;
    if flags == 0 && wpa_flags == 0 && rsn_flags == 0 {
        return WifiSecurity::Open;
    }
    if rsn_flags != 0 {
        return WifiSecurity::Wpa3;
    }
    if wpa_flags != 0 {
        return WifiSecurity::Wpa;
    }
    if flags != 0 {
        return WifiSecurity::Wep;
    }
    WifiSecurity::Unknown
}

/// Queries all network devices from NetworkManager and returns their status.
pub async fn get_all_interfaces(connection: &Connection) -> Vec<InterfaceStatus> {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy: {e}");
            return Vec::new();
        }
    };

    let device_paths = match manager.get_devices().await {
        Ok(paths) => paths,
        Err(e) => {
            error!("Network Service: failed to get devices: {e}");
            return Vec::new();
        }
    };

    let mut interfaces = Vec::new();

    for path in device_paths {
        let device = match NetworkDeviceProxy::new(connection, path.clone()).await {
            Ok(d) => d,
            Err(e) => {
                debug!("Network Service: failed to create device proxy for {path}: {e}");
                continue;
            }
        };

        let interface_name = device.interface().await.unwrap_or_default();
        let device_type_u32 = device.device_type().await.unwrap_or(0);
        let state_u32 = device.state().await.unwrap_or(0);
        let interface_type = device_type_from_u32(device_type_u32);
        let state = device_state_from_u32(state_u32);

        let internet_accessible = state == NetworkConnectionState::Connected;
        let mut status = InterfaceStatus {
            interface_type,
            interface_name: stabby::string::String::from(interface_name),
            state,
            ssid: stabby::option::Option::None(),
            signal: stabby::option::Option::None(),
            ipv4_address: stabby::option::Option::None(),
            ipv6_address: stabby::option::Option::None(),
            internet_accessible,
            wifi_password: stabby::option::Option::None(),
        };

        if interface_type == NetworkInterfaceType::Wifi
            && let Ok(wireless) = WirelessDeviceProxy::new(connection, path.clone()).await
            && let Ok(ap_path) = wireless.active_access_point().await
            && !ap_path.as_str().is_empty()
            && let Ok(ap) = AccessPointProxy::new(connection, ap_path.clone()).await
        {
            let ssid_bytes = ap.ssid().await.unwrap_or_default();
            let ssid_str = ssid_bytes_to_string(&ssid_bytes);
            if !ssid_str.is_empty() {
                status.ssid = stabby::option::Option::Some(stabby::string::String::from(ssid_str.clone()));
                if let Some(psk) = get_wifi_password(connection, &ssid_str).await {
                    status.wifi_password = stabby::option::Option::Some(stabby::string::String::from(psk));
                }
            }
            let signal = ap.strength().await.unwrap_or(0);
            status.signal = stabby::option::Option::Some(signal);
        }

        if let Ok(ip4_path) = device.ip4_config().await
            && !ip4_path.as_str().is_empty()
            && let Ok(ip4_config) = IP4ConfigProxy::new(connection, ip4_path).await
            && let Ok(addresses) = ip4_config.addresses().await
            && let Some(first_addr) = addresses.first()
        {
            let ip = first_addr.split('/').next().unwrap_or(first_addr);
            status.ipv4_address = stabby::option::Option::Some(stabby::string::String::from(ip));
        }

        if let Ok(ip6_path) = device.ip6_config().await
            && !ip6_path.as_str().is_empty()
            && let Ok(ip6_config) = IP6ConfigProxy::new(connection, ip6_path).await
            && let Ok(addresses) = ip6_config.addresses().await
            && let Some(first_addr) = addresses.first()
        {
            let ip = first_addr.split('/').next().unwrap_or(first_addr);
            status.ipv6_address = stabby::option::Option::Some(stabby::string::String::from(ip));
        }

        interfaces.push(status);
    }

    interfaces
}

/// Queries the radio enabled state from NetworkManager.
pub async fn get_radio_state(connection: &Connection) -> (bool, bool) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for radio state: {e}");
            return (false, false);
        }
    };
    let wifi_enabled = manager.wireless_enabled().await.unwrap_or(false);
    let wwan_enabled = manager.wwan_enabled().await.unwrap_or(false);
    (wifi_enabled, wwan_enabled)
}

/// Sets the wireless enabled state.
pub async fn set_wireless_enabled(connection: &Connection, enabled: bool) {
    if let Ok(proxy) = NetworkManagerProxy::new(connection).await
        && let Err(e) = proxy.set_wireless_enabled(enabled).await
    {
        error!("Network Service: failed to set wireless_enabled={enabled}: {e}");
    }
}

/// Sets the WWAN enabled state.
pub async fn set_wwan_enabled(connection: &Connection, enabled: bool) {
    if let Ok(proxy) = NetworkManagerProxy::new(connection).await
        && let Err(e) = proxy.set_wwan_enabled(enabled).await
    {
        error!("Network Service: failed to set wwan_enabled={enabled}: {e}");
    }
}

/// Requests a WLAN scan on all wireless devices.
pub async fn request_wifi_scan(connection: &Connection) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for scan: {e}");
            return;
        }
    };

    let device_paths = match manager.get_devices().await {
        Ok(paths) => paths,
        Err(e) => {
            error!("Network Service: failed to get devices for scan: {e}");
            return;
        }
    };

    for path in device_paths {
        let device = match NetworkDeviceProxy::new(connection, path.clone()).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let device_type_u32 = device.device_type().await.unwrap_or(0);
        if device_type_u32 == NM_DEVICE_TYPE_WIFI
            && let Ok(wireless) = WirelessDeviceProxy::new(connection, path.clone()).await
        {
            let options = std::collections::HashMap::new();
            if let Err(e) = wireless.request_scan(options).await {
                debug!("Network Service: scan request failed on {path}: {e}");
            }
        }
    }
}

/// Retrieves all WLAN access points from all wireless devices.
pub async fn get_all_access_points(connection: &Connection) -> Vec<AccessPointInfo> {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for AP scan: {e}");
            return Vec::new();
        }
    };

    let device_paths = match manager.get_devices().await {
        Ok(paths) => paths,
        Err(e) => {
            error!("Network Service: failed to get devices for AP scan: {e}");
            return Vec::new();
        }
    };

    let mut all_aps: Vec<AccessPointInfo> = Vec::new();

    for path in device_paths {
        let device = match NetworkDeviceProxy::new(connection, path.clone()).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let device_type_u32 = device.device_type().await.unwrap_or(0);
        if device_type_u32 != NM_DEVICE_TYPE_WIFI {
            continue;
        }

        let wireless = match WirelessDeviceProxy::new(connection, path.clone()).await {
            Ok(w) => w,
            Err(_) => continue,
        };

        let active_ap_path = wireless.active_access_point().await.unwrap_or_default();
        let active_ap_str = active_ap_path.as_str().to_string();

        let ap_paths = match wireless.get_all_access_points().await {
            Ok(paths) => paths,
            Err(e) => {
                debug!("Network Service: failed to get APs from {path}: {e}");
                continue;
            }
        };

        let known_connections = get_known_ssids(connection).await;

        for ap_path in ap_paths {
            let ap = match AccessPointProxy::new(connection, ap_path.clone()).await {
                Ok(ap) => ap,
                Err(_) => continue,
            };

            let ssid_bytes = ap.ssid().await.unwrap_or_default();
            let ssid_str = ssid_bytes_to_string(&ssid_bytes);
            if ssid_str.is_empty() {
                continue;
            }

            let bssid = ap.bssid().await.unwrap_or_default();
            let signal = ap.strength().await.unwrap_or(0);
            let frequency = ap.frequency().await.unwrap_or(0);
            let flags = ap.flags().await.unwrap_or(0);
            let wpa_flags = ap.wpa_flags().await.unwrap_or(0);
            let rsn_flags = ap.rsn_flags().await.unwrap_or(0);
            let security = security_from_flags(flags, wpa_flags, rsn_flags);
            let is_connected = ap_path.as_str() == active_ap_str;
            let is_known = known_connections.contains(&ssid_str);

            all_aps.push(AccessPointInfo {
                ssid: stabby::string::String::from(ssid_str),
                bssid: stabby::string::String::from(bssid),
                signal,
                frequency,
                security,
                is_connected,
                is_known,
            });
        }
    }

    all_aps.sort_by_key(|ap| std::cmp::Reverse(ap.signal));
    all_aps
}

/// Retrieves the SSIDs of all saved (known) connections from NetworkManager settings.
async fn get_known_ssids(connection: &Connection) -> Vec<String> {
    let conn_paths = list_all_connections(connection).await;

    let mut ssids = Vec::new();

    for path in conn_paths {
        if let Ok(settings_conn) = SettingsConnectionProxy::new(connection, path).await
            && let Ok(settings) = settings_conn.get_settings().await
        {
            if let Some(connection_map) = settings.get("connection")
                && let Some(v) = connection_map.get("id")
                && let zbus::zvariant::Value::Str(id) = &**v
            {
                ssids.push(id.to_string());
            }
            if let Some(wireless_map) = settings.get("802-11-wireless")
                && let Some(v) = wireless_map.get("ssid")
                && let zbus::zvariant::Value::Array(ssid_arr) = &**v
            {
                let bytes: Vec<u8> = ssid_arr
                    .iter()
                    .filter_map(|v| if let zbus::zvariant::Value::U8(b) = v { Some(*b) } else { None })
                    .collect();
                let ssid_str = ssid_bytes_to_string(&bytes);
                if !ssid_str.is_empty() {
                    ssids.push(ssid_str);
                }
            }
        }
    }

    ssids
}

/// Retrieves the WiFi password (PSK) for a given SSID from NetworkManager settings.
pub async fn get_wifi_password(connection: &Connection, target_ssid: &str) -> Option<String> {
    let conn_paths = list_all_connections(connection).await;

    for path in conn_paths {
        if let Ok(settings_conn) = SettingsConnectionProxy::new(connection, path.clone()).await
            && let Ok(settings) = settings_conn.get_settings().await
            && let Some(wireless_map) = settings.get("802-11-wireless")
            && let Some(v) = wireless_map.get("ssid")
            && let zbus::zvariant::Value::Array(ssid_arr) = &**v
        {
            let bytes: Vec<u8> = ssid_arr
                .iter()
                .filter_map(|v| if let zbus::zvariant::Value::U8(b) = v { Some(*b) } else { None })
                .collect();
            let ssid_str = ssid_bytes_to_string(&bytes);
            if ssid_str == target_ssid {
                if let Ok(secrets) = settings_conn.get_secrets("802-11-wireless-security").await
                    && let Some(security_map) = secrets.get("802-11-wireless-security")
                    && let Some(psk_value) = security_map.get("psk")
                    && let zbus::zvariant::Value::Str(psk) = &**psk_value
                {
                    return Some(psk.to_string());
                }
                return None;
            }
        }
    }
    None
}

/// Retrieves all VPN connection profiles from NetworkManager.
pub async fn get_vpn_profiles(connection: &Connection) -> Vec<VpnProfileInfo> {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for VPN: {e}");
            return Vec::new();
        }
    };

    let all_conn_paths = list_all_connections(connection).await;

    let active_conn_paths = manager.active_connections().await.unwrap_or_default();
    trace!("Network Service: active_connections returned {} paths: {:?}", active_conn_paths.len(), active_conn_paths);

    let mut active_uuids: Vec<String> = Vec::new();
    for path in &active_conn_paths {
        match ActiveConnectionProxy::new(connection, path.clone()).await {
            Ok(active) => {
                let conn_type = active.type_().await.unwrap_or_default();
                trace!("Network Service: active connection path={} type={}", path, conn_type);
                if conn_type == "vpn" || conn_type == "wireguard" {
                    let uuid = active.uuid().await.unwrap_or_default();
                    trace!("Network Service: active VPN uuid={}", uuid);
                    active_uuids.push(uuid);
                }
            }
            Err(e) => {
                trace!("Network Service: failed to create ActiveConnectionProxy for {}: {e}", path);
            }
        }
    }

    let mut profiles = Vec::new();

    for path in all_conn_paths {
        if let Ok(settings_conn) = SettingsConnectionProxy::new(connection, path.clone()).await
            && let Ok(settings) = settings_conn.get_settings().await
            && let Some(connection_map) = settings.get("connection")
            && let Some(v) = connection_map.get("type")
            && let zbus::zvariant::Value::Str(conn_type) = &**v
        {
            let conn_type_str = conn_type.to_string();
            if conn_type_str == "vpn" || conn_type_str == "wireguard" {
                let id = connection_map
                    .get("id")
                    .and_then(|v| match &**v {
                        zbus::zvariant::Value::Str(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let uuid = connection_map
                    .get("uuid")
                    .and_then(|v| match &**v {
                        zbus::zvariant::Value::Str(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let is_active = active_uuids.contains(&uuid);
                debug!("Network Service: VPN profile id={} uuid={} is_active={} active_uuids={:?}", id, uuid, is_active, active_uuids);
                profiles.push(VpnProfileInfo {
                    name: stabby::string::String::from(id),
                    vpn_type: stabby::string::String::from(conn_type_str),
                    is_active,
                    uuid: stabby::string::String::from(uuid),
                });
            }
        }
    }

    profiles
}

/// Activates a VPN connection by UUID.
pub async fn activate_vpn(connection: &Connection, uuid: &str) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for VPN activate: {e}");
            return;
        }
    };

    let all_conn_paths = list_all_connections(connection).await;

    for path in all_conn_paths {
        if let Ok(settings_conn) = SettingsConnectionProxy::new(connection, path.clone()).await
            && let Ok(settings) = settings_conn.get_settings().await
            && let Some(connection_map) = settings.get("connection")
        {
            let conn_uuid = connection_map
                .get("uuid")
                .and_then(|v| match &**v {
                    zbus::zvariant::Value::Str(s) => Some(s.to_string()),
                    _ => None,
                })
                .unwrap_or_default();
            if conn_uuid == uuid {
                let empty_path = zbus::zvariant::ObjectPath::try_from("/").unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
                if let Err(e) = manager.activate_connection(&path, &empty_path, &empty_path).await {
                    error!("Network Service: failed to activate VPN {uuid}: {e}");
                } else {
                    debug!("Network Service: activated VPN {uuid}");
                }
                return;
            }
        }
    }
    debug!("Network Service: no VPN connection found for uuid {uuid}");
}

/// Deactivates an active VPN connection by UUID.
pub async fn deactivate_vpn(connection: &Connection, uuid: &str) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for VPN deactivate: {e}");
            return;
        }
    };

    let active_conn_paths = match manager.active_connections().await {
        Ok(paths) => paths,
        Err(e) => {
            error!("Network Service: failed to get active connections for VPN deactivate: {e}");
            return;
        }
    };

    for path in active_conn_paths {
        if let Ok(active) = ActiveConnectionProxy::new(connection, path.clone()).await {
            let active_uuid = active.uuid().await.unwrap_or_default();
            if active_uuid == uuid {
                let obj_path = zbus::zvariant::ObjectPath::try_from(path.as_str()).unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
                if let Err(e) = manager.deactivate_connection(&obj_path).await {
                    error!("Network Service: failed to deactivate VPN {uuid}: {e}");
                }
                return;
            }
        }
    }
}

/// Connects to a WiFi network by SSID. If the network is known (saved in NetworkManager),
/// activates the existing connection profile. Otherwise, creates and activates a new connection
/// with the provided password.
pub async fn connect_to_wifi(connection: &Connection, ssid: &str, password: Option<&str>) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for connect: {e}");
            return;
        }
    };

    let known_ssids = get_known_ssids(connection).await;
    let is_known = known_ssids.iter().any(|s| s == ssid);

    let wifi_device_path = match get_wifi_device_path(connection).await {
        Some(path) => path,
        None => {
            error!("Network Service: no WiFi device found for connect");
            return;
        }
    };

    if is_known {
        debug!("Network Service: connecting to known SSID {ssid}");
        let all_conn_paths = list_all_connections(connection).await;

        for path in all_conn_paths {
            if let Ok(settings_conn) = SettingsConnectionProxy::new(connection, path.clone()).await
                && let Ok(settings) = settings_conn.get_settings().await
                && let Some(wireless_map) = settings.get("802-11-wireless")
                && let Some(v) = wireless_map.get("ssid")
                && let zbus::zvariant::Value::Array(ssid_arr) = &**v
            {
                let bytes: Vec<u8> = ssid_arr
                    .iter()
                    .filter_map(|v| if let zbus::zvariant::Value::U8(b) = v { Some(*b) } else { None })
                    .collect();
                let conn_ssid = ssid_bytes_to_string(&bytes);
                if conn_ssid == ssid {
                    let empty_path = zbus::zvariant::ObjectPath::try_from("/").unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
                    let device_path =
                        zbus::zvariant::ObjectPath::try_from(wifi_device_path.as_str()).unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
                    if let Err(e) = manager.activate_connection(&path, &device_path, &empty_path).await {
                        error!("Network Service: failed to activate connection for {ssid}: {e}");
                    }
                    return;
                }
            }
        }
        error!("Network Service: SSID {ssid} was in known list but no matching connection profile found");
    } else {
        debug!("Network Service: adding and activating new connection for SSID {ssid}");
        let ssid_bytes: Vec<u8> = ssid.as_bytes().to_vec();

        let mut connection_map = std::collections::HashMap::new();
        let mut conn_section = std::collections::HashMap::new();
        conn_section.insert("type", zbus::zvariant::Value::new("802-11-wireless"));
        conn_section.insert("id", zbus::zvariant::Value::new(ssid));
        connection_map.insert("connection", conn_section);

        let mut wireless_section = std::collections::HashMap::new();
        let ssid_variant: zbus::zvariant::Value =
            zbus::zvariant::Value::Array(zbus::zvariant::Array::from(ssid_bytes.iter().map(|b| zbus::zvariant::Value::U8(*b)).collect::<Vec<_>>()));
        wireless_section.insert("ssid", ssid_variant);
        if let Some(pw) = password {
            let mut security_section = std::collections::HashMap::new();
            security_section.insert("key-mgmt", zbus::zvariant::Value::new("wpa-psk"));
            security_section.insert("psk", zbus::zvariant::Value::new(pw));
            connection_map.insert("802-11-wireless-security", security_section);
        }
        connection_map.insert("802-11-wireless", wireless_section);

        let empty_path = zbus::zvariant::ObjectPath::try_from("/").unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
        let device_path = zbus::zvariant::ObjectPath::try_from(wifi_device_path.as_str()).unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));

        if let Err(e) = manager.add_and_activate_connection(connection_map, &device_path, &empty_path).await {
            error!("Network Service: failed to add and activate connection for {ssid}: {e}");
        }
    }
}

/// Returns the D-Bus object path of the first WiFi device, if any.
pub async fn get_wifi_device_path(connection: &Connection) -> Option<zbus::zvariant::OwnedObjectPath> {
    let manager = NetworkManagerProxy::new(connection).await.ok()?;
    let device_paths = manager.get_devices().await.ok()?;
    for path in device_paths {
        if let Ok(device) = NetworkDeviceProxy::new(connection, path.clone()).await {
            let device_type_u32 = device.device_type().await.unwrap_or(0);
            if device_type_u32 == NM_DEVICE_TYPE_WIFI {
                return Some(path);
            }
        }
    }
    None
}

/// Returns the D-Bus object path of the device with the given interface name, if any.
async fn get_device_path_by_interface(connection: &Connection, interface_name: &str) -> Option<zbus::zvariant::OwnedObjectPath> {
    let manager = NetworkManagerProxy::new(connection).await.ok()?;
    let device_paths = manager.get_devices().await.ok()?;
    for path in device_paths {
        if let Ok(device) = NetworkDeviceProxy::new(connection, path.clone()).await {
            if let Ok(iface) = device.interface().await {
                if iface.as_str() == interface_name {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Activates a connection profile for the given interface name (e.g., Ethernet reconnect).
/// Finds the first saved connection profile whose `connection.interface-name` matches
/// the given interface name and activates it on the corresponding device.
pub async fn connect_interface(connection: &Connection, interface_name: &str) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for connect_interface: {e}");
            return;
        }
    };

    let connection_paths = list_all_connections(connection).await;

    let device_path = match get_device_path_by_interface(connection, interface_name).await {
        Some(path) => path,
        None => {
            error!("Network Service: no device found for interface {interface_name}");
            return;
        }
    };

    let empty_path = zbus::zvariant::ObjectPath::try_from("/").unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));

    for conn_path in connection_paths {
        let settings = match SettingsConnectionProxy::new(connection, conn_path.clone()).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let settings_map = match settings.get_settings().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(conn_settings) = settings_map.get("connection")
            && let Some(iface_value) = conn_settings.get("interface-name")
        {
            let conn_iface = iface_value.to_string().trim_matches('"').to_string();
            if conn_iface == interface_name {
                if let Err(e) = manager.activate_connection(&conn_path, &device_path, &empty_path).await {
                    error!("Network Service: failed to activate connection for {interface_name}: {e}");
                }
                return;
            }
        }
    }
    debug!("Network Service: no saved connection profile found for interface {interface_name}");
}

/// Deactivates the active connection on the primary wireless device.
pub async fn disconnect_wifi(connection: &Connection) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for disconnect: {e}");
            return;
        }
    };

    let device_paths = match manager.get_devices().await {
        Ok(paths) => paths,
        Err(e) => {
            error!("Network Service: failed to get devices for disconnect: {e}");
            return;
        }
    };

    for path in device_paths {
        let device = match NetworkDeviceProxy::new(connection, path.clone()).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let device_type_u32 = device.device_type().await.unwrap_or(0);
        if device_type_u32 == NM_DEVICE_TYPE_WIFI
            && let Ok(active_conn_path) = device.active_connection().await
            && !active_conn_path.as_str().is_empty()
        {
            let obj_path =
                zbus::zvariant::ObjectPath::try_from(active_conn_path.as_str()).unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
            if let Err(e) = manager.deactivate_connection(&obj_path).await {
                error!("Network Service: failed to disconnect WiFi: {e}");
            }
            return;
        }
    }
}

/// Deactivates the active connection on a specific interface by interface name.
pub async fn disconnect_interface(connection: &Connection, interface_name: &str) {
    let manager = match NetworkManagerProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            error!("Network Service: failed to create NM proxy for disconnect: {e}");
            return;
        }
    };

    let device_paths = match manager.get_devices().await {
        Ok(paths) => paths,
        Err(e) => {
            error!("Network Service: failed to get devices for disconnect: {e}");
            return;
        }
    };

    for path in device_paths {
        let device = match NetworkDeviceProxy::new(connection, path.clone()).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let iface = device.interface().await.unwrap_or_default();
        if iface.as_str() == interface_name
            && let Ok(active_conn_path) = device.active_connection().await
            && !active_conn_path.as_str().is_empty()
        {
            let obj_path =
                zbus::zvariant::ObjectPath::try_from(active_conn_path.as_str()).unwrap_or(zbus::zvariant::ObjectPath::from_static_str_unchecked("/"));
            if let Err(e) = manager.deactivate_connection(&obj_path).await {
                error!("Network Service: failed to disconnect {interface_name}: {e}");
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_from_u32() {
        assert_eq!(device_type_from_u32(NM_DEVICE_TYPE_ETHERNET), NetworkInterfaceType::Ethernet);
        assert_eq!(device_type_from_u32(NM_DEVICE_TYPE_WIFI), NetworkInterfaceType::Wifi);
        assert_eq!(device_type_from_u32(NM_DEVICE_TYPE_BT), NetworkInterfaceType::Bluetooth);
        assert_eq!(device_type_from_u32(NM_DEVICE_TYPE_VPN), NetworkInterfaceType::Vpn);
        assert_eq!(device_type_from_u32(0), NetworkInterfaceType::Other);
        assert_eq!(device_type_from_u32(99), NetworkInterfaceType::Other);
    }

    #[test]
    fn test_device_state_from_u32() {
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_ACTIVATED), NetworkConnectionState::Connected);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_FAILED), NetworkConnectionState::Failed);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_UNAVAILABLE), NetworkConnectionState::Unavailable);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_DISCONNECTED), NetworkConnectionState::Disconnected);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_PREPARE), NetworkConnectionState::Connecting);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_CONFIG), NetworkConnectionState::Connecting);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_NEED_AUTH), NetworkConnectionState::Connecting);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_IP_CONFIG), NetworkConnectionState::Connecting);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_IP_CHECK), NetworkConnectionState::Connecting);
        assert_eq!(device_state_from_u32(NM_DEVICE_STATE_SECONDARIES), NetworkConnectionState::Connecting);
        assert_eq!(device_state_from_u32(0), NetworkConnectionState::Disconnected);
        assert_eq!(device_state_from_u32(200), NetworkConnectionState::Disconnected);
    }

    #[test]
    fn test_security_from_flags() {
        assert_eq!(security_from_flags(0, 0, 0), WifiSecurity::Open);
        assert_eq!(security_from_flags(0, 0, 0x1), WifiSecurity::Wpa3);
        assert_eq!(security_from_flags(0, 0x1, 0), WifiSecurity::Wpa);
        assert_eq!(security_from_flags(0x1, 0, 0), WifiSecurity::Wep);
        assert_eq!(security_from_flags(0x1, 0x1, 0x1), WifiSecurity::Wpa3);
        assert_eq!(security_from_flags(0x1, 0x1, 0), WifiSecurity::Wpa);
    }

    #[test]
    fn test_ssid_bytes_to_string() {
        assert_eq!(ssid_bytes_to_string(b"MyWiFi"), "MyWiFi");
        assert_eq!(ssid_bytes_to_string(b""), "");
        assert_eq!(ssid_bytes_to_string(b"\0\0"), "");
        assert_eq!(ssid_bytes_to_string(b"Net\0"), "Net");
    }
}
