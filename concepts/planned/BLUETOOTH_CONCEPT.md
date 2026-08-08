# Concept: Bluetooth Service & Widget

This document describes the concept for a **Bluetooth Service**, a **Bluetooth Widget**, and the shared **Airplane Mode Coordination** between Network and
Bluetooth services. All components follow the decoupled architecture of the *Smearor Swipe Launcher*.

---

## 1. Motivation

Bluetooth is a separate subsystem from NetworkManager, managed by **BlueZ** via D-Bus. To maintain clean separation of concerns, Bluetooth functionality is
implemented in dedicated crates (`model/bluetooth`, `services/bluetooth`, `plugins/bluetooth`), analogous to the Network crates.

The **Airplane Mode** feature must coordinate both services: turning off WiFi/WWAN (Network service) and turning off Bluetooth (Bluetooth service)
simultaneously. This is achieved through a shared `airplane_mode` command topic that both services listen to.

---

## 2. Crate Structure

| Crate       | Path                  | Responsibility                                       |
|-------------|-----------------------|------------------------------------------------------|
| **Model**   | `model/bluetooth/`    | Shared structs, enums, message formats, FFI types    |
| **Service** | `services/bluetooth/` | BlueZ D-Bus integration, status broadcasts, commands |
| **Widget**  | `plugins/bluetooth/`  | GTK4 tile widget with view-based rotation            |

---

## 3. Model Crate (`model/bluetooth`)

### 3.1 Message Topics

```rust
pub const TOPIC_STATUS: &str = "service.bluetooth.status";
pub const TOPIC_SCAN_RESULTS: &str = "service.bluetooth.scan_results";
pub const TOPIC_COMMAND: &str = "service.bluetooth.command";
pub const TOPIC_AIRPLANE: &str = "service.bluetooth.airplane";
pub const TOPIC_DEVICE_EVENT: &str = "service.bluetooth.device_event";
pub const TOPIC_PROXIMITY: &str = "service.bluetooth.proximity";
```

### 3.2 Bluetooth Status Message

```rust
/// Status message for Bluetooth adapter and connected devices.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct BluetoothStatusMessage {
    /// Whether Bluetooth is powered on.
    pub powered: bool,
    /// Whether the adapter is discoverable by other devices.
    pub discoverable: bool,
    /// Whether a device discovery scan is currently active.
    pub discovering: bool,
    /// List of currently connected devices.
    pub connected_devices: StabbyVec<DeviceStatus>,
    /// Adapter address (e.g., "AA:BB:CC:DD:EE:FF").
    pub adapter_address: StabbyString,
    /// Adapter name (human-readable, from BlueZ).
    pub adapter_name: StabbyString,
    /// Timestamp of the last status update (ISO 8601).
    pub last_updated: StabbyString,
    /// Whether auto-accept pairing is currently enabled.
    /// Toggled by the user via the widget; reflected in status for UI feedback.
    pub auto_accept_pairing: bool,
}
```

### 3.3 Device Status

```rust
/// Status of a single Bluetooth device.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DeviceStatus {
    /// Human-readable device name (e.g., "Sony WH-1000XM5").
    pub name: StabbyString,
    /// Bluetooth device address (e.g., "AA:BB:CC:DD:EE:FF").
    pub address: StabbyString,
    /// Device type icon name from BlueZ (e.g., "audio-headphones", "input-keyboard").
    pub device_type: StabbyString,
    /// Whether the device is currently connected.
    pub connected: bool,
    /// Whether the device is paired.
    pub paired: bool,
    /// Battery level in percent (0-100), if reported by the device.
    pub battery_level: StabbyOption<u8>,
    /// Whether data is currently being transferred (e.g., file transfer active).
    pub transferring: bool,
}
```

### 3.4 Scan Results Message

```rust
/// Scan results message containing discovered Bluetooth devices.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ScanResultsMessage {
    /// List of discovered devices (may include already-paired devices).
    pub devices: StabbyVec<DeviceStatus>,
    /// Timestamp of the scan (ISO 8601).
    pub scan_time: StabbyString,
}
```

### 3.5 Command Message

```rust
/// Actions that the Bluetooth service can perform.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct BluetoothCommandMessage {
    /// The action to perform.
    pub action: BluetoothCommandAction,
    /// Device address for device-specific actions.
    pub address: StabbyOption<StabbyString>,
    /// Target state for the action. Semantics depend on `action`:
    /// - `TogglePower`: `true` = power on, `false` = power off.
    /// - `ToggleDiscoverable`: `true` = discoverable on, `false` = off.
    /// - `AirplaneMode`: `true` = airplane mode active (Bluetooth OFF), `false` = inactive (Bluetooth ON).
    pub enabled: bool,
}

/// Available Bluetooth command actions.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum BluetoothCommandAction {
    /// Toggle Bluetooth power on/off.
    #[default]
    TogglePower,
    /// Toggle discoverable mode on/off.
    ToggleDiscoverable,
    /// Start a device discovery scan.
    StartScan,
    /// Stop an ongoing device discovery scan.
    StopScan,
    /// Connect to a device by address.
    ConnectDevice,
    /// Disconnect from a device by address.
    DisconnectDevice,
    /// Pair with a device by address.
    PairDevice,
    /// Remove a paired device by address.
    RemoveDevice,
    /// Set airplane mode. `enabled = true` means airplane mode ON (Bluetooth powered off).
    /// `enabled = false` means airplane mode OFF (Bluetooth powered on).
    /// Shared with Network service for coordinated airplane mode toggling.
    AirplaneMode,
    /// Toggle auto-accept pairing mode at runtime.
    /// When enabled, SSP pairing confirmations are auto-accepted.
    /// When disabled, all pairing requests are rejected.
    ToggleAutoAccept,
}
```

### 3.6 Bluetooth View Enum

```rust
/// Available Bluetooth views that the widget can display.
/// Each variant corresponds to a data category rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum BluetoothView {
    /// Bluetooth power status: on/off, adapter name.
    /// Clicking the tile in this view toggles Bluetooth power.
    #[default]
    PowerStatus,
    /// Connected devices: shows the first connected device name and type.
    /// Clicking the tile in this view disconnects the first connected device.
    ConnectedDevices,
    /// Scan results: count of discovered devices.
    /// Clicking the tile in this view starts a scan.
    /// Long-pressing the tile in this view toggles auto-accept pairing mode.
    ScanResults,
    /// Airplane mode status: on or off.
    /// Clicking the tile in this view toggles airplane mode (coordinates with Network service).
    Airplane,
    /// Battery status: shows battery level of the first connected device that reports it.
    Battery,
}
```

### 3.7 Device Type Icon Mapping

BlueZ reports device type icons as strings (e.g., `"audio-headphones"`, `"input-keyboard"`, `"phone"`). The model crate provides a mapping from BlueZ device
type strings to `BluetoothDeviceType` enum variants, which the widget uses to select dedicated Nerd Font icons.

```rust
/// Categorized Bluetooth device types based on BlueZ icon names.
/// Used by the widget to select appropriate Nerd Font icons.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum BluetoothDeviceType {
    /// Audio headphones or headset (BlueZ: "audio-headphones", "audio-headset").
    AudioHeadphones,
    /// Audio speaker (BlueZ: "audio-speaker").
    AudioSpeaker,
    /// Keyboard input device (BlueZ: "input-keyboard", "input-keyboard-mouse").
    InputKeyboard,
    /// Mouse or pointing device (BlueZ: "input-mouse", "input-tablet").
    InputMouse,
    /// Gaming controller (BlueZ: "input-gaming").
    InputGaming,
    /// Phone device (BlueZ: "phone").
    Phone,
    /// Computer or laptop (BlueZ: "computer", "laptop").
    Computer,
    /// Camera device (BlueZ: "camera", "video-display").
    Camera,
    /// Printer or scanner (BlueZ: "printer", "scanner").
    Printer,
    /// Wearable device (BlueZ: "wearable", "watch").
    Wearable,
    /// Network access point (BlueZ: "network-router").
    NetworkAccessPoint,
    /// Unknown or unmapped device type.
    #[default]
    Unknown,
}

impl BluetoothDeviceType {
    /// Maps a BlueZ device type icon string to a `BluetoothDeviceType` variant.
    /// Performs case-insensitive substring matching against known BlueZ icon names.
    pub fn from_bluez_icon(icon: &str) -> Self {
        let icon = icon.to_lowercase();
        if icon.contains("headphone") || icon.contains("headset") {
            Self::AudioHeadphones
        } else if icon.contains("speaker") {
            Self::AudioSpeaker
        } else if icon.contains("keyboard") {
            Self::InputKeyboard
        } else if icon.contains("mouse") || icon.contains("tablet") {
            Self::InputMouse
        } else if icon.contains("gaming") {
            Self::InputGaming
        } else if icon.contains("phone") {
            Self::Phone
        } else if icon.contains("computer") || icon.contains("laptop") {
            Self::Computer
        } else if icon.contains("camera") || icon.contains("video") {
            Self::Camera
        } else if icon.contains("printer") || icon.contains("scanner") {
            Self::Printer
        } else if icon.contains("wearable") || icon.contains("watch") {
            Self::Wearable
        } else if icon.contains("router") || icon.contains("network") {
            Self::NetworkAccessPoint
        } else {
            Self::Unknown
        }
    }
}
```

### 3.7a Device Event Message

Emitted by the service when a device connects or disconnects. Used by:

- **Bluetooth-Automation**: triggers `on_connect`/`on_disconnect` action lists (see Section 4.9)
- **Audio-Routing-Integration**: Audio Service listens for audio device connects to switch output sink (see Section 6.5)
- **Proximity Actions**: disconnect events with `reason = OutOfRange` trigger proximity handling (see Section 6.6)

```rust
/// Event type for device connect/disconnect.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum BluetoothDeviceEventType {
    /// Device connected.
    #[default]
    Connected,
    /// Device disconnected.
    Disconnected,
}

/// Reason for a device disconnect event.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum BluetoothDisconnectReason {
    /// Explicit disconnect by user or service.
    #[default]
    Explicit,
    /// Device went out of range (signal lost).
    OutOfRange,
}

/// Device event message broadcast on `TOPIC_DEVICE_EVENT`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DeviceEventMessage {
    /// Type of event (Connected or Disconnected).
    pub event_type: BluetoothDeviceEventType,
    /// Device address (e.g., "AA:BB:CC:DD:EE:FF").
    pub address: StabbyString,
    /// Device name (human-readable).
    pub name: StabbyString,
    /// Device type icon name from BlueZ.
    pub device_type: StabbyString,
    /// Reason for disconnect (only set for Disconnected events).
    pub disconnect_reason: StabbyOption<BluetoothDisconnectReason>,
}
```

### 3.7b Proximity Event Message

Emitted by the service when a device's proximity state changes (enters or leaves range). Broadcast on `TOPIC_PROXIMITY`. Other services and widgets can
subscribe to trigger actions such as screen locking or music pausing.

```rust
/// Proximity state for a Bluetooth device.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum ProximityState {
    /// Device is in range and reachable.
    #[default]
    InRange,
    /// Device has gone out of range.
    OutOfRange,
}

/// Proximity event message broadcast on `TOPIC_PROXIMITY`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DeviceProximityEvent {
    /// Device address.
    pub address: StabbyString,
    /// Device name (human-readable).
    pub name: StabbyString,
    /// Current proximity state.
    pub state: ProximityState,
    /// RSSI value in dBm at the time of the event (if available).
    pub rssi: StabbyOption<i16>,
}
```

### 3.8 JSON Converters

The model crate uses the `impl_json_convertible!` macro for FFI serialization, analogous to `model/app-launcher/src/lib.rs`. Manual `parse_*` functions are
forbidden. All structs must derive `Default` for deserialization fallback.

In `lib.rs`:

```rust
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(BluetoothStatusMessageConverter, BluetoothStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(ScanResultsMessageConverter, ScanResultsMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(BluetoothCommandMessageConverter, BluetoothCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(DeviceEventMessageConverter, DeviceEventMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(DeviceProximityEventConverter, DeviceProximityEvent, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for Bluetooth messages.
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    BluetoothStatusMessageConverter::register_in_host(context);
    ScanResultsMessageConverter::register_in_host(context);
    BluetoothCommandMessageConverter::register_in_host(context);
    DeviceEventMessageConverter::register_in_host(context);
    DeviceProximityEventConverter::register_in_host(context);
}
```

All FFI-relevant types carry `#[stabby::stabby]`. The `stabby` dependency must include the `serde` feature
(`stabby = { workspace = true, features = ["serde"] }`).

---

## 4. Service Crate (`services/bluetooth`)

### 4.1 Overview

The Bluetooth Service is a singleton background service that communicates with **BlueZ** via D-Bus. It subscribes to BlueZ D-Bus signals (`PropertiesChanged`,
`InterfacesAdded`, `InterfacesRemoved`) for event-driven status updates, publishes status on `TOPIC_STATUS`, and processes incoming commands on `TOPIC_COMMAND`.
An initial `do_refresh` call on startup fetches the current state; thereafter, signal handlers react to changes. A low-frequency fallback interval (every 30s)
guards against missed signals (e.g. BlueZ restart).

### 4.2 BlueZ D-Bus Interfaces

| Interface                            | Object Path                 | Methods / Properties Used                                                                          |
|--------------------------------------|-----------------------------|----------------------------------------------------------------------------------------------------|
| `org.bluez.Adapter1`                 | `/org/bluez/hci0`           | `Powered`, `Discoverable`, `Discovering`, `StartDiscovery`, `StopDiscovery`                        |
| `org.bluez.Device1`                  | `/org/bluez/hci0/dev_XX_XX` | `Connect`, `Disconnect`, `Pair`, `RemoveDevice`, `Connected`, `Name`, `Address`, `Icon`, `Battery` |
| `org.freedesktop.DBus.ObjectManager` | `/`                         | `GetManagedObjects`, `InterfacesAdded` / `InterfacesRemoved` signals                               |
| `org.freedesktop.DBus.Properties`    | any BlueZ object            | `PropertiesChanged` signal — event-driven updates for adapter and device properties                |

### 4.3 D-Bus Proxy Traits (`dbus.rs`)

```rust
/// BlueZ Adapter1 interface.
#[zbus::proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez"
)]
trait Adapter1 {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn discoverable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_discoverable(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn discovering(&self) -> zbus::Result<bool>;
    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
}

/// BlueZ Device1 interface.
#[zbus::proxy(
    interface = "org.bluez.Device1",
    default_service = "org.bluez"
)]
trait Device1 {
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;
    fn connect(&self) -> zbus::Result<()>;
    fn disconnect(&self) -> zbus::Result<()>;
    fn pair(&self) -> zbus::Result<()>;
}

/// org.freedesktop.DBus.ObjectManager interface for enumerating BlueZ objects.
#[zbus::proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.bluez",
    default_path = "/"
)]
trait ObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<
        std::collections::HashMap<zbus::zvariant::OwnedObjectPath, std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>
    >;

    /// Stream of `InterfacesAdded` signals — emitted when a new device appears (e.g. during scan).
    fn receive_interfaces_added(&self) -> zbus::Result<zbus::MessageStream>;

    /// Stream of `InterfacesRemoved` signals — emitted when a device is removed.
    fn receive_interfaces_removed(&self) -> zbus::Result<zbus::MessageStream>;
}

/// org.freedesktop.DBus.Properties interface for receiving property change signals.
#[zbus::proxy(
    interface = "org.freedesktop.DBus.Properties",
    default_service = "org.bluez"
)]
trait Properties {
    /// Stream of `PropertiesChanged` signals — emitted when any BlueZ property changes.
    fn receive_properties_changed(&self) -> zbus::Result<zbus::MessageStream>;
}
```

### 4.3a BlueZ Pairing Agent (`agent.rs`)

BlueZ requires a registered pairing agent (`org.bluez.Agent1`) at `org.bluez.AgentManager1` for PIN entry and Secure Simple Pairing (SSP) confirmations. Without
an agent, pairing with security-relevant devices (e.g. keyboards) fails silently.

The service implements a **minimal auto-accept agent** that handles common SSP flows automatically:

```rust
/// D-Bus proxy for `org.bluez.AgentManager1`.
/// Used to register the service's pairing agent with BlueZ.
#[zbus::proxy(
    interface = "org.bluez.AgentManager1",
    default_service = "org.bluez",
    default_path = "/org/bluez"
)]
trait AgentManager {
    /// Register an agent at the given D-Bus object path with the specified capability.
    fn register_agent(&self, agent: &str, capability: &str) -> zbus::Result<()>;
    /// Request that this agent becomes the default agent.
    fn request_default_agent(&self, agent: &str) -> zbus::Result<()>;
    /// Unregister an agent.
    fn unregister_agent(&self, agent: &str) -> zbus::Result<()>;
}

/// Capability string passed to `register_agent`.
/// `DisplayYesNo` enables SSP confirmation with auto-accept.
const AGENT_CAPABILITY: &str = "DisplayYesNo";

/// D-Bus object path where the agent is exported.
const AGENT_PATH: &str = "/smearor/bluetooth/agent";
```

The agent is exported as a D-Bus object implementing `org.bluez.Agent1`. Auto-accept behaviour is controlled by a runtime flag (`auto_accept_pairing`) that can
be toggled at any time via a command message. When auto-accept is disabled (the default), all pairing confirmation requests are **rejected** — the user must
explicitly enable auto-accept via the widget before pairing new devices:

| Agent1 Method          | Behaviour when `auto_accept = true`          | Behaviour when `auto_accept = false` |
|------------------------|----------------------------------------------|--------------------------------------|
| `RequestConfirmation`  | Auto-accept (log device address and passkey) | Reject (log rejected attempt)        |
| `RequestAuthorization` | Auto-accept                                  | Reject                               |
| `DisplayPinCode`       | Log PIN code; auto-accept                    | Log PIN code; reject                 |
| `DisplayPasskey`       | Log passkey; auto-accept                     | Log passkey; reject                  |
| `RequestPinCode`       | Reject (requires UI — future enhancement)    | Reject                               |
| `RequestPasskey`       | Reject (requires UI — future enhancement)    | Reject                               |
| `Cancel`               | Log cancellation; no action                  | Log cancellation; no action          |
| `Release`              | Log release; no action                       | Log release; no action               |

The `BluetoothAgent` struct holds the `auto_accept` flag in an `Arc<AtomicBool>` so it can be toggled from the command channel without blocking the D-Bus
handler:

```rust
pub struct BluetoothAgent {
    auto_accept: Arc<AtomicBool>,
}
```

The agent is registered during `start()` after the D-Bus connection is established:

```rust
async fn register_pairing_agent(connection: &zbus::Connection) -> Result<(), zbus::Error> {
    let agent_manager = AgentManagerProxy::new(connection).await?;
    agent_manager.register_agent(AGENT_PATH, AGENT_CAPABILITY).await?;
    agent_manager.request_default_agent(AGENT_PATH).await?;
    debug!("Bluetooth Service: pairing agent registered as default");
    Ok(())
}
```

The agent object is exported via `connection.object_server().at(AGENT_PATH, BluetoothAgent)`.
`BluetoothAgent` is a struct implementing the `Agent1` trait via `#[zbus::interface]`.

**Security**: Auto-accept is **disabled by default** (`auto_accept_pairing = false` in config). The user must explicitly toggle it via the widget's
`ScanResults` view long-press before pairing new devices. This prevents unwanted pairing by arbitrary Bluetooth devices in range.

**Limitations**: `RequestPinCode` and `RequestPasskey` are rejected because they require interactive UI for manual PIN/passkey entry. This means keyboards and
other PIN-pflichtige devices cannot be paired in this phase. SSP devices (headphones, speakers, mice, gamepads) work via `RequestConfirmation`
when auto-accept is enabled. Full PIN/passkey UI support is planned as a future enhancement (see Section 13).

### 4.4 Core Functions (`dbus.rs`)

| Function                              | Description                                                                     |
|---------------------------------------|---------------------------------------------------------------------------------|
| `get_adapter(connection)`             | Returns the first available `Adapter1Proxy`                                     |
| `get_adapter_state(connection)`       | Returns `(powered, discoverable, discovering, address, name)`                   |
| `get_all_devices(connection)`         | Enumerates all devices via `GetManagedObjects`                                  |
| `get_connected_devices(connection)`   | Returns `Vec<DeviceStatus>` for connected devices only                          |
| `start_discovery(connection)`         | Calls `StartDiscovery` on the adapter                                           |
| `stop_discovery(connection)`          | Calls `StopDiscovery` on the adapter                                            |
| `connect_device(connection, addr)`    | Connects to a device by address                                                 |
| `disconnect_device(connection, addr)` | Disconnects from a device by address                                            |
| `pair_device(connection, addr)`       | Pairs with a device by address (requires registered agent)                      |
| `remove_device(connection, addr)`     | Removes a paired device from the adapter                                        |
| `register_pairing_agent(connection)`  | Registers the auto-accept agent at `org.bluez.AgentManager1`                    |
| `set_powered(connection, powered)`    | Enables or disables the adapter                                                 |
| `set_discoverable(connection, on)`    | Toggles discoverable mode                                                       |
| `is_relevant_property(iface, prop)`   | Filters `PropertiesChanged` signals for relevant properties                     |
| `get_device_rssi(connection, addr)`   | Reads current RSSI (dBm) for a connected device via `Device1Proxy::cached_rssi` |

The `is_relevant_property` function filters `PropertiesChanged` signals to avoid unnecessary refreshes. This filtering applies **only** to signal-driven
refreshes — it does not affect active RSSI polling for proximity detection (see Section 6.6), which reads RSSI directly via `get_device_rssi`:

- **Adapter1**: `Powered`, `Discoverable`, `Discovering` → trigger `do_refresh`
- **Device1**: `Connected`, `Name`, `Icon`, `Paired`, `Battery` → trigger `do_refresh`
- **Other properties** (e.g. `RSSI`, `TxPower`): ignored in `PropertiesChanged` handler — RSSI is polled separately via `get_device_rssi` for proximity
  detection

### 4.5 Service Struct (`service.rs`)

The service uses `tokio::sync::mpsc` for command channels (not `std::sync::mpsc`). The `ServicePlugin` trait provides `on_message` and `start` methods — the
service must implement it.

```rust
/// Internal command enum for the service event loop.
pub enum BluetoothCommand {
    /// Toggle Bluetooth power on/off.
    TogglePower,
    /// Toggle discoverable mode on/off.
    ToggleDiscoverable,
    /// Start a device discovery scan.
    StartScan,
    /// Stop an ongoing device discovery scan.
    StopScan,
    /// Connect to a device by address.
    ConnectDevice(String),
    /// Disconnect from a device by address.
    DisconnectDevice(String),
    /// Pair with a device by address.
    PairDevice(String),
    /// Remove a paired device by address.
    RemoveDevice(String),
    /// Airplane mode toggle (shared with Network service).
    AirplaneMode(bool),
    /// Refresh all status information.
    Refresh,
}

pub struct BluetoothService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: BluetoothServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<BluetoothCommand>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<BluetoothCommand>>,
    pub shared_state: Arc<Mutex<BluetoothSharedState>>,
}

impl BluetoothService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_bluetooth_model::register_json_converters(core_context);

        let bluetooth_config: BluetoothServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<BluetoothCommand>();
        let meta = PluginMeta::try_from(&config)?;
        let shared_state = Arc::new(Mutex::new(BluetoothSharedState::default()));

        let service = BluetoothService {
            meta,
            core_context,
            config: bluetooth_config,
            command_sender,
            command_receiver: Some(command_receiver),
            shared_state,
        };
        Ok(service)
    }
}
```

The service implements the following traits:

- `ServicePlugin` — provides `on_message` (dispatches `FfiEnvelope` to typed `MessageHandler`) and `start` (spawns async runtime)
- `MessageHandler<FfiEnvelopePayload<BluetoothCommandMessage>>` — converts incoming command messages to internal `BluetoothCommand` enum
- `MessageBroadcaster` — empty impl for broadcasting messages
- `MessageTopicBroadcaster<BluetoothStatusMessage>` — for broadcasting status on `TOPIC_STATUS`
- `MessageTopicBroadcaster<ScanResultsMessage>` — for broadcasting scan results on `TOPIC_SCAN_RESULTS`
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`
- `McpCapabilitiesRegistrator` — registers MCP tools (see Section 4.8)

The `start` method spawns a thread with `tokio::runtime::Builder::new_current_thread().enable_all()` + `LocalSet`, analogous to `NetworkService::start`:

### 4.6 Async Loop (`run_bluetooth_async`)

The async loop uses `tokio::select!` with four signal streams and the command channel. D-Bus signal streams are subscribed **before** the initial `do_refresh`
call to avoid a race condition where signals emitted between the initial state fetch and stream subscription would be lost. A low-frequency fallback interval
(every 30s) guards against missed signals (e.g. BlueZ restart).

```rust
async fn run_bluetooth_async(
    meta: PluginMeta,
    core_context: FfiCoreContext,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<BluetoothCommand>,
    config: BluetoothServiceConfig,
    shared_state: Arc<Mutex<BluetoothSharedState>>,
) {
    let connection = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            error!("Bluetooth Service: failed to create D-Bus connection: {e}");
            return;
        }
    };

    // Subscribe to BlueZ D-Bus signals BEFORE the initial state fetch.
    // This prevents a race condition where signals emitted between
    // do_refresh() and stream subscription would be lost.
    // Proxy creation is retried up to 3 times with 1s delay to handle
    // cases where BlueZ is not yet ready at service startup.
    let object_manager = retry_proxy(|| ObjectManagerProxy::new(&connection), "ObjectManager").await;
    let properties = retry_proxy(|| PropertiesProxy::new(&connection), "Properties").await;

    let mut props_stream = properties.and_then(|p| p.receive_properties_changed().ok()).into_iter().flatten();
    let mut interfaces_added_stream = object_manager.and_then(|m| m.receive_interfaces_added().ok()).into_iter().flatten();
    let mut interfaces_removed_stream = object_manager.and_then(|m| m.receive_interfaces_removed().ok()).into_iter().flatten();

    // Initial state fetch on startup (after streams are subscribed)
    do_refresh(&connection, &shared_state, &meta, &core_context).await;

    // Low-frequency fallback interval (guards against missed signals)
    let mut fallback_interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Fallback: refresh every 30s in case signals were missed
            _ = fallback_interval.tick() => {
                do_refresh(&connection, &shared_state, &meta, &core_context).await;
            }
            // Command channel: handle incoming commands from the message broker
            Some(cmd) = command_receiver.recv() => {
                match cmd {
                    BluetoothCommand::TogglePower => { /* set_powered */ }
                    BluetoothCommand::ToggleDiscoverable => { /* set_discoverable */ }
                    BluetoothCommand::StartScan => { /* start_discovery */ }
                    BluetoothCommand::StopScan => { /* stop_discovery */ }
                    BluetoothCommand::ConnectDevice(addr) => { /* connect_device */ }
                    BluetoothCommand::DisconnectDevice(addr) => { /* disconnect_device */ }
                    BluetoothCommand::PairDevice(addr) => { /* pair_device */ }
                    BluetoothCommand::RemoveDevice(addr) => { /* remove_device */ }
                    BluetoothCommand::AirplaneMode(enabled) => { /* set_powered(!enabled) */ }
                    BluetoothCommand::Refresh => {}
                }
                do_refresh(&connection, &shared_state, &meta, &core_context).await;
            }
            // PropertiesChanged: adapter or device property changed
            Some(msg) = props_stream.next() => {
                if is_relevant_properties_changed(&msg) {
                    do_refresh(&connection, &shared_state, &meta, &core_context).await;
                }
            }
            // InterfacesAdded: new device appeared (e.g. during scan)
            Some(msg) = interfaces_added_stream.next() => {
                do_refresh(&connection, &shared_state, &meta, &core_context).await;
            }
            // InterfacesRemoved: device removed
            Some(msg) = interfaces_removed_stream.next() => {
                do_refresh(&connection, &shared_state, &meta, &core_context).await;
            }
        }
    }
}
```

The `retry_proxy` helper function retries D-Bus proxy creation up to 3 times with a 1-second delay between attempts. If all retries fail, it logs an error and
returns `None`, which causes the corresponding signal stream to remain empty. The service continues operating with the fallback interval and command channel,
but the error is visible in logs rather than failing silently:

```rust
async fn retry_proxy<T, F, Fut>(create: F, name: &str) -> Option<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output=zbus::Result<T>>,
{
    for attempt in 1..=3 {
        match create().await {
            Ok(proxy) => return Some(proxy),
            Err(e) => {
                error!("Bluetooth Service: failed to create {name} proxy (attempt {attempt}/3): {e}");
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
    error!("Bluetooth Service: {name} proxy creation failed after 3 attempts — signal stream will be inactive");
    None
}
```

The `is_relevant_properties_changed` function parses the `PropertiesChanged` message and checks whether the changed interface and properties are relevant (see
`is_relevant_property` in Section 4.4). Irrelevant signals (e.g. `RSSI` updates) are filtered out to avoid unnecessary refreshes.

The `do_refresh` function polls adapter state and connected devices via D-Bus, builds a `BluetoothStatusMessage`, and broadcasts it via `send_status` (analogous
to `NetworkService::send_status`).

**Mutex discipline**: D-Bus calls (`get_adapter_state`, `get_connected_devices`) are async and potentially slow. They must be executed **outside** the
`shared_state` mutex lock to avoid blocking other tasks. The mutex is acquired only briefly for writing the updated state and constructing the
`BluetoothStatusMessage`:

```rust
async fn do_refresh(
    connection: &zbus::Connection,
    shared_state: &Arc<Mutex<BluetoothSharedState>>,
    meta: &PluginMeta,
    core_context: &FfiCoreContext,
) {
    // 1. Async D-Bus queries without holding the lock
    let adapter_state = match get_adapter_state(connection).await {
        Ok(state) => state,
        Err(e) => {
            error!("Bluetooth Service: failed to get adapter state: {e}");
            return;
        }
    };
    let connected_devs = match get_connected_devices(connection).await {
        Ok(devs) => devs,
        Err(e) => {
            error!("Bluetooth Service: failed to get connected devices: {e}");
            return;
        }
    };

    // 2. Brief lock to update shared state and build status message
    let status = {
        let mut state = shared_state.lock().await;
        state.update(adapter_state, connected_devs);
        state.build_status_message()
    };

    // 3. Broadcast outside the lock
    send_status(meta, core_context, status);
}
```

The `send_status` function broadcasts the message via `FfiCoreContext`:

```rust
fn send_status(meta: &PluginMeta, core_context: &FfiCoreContext, status: BluetoothStatusMessage) {
    let payload_ptr = Box::into_raw(Box::new(status)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(meta.id.clone()),
        target_instance_id: stabby::string::String::from("*"),
        topic: stabby::string::String::from(BluetoothStatusMessage::topic()),
        type_id: BluetoothStatusMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<BluetoothStatusMessage>),
    };
    core_context.send_message(envelope);
}
```

### 4.7 Service Config

```rust
/// A single automation action triggered by a device event.
/// Each action sends a message to a topic with an optional payload.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BluetoothAutomationAction {
    /// Message topic to send the action to (e.g., "service.audio.command", "service.power.command").
    pub topic: String,
    /// JSON payload to send with the message.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Automation rules for a specific device address.
/// Actions are triggered on connect/disconnect events for this device.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BluetoothAutomationRule {
    /// Device address to match (e.g., "AA:BB:CC:DD:EE:FF").
    pub address: String,
    /// Actions to execute when this device connects.
    #[serde(default)]
    pub on_connect: Vec<BluetoothAutomationAction>,
    /// Actions to execute when this device disconnects.
    #[serde(default)]
    pub on_disconnect: Vec<BluetoothAutomationAction>,
}

/// Configuration for the Bluetooth service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BluetoothServiceConfig {
    /// Whether to enable device scanning support.
    #[serde(default = "default_true")]
    pub enable_scanning: bool,
    /// Maximum number of devices to include in scan results.
    #[serde(default = "default_max_devices")]
    pub max_devices: usize,
    /// Automation rules: actions triggered on device connect/disconnect events.
    /// Each rule matches a device by address and executes the configured action list.
    #[serde(default)]
    pub automation: Vec<BluetoothAutomationRule>,
    /// Whether to register the pairing agent with BlueZ.
    /// When enabled, the service registers as default agent at `org.bluez.AgentManager1`.
    /// When disabled, an external agent (e.g. blueman-agent) must be running for pairing to work.
    #[serde(default = "default_true")]
    pub enable_pairing_agent: bool,
    /// Whether to auto-accept SSP pairing confirmations.
    /// **Default: false** — auto-accept is disabled for security reasons.
    /// Any Bluetooth device in range could pair without user consent if this were enabled by default.
    /// The user can toggle auto-accept at runtime via the widget's `ScanResults` view long-press.
    #[serde(default)]
    pub auto_accept_pairing: bool,
}

fn default_true() -> bool { true }
fn default_max_devices() -> usize { 15 }
```

### 4.8 Bluetooth-Automation (Connect/Disconnect Events)

The service evaluates `automation` rules from `BluetoothServiceConfig` when a `DeviceEventMessage` is generated (i.e., when `PropertiesChanged` signals a
`Connected` property change on a `Device1` interface):

1. Service detects connect/disconnect via `PropertiesChanged` signal on `Device1.Connected`.
2. Service broadcasts `DeviceEventMessage` on `TOPIC_DEVICE_EVENT`.
3. Service checks `config.automation` for rules matching the device address.
4. For each matching rule, executes `on_connect` or `on_disconnect` actions by sending messages to the configured topics.

Example: When headphones connect → start music player; when phone disconnects → lock screen.

```rust
async fn handle_device_event(
    event: &DeviceEventMessage,
    config: &BluetoothServiceConfig,
    core_context: &FfiCoreContext,
    meta: &PluginMeta,
) {
    // Broadcast the event for other services (e.g., Audio Service for routing)
    broadcast_device_event(meta, core_context, event);

    // Evaluate automation rules
    for rule in &config.automation {
        if rule.address == event.address.to_string() {
            let actions = match event.event_type {
                BluetoothDeviceEventType::Connected => &rule.on_connect,
                BluetoothDeviceEventType::Disconnected => &rule.on_disconnect,
            };
            for action in actions {
                send_automation_action(core_context, meta, action).await;
            }
        }
    }
}
```

### 4.9 MCP Tools

The service implements `McpCapabilitiesRegistrator` to register MCP tools for external automation. This is called during `start()` via
`self.register_mcp_capabilities()`, analogous to `NetworkService`.

The service also handles `InvokeToolMessage` and `InvokeResourceMessage` via `MessageHandler` implementations, checking
`envelope.topic == TOPIC_MCP_INVOKE_TOOL` in `on_message`.

| Tool Name              | Parameters        | Description                   |
|------------------------|-------------------|-------------------------------|
| `bluetooth.toggle`     | `enabled: bool`   | Toggle Bluetooth power        |
| `bluetooth.scan`       | —                 | Start a device discovery scan |
| `bluetooth.connect`    | `address: String` | Connect to a device           |
| `bluetooth.disconnect` | `address: String` | Disconnect from a device      |
| `bluetooth.pair`       | `address: String` | Pair with a device            |
| `bluetooth.remove`     | `address: String` | Remove a paired device        |
| `bluetooth.status`     | —                 | Get current Bluetooth status  |

---

## 5. Widget Crate (`plugins/bluetooth`)

### 5.1 Widget Struct

The widget mirrors the Network Widget architecture with a compact tile, view-based rotation, and `gtk4::Image` for Nerd Font icons. It uses `Rc<RefCell<...>>`
for interior mutability and `glib::clone!` for closure ownership.

The widget follows the **unified 4-line layout** used by all GTK-based widgets:

| Line | Height      | Content                          |
|------|-------------|----------------------------------|
| 0    | `icon_size` | Icon (`gtk4::Image`)             |
| 1    | 20px        | `widget-main-text` (value label) |
| 2    | 16px        | `widget-info-text` (info label)  |
| 3    | 16px        | spacer                           |

In Compact mode with `icon_only = true`, lines 1–3 retain their `height_request` to preserve icon alignment across widgets.

```rust
pub struct BluetoothWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: BluetoothWidgetConfig,
    pub icon_image: Rc<RefCell<Option<gtk4::Image>>>,
    pub value_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<BluetoothStatusMessage>>>,
    pub latest_scan: Rc<RefCell<Option<ScanResultsMessage>>>,
    pub latest_personalization: Rc<RefCell<Option<PersonalizationStatusMessage>>>,
}
```

The widget implements the following traits:

- `WidgetPlugin` — provides `on_message` (dispatches `FfiEnvelope` to typed `MessageHandler`) and `start` (spawns listener task)
- `WidgetBuilder` — provides `build_widget` returning a `gtk4::Box` with icon, labels, and gesture handlers
- `MessageHandler<FfiEnvelopePayload<BluetoothStatusMessage>>` — updates `latest_status` and triggers `update_ui`
- `MessageHandler<FfiEnvelopePayload<ScanResultsMessage>>` — updates `latest_scan` and triggers `update_ui`
- `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` — updates `latest_personalization` for locale-aware labels
- `MessageBroadcaster` — for sending command messages to the service
- `MessageTopicBroadcaster<BluetoothCommandMessage>` — for broadcasting commands on `TOPIC_COMMAND`
- `MessageTopicBroadcaster<WidgetUpdateMessage>` — for broadcasting widget updates (headless/Web instance sync)
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`
- `DefaultFallback` — provides fallback click/longpress/drag behavior for `GestureHandler`
- `AcceptTopic<FfiEnvelope>` — topic filtering for incoming messages (subscribes to `TOPIC_STATUS`, `TOPIC_SCAN_RESULTS`, `TOPIC_PERSONALIZATION_STATUS`)

The `new` constructor follows the same pattern as `NetworkWidget::new`:

```rust
impl BluetoothWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_bluetooth_model::register_json_converters(core_context);

        let widget_config: BluetoothWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let meta = PluginMeta::try_from(&config)?;

        let widget = BluetoothWidget {
            meta,
            core_context,
            config: widget_config,
            icon_image: Rc::new(RefCell::new(None)),
            value_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            latest_status: Rc::new(RefCell::new(None)),
            latest_scan: Rc::new(RefCell::new(None)),
            latest_personalization: Rc::new(RefCell::new(None)),
        };
        Ok(widget)
    }
}
```

### 5.2 Widget Config

The widget config uses shared config structs via `#[serde(flatten)]` for dimensions, layout, icon, text colors, and mode, analogous to
`plugins/network/src/config.rs`. Gesture bindings use `ActionBindings` instead of individual `click_topic`/`longpress_topic` fields.

```rust
// Default Nerd Font icon names
pub const DEFAULT_ICON_BLUETOOTH_ON: &str = "nf-md-bluetooth";
pub const DEFAULT_ICON_BLUETOOTH_OFF: &str = "nf-md-bluetooth_off";
pub const DEFAULT_ICON_BLUETOOTH_AUDIO: &str = "nf-md-bluetooth_audio";
pub const DEFAULT_ICON_BLUETOOTH_TRANSFER: &str = "nf-md-bluetooth_transfer";
pub const DEFAULT_ICON_BLUETOOTH_BATTERY: &str = "nf-md-battery_bluetooth";
pub const DEFAULT_ICON_BATTERY_10: &str = "nf-md-battery_10_bluetooth";
pub const DEFAULT_ICON_BATTERY_20: &str = "nf-md-battery_20_bluetooth";
pub const DEFAULT_ICON_BATTERY_30: &str = "nf-md-battery_30_bluetooth";
pub const DEFAULT_ICON_BATTERY_40: &str = "nf-md-battery_40_bluetooth";
pub const DEFAULT_ICON_BATTERY_50: &str = "nf-md-battery_50_bluetooth";
pub const DEFAULT_ICON_BATTERY_60: &str = "nf-md-battery_60_bluetooth";
pub const DEFAULT_ICON_BATTERY_70: &str = "nf-md-battery_70_bluetooth";
pub const DEFAULT_ICON_BATTERY_80: &str = "nf-md-battery_80_bluetooth";
pub const DEFAULT_ICON_BATTERY_90: &str = "nf-md-battery_90_bluetooth";
pub const DEFAULT_ICON_BATTERY_ALERT: &str = "nf-md-battery_alert_bluetooth";
pub const DEFAULT_ICON_BLUETOOTH_SETTINGS: &str = "nf-md-bluetooth_settings";
pub const DEFAULT_ICON_SPEAKER: &str = "nf-md-speaker_bluetooth";
pub const DEFAULT_ICON_AIRPLANE_ON: &str = "nf-md-airplane";
pub const DEFAULT_ICON_AIRPLANE_OFF: &str = "nf-md-airplane_off";

// Default Nerd Font icon names for device types
pub const DEFAULT_ICON_DEVICE_HEADPHONES: &str = "nf-md-headphones";
pub const DEFAULT_ICON_DEVICE_SPEAKER: &str = "nf-md-speaker";
pub const DEFAULT_ICON_DEVICE_KEYBOARD: &str = "nf-md-keyboard";
pub const DEFAULT_ICON_DEVICE_MOUSE: &str = "nf-mouse";
pub const DEFAULT_ICON_DEVICE_GAMING: &str = "nf-md-gamepad_variant";
pub const DEFAULT_ICON_DEVICE_PHONE: &str = "nf-md-cellphone";
pub const DEFAULT_ICON_DEVICE_COMPUTER: &str = "nf-md-laptop";
pub const DEFAULT_ICON_DEVICE_CAMERA: &str = "nf-md-camera";
pub const DEFAULT_ICON_DEVICE_PRINTER: &str = "nf-md-printer";
pub const DEFAULT_ICON_DEVICE_WEARABLE: &str = "nf-md-watch";
pub const DEFAULT_ICON_DEVICE_NETWORK: &str = "nf-md-router_network";
pub const DEFAULT_ICON_DEVICE_UNKNOWN: &str = "nf-md-bluetooth";

/// Bluetooth-specific icon configuration.
/// All Nerd Font icon names used by the Bluetooth widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct BluetoothIcons {
    /// Bluetooth icon: powered on.
    #[builder(default = DEFAULT_ICON_BLUETOOTH_ON.to_string())]
    #[serde(default = "default_icon_bluetooth_on")]
    pub(crate) icon_bluetooth_on: String,

    /// Bluetooth icon: powered off.
    #[builder(default = DEFAULT_ICON_BLUETOOTH_OFF.to_string())]
    #[serde(default = "default_icon_bluetooth_off")]
    pub(crate) icon_bluetooth_off: String,

    /// Bluetooth icon: audio device connected.
    #[builder(default = DEFAULT_ICON_BLUETOOTH_AUDIO.to_string())]
    #[serde(default = "default_icon_bluetooth_audio")]
    pub(crate) icon_bluetooth_audio: String,

    /// Bluetooth icon: data transfer active.
    #[builder(default = DEFAULT_ICON_BLUETOOTH_TRANSFER.to_string())]
    #[serde(default = "default_icon_bluetooth_transfer")]
    pub(crate) icon_bluetooth_transfer: String,

    /// Bluetooth icon: generic battery.
    #[builder(default = DEFAULT_ICON_BLUETOOTH_BATTERY.to_string())]
    #[serde(default = "default_icon_bluetooth_battery")]
    pub(crate) icon_bluetooth_battery: String,

    /// Bluetooth icon: battery 10%.
    #[builder(default = DEFAULT_ICON_BATTERY_10.to_string())]
    #[serde(default = "default_icon_battery_10")]
    pub(crate) icon_battery_10: String,

    /// Bluetooth icon: battery 20%.
    #[builder(default = DEFAULT_ICON_BATTERY_20.to_string())]
    #[serde(default = "default_icon_battery_20")]
    pub(crate) icon_battery_20: String,

    /// Bluetooth icon: battery 30%.
    #[builder(default = DEFAULT_ICON_BATTERY_30.to_string())]
    #[serde(default = "default_icon_battery_30")]
    pub(crate) icon_battery_30: String,

    /// Bluetooth icon: battery 40%.
    #[builder(default = DEFAULT_ICON_BATTERY_40.to_string())]
    #[serde(default = "default_icon_battery_40")]
    pub(crate) icon_battery_40: String,

    /// Bluetooth icon: battery 50%.
    #[builder(default = DEFAULT_ICON_BATTERY_50.to_string())]
    #[serde(default = "default_icon_battery_50")]
    pub(crate) icon_battery_50: String,

    /// Bluetooth icon: battery 60%.
    #[builder(default = DEFAULT_ICON_BATTERY_60.to_string())]
    #[serde(default = "default_icon_battery_60")]
    pub(crate) icon_battery_60: String,

    /// Bluetooth icon: battery 70%.
    #[builder(default = DEFAULT_ICON_BATTERY_70.to_string())]
    #[serde(default = "default_icon_battery_70")]
    pub(crate) icon_battery_70: String,

    /// Bluetooth icon: battery 80%.
    #[builder(default = DEFAULT_ICON_BATTERY_80.to_string())]
    #[serde(default = "default_icon_battery_80")]
    pub(crate) icon_battery_80: String,

    /// Bluetooth icon: battery 90%.
    #[builder(default = DEFAULT_ICON_BATTERY_90.to_string())]
    #[serde(default = "default_icon_battery_90")]
    pub(crate) icon_battery_90: String,

    /// Bluetooth icon: battery alert (low battery).
    #[builder(default = DEFAULT_ICON_BATTERY_ALERT.to_string())]
    #[serde(default = "default_icon_battery_alert")]
    pub(crate) icon_battery_alert: String,

    /// Bluetooth icon: settings.
    #[builder(default = DEFAULT_ICON_BLUETOOTH_SETTINGS.to_string())]
    #[serde(default = "default_icon_bluetooth_settings")]
    pub(crate) icon_bluetooth_settings: String,

    /// Bluetooth icon: speaker connected.
    #[builder(default = DEFAULT_ICON_SPEAKER.to_string())]
    #[serde(default = "default_icon_speaker")]
    pub(crate) icon_speaker: String,

    /// Airplane icon: airplane mode on.
    #[builder(default = DEFAULT_ICON_AIRPLANE_ON.to_string())]
    #[serde(default = "default_icon_airplane_on")]
    pub(crate) icon_airplane_on: String,

    /// Airplane icon: airplane mode off.
    #[builder(default = DEFAULT_ICON_AIRPLANE_OFF.to_string())]
    #[serde(default = "default_icon_airplane_off")]
    pub(crate) icon_airplane_off: String,

    /// Device type icon: headphones / headset.
    #[builder(default = DEFAULT_ICON_DEVICE_HEADPHONES.to_string())]
    #[serde(default = "default_icon_device_headphones")]
    pub(crate) icon_device_headphones: String,

    /// Device type icon: speaker.
    #[builder(default = DEFAULT_ICON_DEVICE_SPEAKER.to_string())]
    #[serde(default = "default_icon_device_speaker")]
    pub(crate) icon_device_speaker: String,

    /// Device type icon: keyboard.
    #[builder(default = DEFAULT_ICON_DEVICE_KEYBOARD.to_string())]
    #[serde(default = "default_icon_device_keyboard")]
    pub(crate) icon_device_keyboard: String,

    /// Device type icon: mouse / pointing device.
    #[builder(default = DEFAULT_ICON_DEVICE_MOUSE.to_string())]
    #[serde(default = "default_icon_device_mouse")]
    pub(crate) icon_device_mouse: String,

    /// Device type icon: gaming controller.
    #[builder(default = DEFAULT_ICON_DEVICE_GAMING.to_string())]
    #[serde(default = "default_icon_device_gaming")]
    pub(crate) icon_device_gaming: String,

    /// Device type icon: phone.
    #[builder(default = DEFAULT_ICON_DEVICE_PHONE.to_string())]
    #[serde(default = "default_icon_device_phone")]
    pub(crate) icon_device_phone: String,

    /// Device type icon: computer / laptop.
    #[builder(default = DEFAULT_ICON_DEVICE_COMPUTER.to_string())]
    #[serde(default = "default_icon_device_computer")]
    pub(crate) icon_device_computer: String,

    /// Device type icon: camera.
    #[builder(default = DEFAULT_ICON_DEVICE_CAMERA.to_string())]
    #[serde(default = "default_icon_device_camera")]
    pub(crate) icon_device_camera: String,

    /// Device type icon: printer / scanner.
    #[builder(default = DEFAULT_ICON_DEVICE_PRINTER.to_string())]
    #[serde(default = "default_icon_device_printer")]
    pub(crate) icon_device_printer: String,

    /// Device type icon: wearable / watch.
    #[builder(default = DEFAULT_ICON_DEVICE_WEARABLE.to_string())]
    #[serde(default = "default_icon_device_wearable")]
    pub(crate) icon_device_wearable: String,

    /// Device type icon: network access point / router.
    #[builder(default = DEFAULT_ICON_DEVICE_NETWORK.to_string())]
    #[serde(default = "default_icon_device_network")]
    pub(crate) icon_device_network: String,

    /// Device type icon: unknown / unmapped device.
    #[builder(default = DEFAULT_ICON_DEVICE_UNKNOWN.to_string())]
    #[serde(default = "default_icon_device_unknown")]
    pub(crate) icon_device_unknown: String,
}

impl Default for BluetoothIcons {
    fn default() -> Self {
        BluetoothIcons {
            icon_bluetooth_on: DEFAULT_ICON_BLUETOOTH_ON.to_string(),
            icon_bluetooth_off: DEFAULT_ICON_BLUETOOTH_OFF.to_string(),
            icon_bluetooth_audio: DEFAULT_ICON_BLUETOOTH_AUDIO.to_string(),
            icon_bluetooth_transfer: DEFAULT_ICON_BLUETOOTH_TRANSFER.to_string(),
            icon_bluetooth_battery: DEFAULT_ICON_BLUETOOTH_BATTERY.to_string(),
            icon_battery_10: DEFAULT_ICON_BATTERY_10.to_string(),
            icon_battery_20: DEFAULT_ICON_BATTERY_20.to_string(),
            icon_battery_30: DEFAULT_ICON_BATTERY_30.to_string(),
            icon_battery_40: DEFAULT_ICON_BATTERY_40.to_string(),
            icon_battery_50: DEFAULT_ICON_BATTERY_50.to_string(),
            icon_battery_60: DEFAULT_ICON_BATTERY_60.to_string(),
            icon_battery_70: DEFAULT_ICON_BATTERY_70.to_string(),
            icon_battery_80: DEFAULT_ICON_BATTERY_80.to_string(),
            icon_battery_90: DEFAULT_ICON_BATTERY_90.to_string(),
            icon_battery_alert: DEFAULT_ICON_BATTERY_ALERT.to_string(),
            icon_bluetooth_settings: DEFAULT_ICON_BLUETOOTH_SETTINGS.to_string(),
            icon_speaker: DEFAULT_ICON_SPEAKER.to_string(),
            icon_airplane_on: DEFAULT_ICON_AIRPLANE_ON.to_string(),
            icon_airplane_off: DEFAULT_ICON_AIRPLANE_OFF.to_string(),
            icon_device_headphones: DEFAULT_ICON_DEVICE_HEADPHONES.to_string(),
            icon_device_speaker: DEFAULT_ICON_DEVICE_SPEAKER.to_string(),
            icon_device_keyboard: DEFAULT_ICON_DEVICE_KEYBOARD.to_string(),
            icon_device_mouse: DEFAULT_ICON_DEVICE_MOUSE.to_string(),
            icon_device_gaming: DEFAULT_ICON_DEVICE_GAMING.to_string(),
            icon_device_phone: DEFAULT_ICON_DEVICE_PHONE.to_string(),
            icon_device_computer: DEFAULT_ICON_DEVICE_COMPUTER.to_string(),
            icon_device_camera: DEFAULT_ICON_DEVICE_CAMERA.to_string(),
            icon_device_printer: DEFAULT_ICON_DEVICE_PRINTER.to_string(),
            icon_device_wearable: DEFAULT_ICON_DEVICE_WEARABLE.to_string(),
            icon_device_network: DEFAULT_ICON_DEVICE_NETWORK.to_string(),
            icon_device_unknown: DEFAULT_ICON_DEVICE_UNKNOWN.to_string(),
        }
    }
}

/// Configuration for the Bluetooth widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct BluetoothWidgetConfig {
    /// Shared widget dimensions (width, height, max_width).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,

    /// Shared widget layout (spacing, css_classes).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,

    /// Shared widget icon settings (icon_size, icon_only, icon_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon: WidgetIcon,

    /// Shared widget text colors (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,

    /// Shared widget mode (compact, wide).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) mode: WidgetMode,

    /// Action bindings for click, longpress, drag, and scroll gestures.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) actions: ActionBindings,

    /// Bluetooth-specific Nerd Font icons.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icons: BluetoothIcons,

    /// Views to cycle through on swipe up/down.
    #[builder(default)]
    pub(crate) views: Vec<BluetoothView>,

    /// Maximum number of devices to show in scan results.
    #[builder(default = 10)]
    pub(crate) max_devices: usize,
}
```

The `ActionBindings` struct (from `plugin-api`) provides `click`, `longpress`, `drag_up`, `drag_down`, `drag_left`, `drag_right`, and `scroll` action fields.
Each action is an `Option<Action>` with `topic`, `payload`, and `instance` fields. This replaces the old `click_topic`/`click_payload`/`click_instance`/
`longpress_topic`/`longpress_payload`/`longpress_instance` pattern.

Each binding supports a `mode` field of type `BindingMode` (`replace` or `supplement`), configurable via TOML (e.g. `click_mode = "supplement"`). In `replace`
mode (default), a configured binding replaces the widget's default fallback behavior. In `supplement` mode, both the binding **and** the default fallback are
dispatched. This allows e.g. configuring a `click` binding that sends a message while still toggling Bluetooth power. This is analogous to the `network` and
`weather` widgets.

### 5.3 View Rendering

`render_view` returns a `ViewData` struct (with `icon_name`, `main_text`, `info_text`, and optional `icon_color`), analogous to `NetworkWidget::render_view`.
Icon names are Nerd Font names (e.g. `nf-md-bluetooth`) resolved via
`resolve_gtk_nerd_icon()` to GTK symbolic icon GResource paths (e.g. `nf-md-bluetooth-symbolic.svg`) in the GTK rendering pipeline, and via
`resolve_icon_codepoint()` to Unicode codepoints in the pixel/atomic rendering pipeline.

```rust
fn render_view(
    status: &BluetoothStatusMessage,
    scan: Option<&ScanResultsMessage>,
    config: &BluetoothWidgetConfig,
    view: BluetoothView,
    personalization: Option<&PersonalizationStatusMessage>,
) -> ViewData {
    let labels = BluetoothLabel::from_personalization(personalization);
    match view {
        BluetoothView::PowerStatus => {
            if status.powered {
                let connected_count = status.connected_devices.len();
                let info = if connected_count > 0 {
                    format!("{connected_count} {}", labels.devices)
                } else {
                    status.adapter_name.to_string()
                };
                ViewData::new(&config.icons.icon_bluetooth_on, &labels.on, &info)
            } else {
                ViewData::new(&config.icons.icon_bluetooth_off, &labels.off, &labels.bluetooth)
            }
        }
        BluetoothView::ConnectedDevices => {
            match status.connected_devices.first() {
                Some(device) => {
                    let icon = if device.transferring {
                        &config.icons.icon_bluetooth_transfer
                    } else {
                        config.icons.icon_for_device_type(&device.device_type.to_string())
                    };
                    let info = if device.connected { &labels.connected } else { &labels.disconnected };
                    ViewData::new(icon, &device.name.to_string(), info)
                }
                None => ViewData::new(&config.icons.icon_bluetooth_off, "--", &labels.no_devices),
            }
        }
        BluetoothView::ScanResults => {
            match scan {
                Some(scan) => {
                    let count = scan.devices.len();
                    ViewData::new(&config.icons.icon_bluetooth_settings, &format!("{count} {}", labels.found), &labels.scan_results)
                }
                None => ViewData::new(&config.icons.icon_bluetooth_settings, "--", &labels.no_scan),
            }
        }
        BluetoothView::Airplane => {
            if !status.powered {
                ViewData::new(&config.icons.icon_airplane_on, "ON", &labels.airplane)
            } else {
                ViewData::new(&config.icons.icon_airplane_off, "OFF", &labels.airplane)
            }
        }
        BluetoothView::Battery => {
            match status.connected_devices.iter().find(|d| d.battery_level.is_some()) {
                Some(device) => {
                    let level = match device.battery_level {
                        stabby::option::StabbyOption::Some(level) => level,
                        stabby::option::StabbyOption::None => 0,
                    };
                    let icon = if level <= 10 {
                        &config.icons.icon_battery_alert
                    } else if level <= 20 {
                        &config.icons.icon_battery_10
                    } else if level <= 30 {
                        &config.icons.icon_battery_20
                    } else if level <= 40 {
                        &config.icons.icon_battery_30
                    } else if level <= 50 {
                        &config.icons.icon_battery_40
                    } else if level <= 60 {
                        &config.icons.icon_battery_50
                    } else if level <= 70 {
                        &config.icons.icon_battery_60
                    } else if level <= 80 {
                        &config.icons.icon_battery_70
                    } else if level <= 90 {
                        &config.icons.icon_battery_80
                    } else {
                        &config.icons.icon_battery_90
                    };
                    let view_data = ViewData::new(icon, &format!("{level}%"), &device.name.to_string());
                    if level <= 10 {
                        view_data.with_color(Color::RED)
                    } else {
                        view_data
                    }
                }
                None => ViewData::new(&config.icons.icon_bluetooth_battery, "--", &labels.no_battery),
            }
        }
    }
}
```

The `BluetoothLabel` struct provides locale-aware labels, analogous to `NetworkLabel` in the Network widget:

```rust
struct BluetoothLabel {
    on: String,
    off: String,
    bluetooth: String,
    connected: String,
    disconnected: String,
    devices: String,
    no_devices: String,
    found: String,
    scan_results: String,
    no_scan: String,
    airplane: String,
    no_battery: String,
}

impl BluetoothLabel {
    fn from_personalization(p: Option<&PersonalizationStatusMessage>) -> Self {
        // Use locale from personalization status, fallback to English
        // Analogous to NetworkLabel::from_personalization
    }
}
```

The `BluetoothIcons` struct provides a helper method to resolve the appropriate Nerd Font icon for a given BlueZ device type string, using the
`BluetoothDeviceType` mapping from the model crate:

```rust
impl BluetoothIcons {
    /// Resolves the Nerd Font icon name for a BlueZ device type string.
    /// Uses `BluetoothDeviceType::from_bluez_icon` to categorize the device,
    /// then selects the corresponding configured icon.
    pub(crate) fn icon_for_device_type(&self, bluez_icon: &str) -> &str {
        match BluetoothDeviceType::from_bluez_icon(bluez_icon) {
            BluetoothDeviceType::AudioHeadphones => &self.icon_device_headphones,
            BluetoothDeviceType::AudioSpeaker => &self.icon_device_speaker,
            BluetoothDeviceType::InputKeyboard => &self.icon_device_keyboard,
            BluetoothDeviceType::InputMouse => &self.icon_device_mouse,
            BluetoothDeviceType::InputGaming => &self.icon_device_gaming,
            BluetoothDeviceType::Phone => &self.icon_device_phone,
            BluetoothDeviceType::Computer => &self.icon_device_computer,
            BluetoothDeviceType::Camera => &self.icon_device_camera,
            BluetoothDeviceType::Printer => &self.icon_device_printer,
            BluetoothDeviceType::Wearable => &self.icon_device_wearable,
            BluetoothDeviceType::NetworkAccessPoint => &self.icon_device_network,
            BluetoothDeviceType::Unknown => &self.icon_device_unknown,
        }
    }
}
```

### 5.3a Multi-Instance Rendering (Headless / Web)

The widget supports all three instance types (GTK, Headless, Web) by implementing the rendering traits analogous to `NetworkWidget`:

- **GTK** (`InstanceType::Gtk`): `WidgetBuilder::build_widget()` produces a `gtk4::Box` with icon, labels, and gesture handlers. Nerd Font icon names are
  resolved via `resolve_gtk_nerd_icon()` to GResource SVGs.
- **Headless** (`InstanceType::Headless`): `GraphicRenderer::render_graphic(w, h)` produces a raw RGBA pixel buffer via `image` + `ab_glyph`. Nerd Font icon
  names are resolved via `resolve_icon_codepoint()` to Unicode codepoints. The widget reuses `render_view` to get `ViewData`, then renders it to pixels.
- **Web** (`InstanceType::Web`): `WebRenderer::render_html(instance_id, plugin_id)` produces an HTML fragment. The widget renders the `ViewData` fields as HTML
  elements with inline styles.

All three pipelines use the same `render_view` function, ensuring consistent output across instance types. The `broadcast_widget_update` call after every UI
update triggers re-rendering in Headless (via `SetButtonImage`)
and Web (via WebSocket push) instances.

### 5.4 Click Handling (View-Dependent)

The widget implements `DefaultFallback` to provide view-dependent click behavior. When the `ActionBindings` click action is not configured, the fallback handler
is invoked:

| View               | Click Action                                                                      |
|--------------------|-----------------------------------------------------------------------------------|
| `PowerStatus`      | Toggle Bluetooth power via `TogglePower` command                                  |
| `ConnectedDevices` | Disconnect first connected device via `DisconnectDevice`                          |
| `ScanResults`      | Start a scan via `StartScan` command                                              |
| `Airplane`         | Toggle airplane mode via `AirplaneMode` command                                   |
| `Battery`          | Broadcast `click` action from `ActionBindings` (typically opens `bluetooth_area`) |

**Long-press fallback** (when no `longpress` action is configured in `ActionBindings`):

| View          | Long-press Action                                         |
|---------------|-----------------------------------------------------------|
| `ScanResults` | Toggle auto-accept pairing via `ToggleAutoAccept` command |
| Other views   | Open `bluetooth_area` (default long-press fallback)       |

### 5.5 Gesture Handling

The widget uses the shared `GestureHandler` trait with `attach_gesture_handlers`, which automatically handles
`GestureDrag` (swipe up/down for view rotation), `GestureClick`, `GestureLongPress`, and `GestureScroll`. No manual `GestureDrag` setup is required.

```rust
// In build_widget:
widget_self.attach_gesture_handlers( & button_widget, & config.actions, & broadcaster, & GestureHandlersConfiguration::default ());
```

The `DefaultFallback` implementation provides view-dependent click behavior when no explicit click action is configured in `ActionBindings`. Swipe up/down
cycles through `config.views` via `next_view()`/`prev_view()`.

After every UI update, the widget broadcasts a `WidgetUpdateMessage` so headless/Web instances can re-render:

```rust
fn broadcast_widget_update(&self) {
    if let Some(ctx) = self.core_context.as_ref() {
        let update = WidgetUpdateMessage::new(&self.meta);
        self.broadcast_message_to_topic_with_context(ctx, update);
    }
}
```

---

## 6. Airplane Mode Coordination

### 6.1 Problem

Airplane Mode should disable **all wireless communication**: WiFi, WWAN, and Bluetooth. Currently, the Network Widget sends `ToggleRadio("all", enabled)` to the
Network Service only. Bluetooth is not affected.

### 6.2 Solution

The **Network Widget** Airplane Mode click sends **two** commands:

1. `NetworkCommandMessage::toggle_radio("all", is_on)` → `service.network.command`
2. `BluetoothCommandMessage { action: AirplaneMode, enabled: is_on }` → `service.bluetooth.command`

Both services listen on their respective command topics and react independently:

- **Network Service**: `set_wireless_enabled(false)` + `set_wwan_enabled(false)` when airplane mode is ON
- **Bluetooth Service**: `set_powered(false)` on all adapters when airplane mode is ON

### 6.3 Airplane Mode State

Each service derives its own `airplane_mode` flag from its state:

- Network: `airplane_mode = !wifi_enabled && !wwan_enabled`
- Bluetooth: `airplane_mode = !powered`

The widget shows airplane mode as ON only when **both** services report airplane mode ON. This requires the widget to subscribe to both `service.network.status`
and `service.bluetooth.status` topics and compute a combined state.

### 6.4 Implementation in Network Widget

The Network Widget's `Airplane` view click handler is updated to also broadcast a Bluetooth airplane mode command:

```rust
NetworkView::Airplane => {
let is_on = self.latest_status.borrow().as_ref().map( | s | s.airplane_mode).unwrap_or(false);
// Network: toggle WiFi + WWAN
let net_command = NetworkCommandMessage::toggle_radio("all", is_on);
broadcaster.broadcast_message_to_topic(net_command);
// Bluetooth: set airplane mode. enabled = true means airplane mode ON (Bluetooth OFF).
// is_on reflects the current airplane mode state; toggling means we send the new target state.
let bt_command = BluetoothCommandMessage {
action: BluetoothCommandAction::AirplaneMode,
address: StabbyOption::None(),
enabled: ! is_on, // toggle: if currently ON, turn OFF; if currently OFF, turn ON
};
broadcaster.broadcast_message_to_topic(bt_command);
}
```

### 6.5 Bluetooth-Audio-Routing-Integration

When a Bluetooth audio device connects, the Audio Service can automatically switch the output sink. This coordination uses the message system, analogous to
Airplane Mode coordination:

1. **Bluetooth Service** detects device connect via `PropertiesChanged` signal.
2. **Bluetooth Service** broadcasts `DeviceEventMessage { event_type: Connected, device_type: <raw BlueZ icon> }` on `TOPIC_DEVICE_EVENT`.
3. **Audio Service** subscribes to `TOPIC_DEVICE_EVENT` and uses `BluetoothDeviceType::from_bluez_icon(&device_type)`
   to categorize the device. If the result is `AudioHeadphones`, `AudioSpeaker`, or any other audio-related variant, the Audio Service switches the default
   output sink.
4. **Audio Service** switches the default output sink to the newly connected Bluetooth device.

Using `BluetoothDeviceType::from_bluez_icon()` instead of raw string matching protects against deviations in BlueZ icon names (e.g. `"audio-headset"` vs
`"audio-headphones"`) — the `from_bluez_icon`
mapping normalizes these variants into a single `AudioHeadphones` category.

The Audio Service implements `MessageHandler<FfiEnvelopePayload<DeviceEventMessage>>` and
`AcceptTopic<FfiEnvelope>` filtering for `TOPIC_DEVICE_EVENT`.

This is a **one-directional coordination**: Bluetooth Service broadcasts events, Audio Service reacts. No changes to the Audio Service's command interface are
needed — it only adds a new message handler.

### 6.6 Proximity Actions

The Bluetooth Service monitors device proximity via **periodic RSSI polling** on connected devices. When a device's RSSI drops below a configurable threshold,
the service broadcasts a
`DeviceProximityEvent { state: OutOfRange }` on `TOPIC_PROXIMITY`.

Other services and widgets can subscribe to `TOPIC_PROXIMITY` to trigger actions:

- **Power Service**: Lock screen when phone goes out of range.
- **Audio Service / MPRIS Service**: Pause music when headphones go out of range.
- **Widget**: Show a "device lost" notification.

**Important**: Proximity detection must be based on RSSI threshold, **not** on disconnect events. BlueZ does not distinguish on the D-Bus level whether a device
went out of range, was powered off by the user, or ran out of battery — in all cases `Connected = false` is reported. Using disconnect events as a proximity
proxy would cause false triggers (e.g. manually turning off a headset would unintentionally lock the screen). Therefore, proximity actions are strictly bound to
RSSI polling:

1. Service polls RSSI of connected devices at a configurable interval (e.g. every 10s). RSSI is read directly via `get_device_rssi(connection, addr)` which
   accesses the `Device1Proxy`
   `RSSI` property — this is **independent** of the `PropertiesChanged` signal filter (which ignores `RSSI` to avoid refresh-thrashing). The polling interval
   runs as a separate
   `tokio::select!` branch in `run_bluetooth_async`. **Idle guard**: The polling branch is skipped when the adapter is powered off (`powered = false`)
   or no devices are connected (`connected_devices.is_empty()`). This avoids unnecessary D-Bus queries at the polling interval when proximity detection is not
   applicable.
2. If RSSI falls below `rssi_threshold` (e.g. -90 dBm), broadcast `DeviceProximityEvent { state: OutOfRange }`.
3. If RSSI recovers above threshold, broadcast `DeviceProximityEvent { state: InRange }`.
4. A `Disconnected` event without prior RSSI drop is **not** treated as a proximity event.

Config example (in `BluetoothServiceConfig` — future extension):

```toml
[services.bluetooth.proximity]
enabled = true
rssi_threshold = -90
check_interval_seconds = 10
```

---

## 7. Config Integration

### 7.1 Service Config (`config.toml`)

```toml
[services.bluetooth]
enable_scanning = true
max_devices = 15

# Automation rules: actions triggered on device connect/disconnect
[[services.bluetooth.automation]]
address = "AA:BB:CC:DD:EE:FF"
on_connect = [
    { topic = "service.mpris.command", payload = { action = "play" } },
]
on_disconnect = [
    { topic = "service.power.command", payload = { action = "lock" } },
]
```

### 7.2 Widget Config (`config.toml`)

```toml
[[plugins]]
id = "bluetooth_widget"
type = "bluetooth"

[plugins.config]
# Shared config structs (via serde flatten)
width = 100
height = 100
spacing = 0
icon_size = 36
max_devices = 10
views = ["PowerStatus", "ConnectedDevices", "ScanResults", "Airplane", "Battery"]

# Bluetooth-specific icons (via serde flatten into BluetoothIcons)
[plugins.config.icons]
icon_bluetooth_on = "nf-md-bluetooth"
icon_bluetooth_off = "nf-md-bluetooth_off"
icon_bluetooth_audio = "nf-md-bluetooth_audio"
icon_bluetooth_transfer = "nf-md-bluetooth_transfer"
icon_bluetooth_battery = "nf-md-battery_bluetooth"
icon_battery_alert = "nf-md-battery_alert_bluetooth"
icon_bluetooth_settings = "nf-md-bluetooth_settings"
icon_speaker = "nf-md-speaker_bluetooth"
icon_airplane_on = "nf-md-airplane"
icon_airplane_off = "nf-md-airplane_off"

# Action bindings (replaces click_topic/longpress_topic)
[plugins.config.actions]
longpress = { topic = "area.open", payload = { area_id = "bluetooth_area" } }
```

### 7.3 Bluetooth Area (`config.toml`)

The `bluetooth_area` is a scroll menu area that provides quick connect/disconnect for all paired devices. Each device is rendered as a button tile (analogous to
`app-launcher` tiles) with a status icon, device name, and connect/disconnect action.

```toml
[[areas]]
id = "bluetooth_area"

# Close button (standard area header)
[[areas.plugins]]
id = "close_bluetooth_area"
type = "button"
[areas.plugins.config]
icon = "nf-md-close"
actions = { click = { topic = "area.close", payload = { area_id = "bluetooth_area" } } }

# Quick-connect tiles for paired devices are generated dynamically by the Bluetooth Widget
# based on the latest status. Each tile shows:
# - Device icon (resolved via icon_for_device_type)
# - Device name as main_text
# - Connection state as info_text ("Connected" / "Disconnected")
# - Click action: connect if disconnected, disconnect if connected
# - Long-press action: open device settings (blueman-manager or bluetoothctl)

# External Bluetooth management apps
[[areas.plugins]]
id = "blueman_manager"
type = "app-launcher"
[areas.plugins.config]
icon = "nf-md-bluetooth_settings"
app_id = "blueman-manager"
```

The Bluetooth Widget generates device tiles dynamically in `start_listeners` when a `BluetoothStatusMessage`
arrives. Each tile sends a `BluetoothCommandMessage` (`ConnectDevice` or `DisconnectDevice`) on click. Device tiles use the same `BluetoothIcons` configuration
as the main widget for consistent icon rendering.

---

## 8. Implementation Phases

### Phase 1: Model Crate (`model/bluetooth`)

**Order:** First — no dependencies.

**Tasks:**

- Create `model/bluetooth/Cargo.toml` with `stabby` (with `serde` feature), `serde`, `serde_json` dependencies
- Implement `messages/mod.rs` with topic constants
- Implement `messages/bluetooth_status.rs` with `BluetoothStatusMessage`, `DeviceStatus`
- Implement `messages/scan_results.rs` with `ScanResultsMessage`
- Implement `messages/command.rs` with `BluetoothCommandMessage`, `BluetoothCommandAction`
- Implement `messages/view.rs` with `BluetoothView` enum
- Implement `messages/device_type.rs` with `BluetoothDeviceType` enum and `from_bluez_icon` mapping
- Implement `messages/device_event.rs` with `DeviceEventMessage`, `BluetoothDeviceEventType`, `BluetoothDisconnectReason`
- Implement `messages/proximity.rs` with `DeviceProximityEvent`, `ProximityState`
- Implement `lib.rs` with `pub use` re-exports
- Add `#[stabby::stabby]` to all FFI-relevant types
- Use `impl_json_convertible!` macro invocations with `serde_json::from_value(json).unwrap_or_default()` for FFI registration
- Implement `register_json_converters(context)` function calling `Converter::register_in_host(context)`
- No manual `parse_*` functions in `json_converters.rs` — use `impl_json_convertible!` only

**Exit Criteria:** `cargo build -p smearor_bluetooth_model` succeeds.

### Phase 2: Service Crate (`services/bluetooth`)

**Order:** Second — depends on Phase 1.

**Tasks:**

- Create `services/bluetooth/Cargo.toml` with `zbus`, `tokio`, `tracing`, `plugin-api`, `model/bluetooth` dependencies
- Implement `dbus.rs` with `Adapter1Proxy`, `Device1Proxy`, `ObjectManagerProxy`, `PropertiesProxy` traits
- Implement `agent.rs` with `AgentManagerProxy` trait and `BluetoothAgent` struct implementing `org.bluez.Agent1` via `#[zbus::interface]`
- Implement `register_pairing_agent` function: register at `org.bluez.AgentManager1` as default agent with `DisplayYesNo` capability
- Implement auto-accept for `RequestConfirmation`, `RequestAuthorization`, `DisplayPinCode`, `DisplayPasskey`
- Implement reject for `RequestPinCode`, `RequestPasskey` (require UI — future enhancement)
- Export agent object on D-Bus object server at `/smearor/bluetooth/agent`
- Implement `dbus.rs` with `get_adapter_state`, `get_all_devices`, `get_connected_devices`, `start_discovery`, `stop_discovery`, `connect_device`,
  `disconnect_device`, `pair_device`, `remove_device`, `set_powered`, `set_discoverable`
- Implement `dbus.rs` with `is_relevant_property` filter for `PropertiesChanged` signals
- Implement `service.rs` with `BluetoothService` struct
- Implement `ServicePlugin` trait (`on_message`, `start`)
- Implement `MessageHandler<FfiEnvelopePayload<BluetoothCommandMessage>>` trait
- Implement `MessageBroadcaster` trait
- Implement `MessageTopicBroadcaster<BluetoothStatusMessage>` trait
- Implement `MessageTopicBroadcaster<ScanResultsMessage>` trait
- Implement `MessageTopicBroadcaster<DeviceEventMessage>` trait for device connect/disconnect events
- Implement `MessageTopicBroadcaster<DeviceProximityEvent>` trait for proximity events
- Implement `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `McpCapabilitiesRegistrator` trait and register MCP tools
- Implement `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` and `MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>>` for MCP
- Implement `new(config, core_context)` constructor with `register_json_converters` call
- Implement `start()` with `std::thread::spawn` + `tokio::runtime::Builder::new_current_thread().enable_all()` + `LocalSet`
- Register pairing agent in `start()` after D-Bus connection is established (if `enable_pairing_agent` is true)
- Implement `run_bluetooth_async` with `tokio::select!` for D-Bus signal streams + command channel + fallback interval
- Implement signal subscriptions: `receive_properties_changed`, `receive_interfaces_added`, `receive_interfaces_removed`
- Implement initial `do_refresh` call on startup for current state
- Implement `do_refresh` to fetch adapter state and connected devices, broadcast `BluetoothStatusMessage`
- Implement `send_status` function for broadcasting messages via `FfiCoreContext`
- Implement `BluetoothServiceConfig` with serde defaults, including `automation` rules and `enable_pairing_agent` flag
- Implement `handle_device_event` function: broadcast `DeviceEventMessage`, evaluate `automation` rules, execute `on_connect`/`on_disconnect` actions
- Implement `send_automation_action` function for dispatching automation actions to configured topics
- Use `service_plugin!(BluetoothService);` macro in `lib.rs`
- Use `tokio::sync::mpsc::unbounded_channel` for command channel (not `std::sync::mpsc`)
- No `unwrap()` or `expect()` in production code

**Exit Criteria:** `cargo build -p smearor_bluetooth_service` succeeds. Service loads and broadcasts status.

### Phase 3: Widget Crate (`plugins/bluetooth`)

**Order:** Third — depends on Phase 1 and Phase 2.

**Tasks:**

- Create `plugins/bluetooth/Cargo.toml` with `gtk4`, `glib`, `plugin-api`, `model/bluetooth`, `model/personalization` dependencies
- Implement `config.rs` with `BluetoothWidgetConfig` struct using shared config structs (`WidgetDimensions`, `WidgetLayout`, `WidgetIcon`, `WidgetTextColors`,
  `WidgetMode`) via `#[serde(flatten)]`
- Implement `BluetoothIcons` struct with all Bluetooth-specific icon fields and `Default` impl, used via `#[serde(flatten)]` in `BluetoothWidgetConfig`
- Use `ActionBindings` via `#[serde(flatten)]` for gesture bindings (replaces `click_topic`/`longpress_topic`)
- Support `BindingMode` (`replace`/`supplement`) per binding via `click_mode`/`longpress_mode`/etc. TOML fields
- Implement `widget.rs` with `BluetoothWidget` struct
- Implement `WidgetPlugin` trait (`on_message`, `start`)
- Implement `WidgetBuilder` trait (`build_widget`)
- Implement `MessageHandler<FfiEnvelopePayload<BluetoothStatusMessage>>` trait
- Implement `MessageHandler<FfiEnvelopePayload<ScanResultsMessage>>` trait
- Implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` trait for locale-aware labels
- Implement `MessageBroadcaster` trait
- Implement `MessageTopicBroadcaster<BluetoothCommandMessage>` trait
- Implement `MessageTopicBroadcaster<WidgetUpdateMessage>` trait for headless/Web instance sync
- Implement `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `DefaultFallback` trait for view-dependent click behavior
- Implement `AcceptTopic<FfiEnvelope>` trait for topic filtering
- Implement `GestureHandler` trait and call `attach_gesture_handlers` in `build_widget`
- Implement `render_view` returning `ViewData` for all `BluetoothView` variants
- Implement `GraphicRenderer::render_graphic` for headless instance pixel rendering
- Implement `WebRenderer::render_html` for web instance HTML fragment rendering
- Use `resolve_gtk_nerd_icon()` for GTK icon resolution and `resolve_icon_codepoint()` for pixel/atomic rendering
- Implement `icon_for_device_type` helper on `BluetoothIcons` using `BluetoothDeviceType::from_bluez_icon`
- Implement `BluetoothLabel` struct for locale-aware labels (analogous to `NetworkLabel`)
- Implement `update_ui` with `glib::MainContext::default().spawn_local` for GTK updates
- Implement `broadcast_widget_update` after every UI update
- Implement `start_listeners` subscribing to `TOPIC_STATUS`, `TOPIC_SCAN_RESULTS`, and `TOPIC_PERSONALIZATION_STATUS`
- Use `glib::MainContext::default().spawn_local` for GTK updates
- Use `tokio::sync::mpsc` for message reception
- Use `widget_plugin!(BluetoothWidget);` macro in `lib.rs`
- No polling loops (`timeout_add_local`); use event-driven `recv().await`
- No `unwrap()` or `expect()` in production code

**Exit Criteria:** `cargo build -p smearor_bluetooth_widget` succeeds. Widget displays Bluetooth status and responds to clicks.

### Phase 4: Airplane Mode Coordination

**Order:** Fourth — depends on Phase 2 and Phase 3.

**Tasks:**

- Update Network Widget `Airplane` view click handler to also broadcast `BluetoothCommandMessage` with `AirplaneMode` action
- Bluetooth Service handles `AirplaneMode` command by calling `set_powered(!enabled)`
- Widget subscribes to both `service.network.status` and `service.bluetooth.status` for combined airplane mode state

**Exit Criteria:** Toggling airplane mode in the Network Widget disables WiFi, WWAN, and Bluetooth simultaneously.

### Phase 5: Workspace Wiring

**Order:** Fifth — depends on all previous phases.

**Tasks:**

- Add `model/bluetooth`, `services/bluetooth`, `plugins/bluetooth` to workspace `Cargo.toml`
- Add service loading to `smearor-swipe-launcher/src/service/loaded_service.rs` or service discovery
- Add plugin loading to `smearor-swipe-launcher/src/plugin/loaded_plugin.rs` or plugin discovery
- Add default config entries to `config.toml`
- Add `bluetooth_area` to area configuration

**Exit Criteria:** Launcher starts with Bluetooth service and widget loaded. `config.toml` contains Bluetooth entries.

### Phase 6: Integration and Tests

**Order:** Sixth — depends on all previous phases.

**Tasks:**

- Verify Bluetooth power toggle works (icon changes, status updates via `PropertiesChanged` signal)
- Verify device connect/disconnect works (status updates via `PropertiesChanged` signal)
- Verify scan starts and results appear (devices appear via `InterfacesAdded` signal)
- Verify airplane mode coordination between Network and Bluetooth
- Verify battery level display for devices that report it
- Verify view rotation (swipe up/down)
- Verify long-press opens `bluetooth_area`
- Test with no Bluetooth adapter (graceful degradation)
- Test with adapter powered off (correct icon and status)
- Test fallback interval triggers refresh when BlueZ restarts (missed signals)
- Test irrelevant `PropertiesChanged` signals (e.g. `RSSI`) do not trigger unnecessary refreshes
- Verify automation rules: `on_connect`/`on_disconnect` actions fire on device events
- Verify `DeviceEventMessage` is broadcast on `TOPIC_DEVICE_EVENT` for connect/disconnect
- Verify `bluetooth_area` quick-connect tiles appear for paired devices and connect/disconnect on click
- Test automation with no matching rules (no actions fired)
- Test `bluetooth_area` with no paired devices (only close button and external apps shown)

**Exit Criteria:** All tests pass. Bluetooth widget is fully functional.

### Phase 7: Documentation

**Order:** Seventh — depends on all previous phases.

**Tasks:**

- Update `book/src/SUMMARY.md` with Bluetooth-related chapters
- Add `book/src/features/bluetooth.md` describing the Bluetooth widget, views, and configuration
- Add `book/src/architecture/bluetooth.md` describing the service architecture, D-Bus integration, and event-driven updates
- Update `book/src/configuration/` with Bluetooth service and widget config examples
- Update `README.md` feature list to include Bluetooth widget and service
- Document `bluetooth_area` quick-connect area in the book
- Document Bluetooth-Automation (`on_connect`/`on_disconnect`) in the book
- Document Airplane Mode coordination between Network and Bluetooth in the book

**Exit Criteria:** `mdbook build` succeeds. README.md lists Bluetooth as a feature. Book contains Bluetooth documentation.

---

## 9. Dependencies

| Crate                | Dependencies                                                             |
|----------------------|--------------------------------------------------------------------------|
| `model/bluetooth`    | `stabby` (with `serde` feature), `serde`, `serde_json`                   |
| `services/bluetooth` | `zbus`, `tokio`, `tracing`, `plugin-api`, `model/bluetooth`              |
| `plugins/bluetooth`  | `gtk4`, `glib`, `plugin-api`, `model/bluetooth`, `model/personalization` |

---

## 10. Error Handling

- All D-Bus calls use `Result<T, E>` with proper error logging via `error!`
- Missing Bluetooth adapter: service broadcasts `powered: false` status, widget shows "Off"
- Device connection failures: logged with `error!`, status refresh follows
- No `unwrap()` or `expect()` in production code
- Graceful degradation when BlueZ is not running

---

## 11. Icon Reference

| Icon Name                 | Nerd Font Icon                  | Usage                               |
|---------------------------|---------------------------------|-------------------------------------|
| `icon_bluetooth_on`       | `nf-md-bluetooth`               | Bluetooth powered on                |
| `icon_bluetooth_off`      | `nf-md-bluetooth_off`           | Bluetooth powered off               |
| `icon_bluetooth_audio`    | `nf-md-bluetooth_audio`         | Audio device connected              |
| `icon_bluetooth_transfer` | `nf-md-bluetooth_transfer`      | Data transfer active                |
| `icon_bluetooth_battery`  | `nf-md-battery_bluetooth`       | Generic battery (no level reported) |
| `icon_battery_10`         | `nf-md-battery_10_bluetooth`    | Battery level <= 20%                |
| `icon_battery_20`         | `nf-md-battery_20_bluetooth`    | Battery level <= 30%                |
| `icon_battery_30`         | `nf-md-battery_30_bluetooth`    | Battery level <= 40%                |
| `icon_battery_40`         | `nf-md-battery_40_bluetooth`    | Battery level <= 50%                |
| `icon_battery_50`         | `nf-md-battery_50_bluetooth`    | Battery level <= 60%                |
| `icon_battery_60`         | `nf-md-battery_60_bluetooth`    | Battery level <= 70%                |
| `icon_battery_70`         | `nf-md-battery_70_bluetooth`    | Battery level <= 80%                |
| `icon_battery_80`         | `nf-md-battery_80_bluetooth`    | Battery level <= 90%                |
| `icon_battery_90`         | `nf-md-battery_90_bluetooth`    | Battery level > 90%                 |
| `icon_battery_alert`      | `nf-md-battery_alert_bluetooth` | Low battery alert (<= 10%)          |
| `icon_bluetooth_settings` | `nf-md-bluetooth_settings`      | Settings / scan view                |
| `icon_speaker`            | `nf-md-speaker_bluetooth`       | Speaker device connected            |
| `icon_airplane_on`        | `nf-md-airplane`                | Airplane mode on                    |
| `icon_airplane_off`       | `nf-md-airplane_off`            | Airplane mode off                   |
| `icon_device_headphones`  | `nf-md-headphones`              | Headphones / headset device         |
| `icon_device_speaker`     | `nf-md-speaker`                 | Speaker device                      |
| `icon_device_keyboard`    | `nf-md-keyboard`                | Keyboard device                     |
| `icon_device_mouse`       | `nf-mouse`                      | Mouse / pointing device             |
| `icon_device_gaming`      | `nf-md-gamepad_variant`         | Gaming controller                   |
| `icon_device_phone`       | `nf-md-cellphone`               | Phone device                        |
| `icon_device_computer`    | `nf-md-laptop`                  | Computer / laptop                   |
| `icon_device_camera`      | `nf-md-camera`                  | Camera device                       |
| `icon_device_printer`     | `nf-md-printer`                 | Printer / scanner                   |
| `icon_device_wearable`    | `nf-md-watch`                   | Wearable / watch                    |
| `icon_device_network`     | `nf-md-router_network`          | Network access point / router       |
| `icon_device_unknown`     | `nf-md-bluetooth`               | Unknown / unmapped device           |

---

## 12. Personalization Integration

The Bluetooth widget subscribes to `TOPIC_PERSONALIZATION_STATUS` (from the Personalization service) to receive locale updates. When a
`PersonalizationStatusMessage` arrives, the widget stores it in `latest_personalization`
and triggers a UI re-render.

The `BluetoothLabel` struct (see Section 5.3) uses the locale from `PersonalizationStatusMessage` to select appropriate label strings for all view text. This is
analogous to `NetworkLabel` in the Network widget.

The widget must implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` and
`AcceptTopic<FfiEnvelope>` filtering for `TOPIC_PERSONALIZATION_STATUS`.

---

## 13. Future Enhancements

- **Signal strength**: Display RSSI for connected devices (if reported by BlueZ)
- **Multiple adapter support**: Handle systems with more than one Bluetooth adapter
- **File transfer**: Integrate with `obexd` for Bluetooth file transfer UI
- **Audio-Codec-Anzeige & Umschaltung**: BlueZ exponiert den aktiven Codec via A2DP-Properties (`SBC`, `AAC`, `LDAC`, `aptX`, `aptX HD`). Widget könnte Codec im
  `ConnectedDevices`-View anzeigen. Umschaltung via `SetProperty` auf `org.bluez.MediaTransport1`. Besonders interessant in Kombination mit dem bestehenden
  Audio-Widget.
- **Geräte-Profile-Anzeige**: BlueZ meldet aktive Profile pro Gerät (`A2DP`, `HFP/HSP`, `AVRCP`). Widget könnte im `ConnectedDevices`-View anzeigen, ob ein
  Headset gerade im Audio- oder Headset-Modus ist. Relevant für Audio-Qualität (A2DP = Stereo, HFP = Mono+Mic).
- **Auto-Connect-Profile**: Konfigurierbare Liste von "Favorite Devices", die automatisch verbunden werden sobald sie verfügbar sind. Service-seitig via
  `InterfacesAdded`-Signal + `Connect()`-Call. Config in `BluetoothServiceConfig`.
- **Bluetooth-Widget als Multi-Device-Widget**: Statt nur das erste verbundene Gerät anzuzeigen, eine scrollbare Liste aller verbundenen Geräte mit
  individuellen Icons und Batterie-Levels. View `ConnectedDevices` würde eine `GtkListBox` oder `GtkFlowBox` verwenden. Größerer UI-Aufwand.
- **Bluetooth-LE-Advertising**: Launcher als BLE-Beacon (Advertising-Daten senden). `org.bluez.LEAdvertisingManager1`. Könnte für Presence-Detection oder
  Smart-Home-Integration genutzt werden.
- **Full Pairing Agent mit PIN/Passkey UI-Integration**: Der aktuell implementierte Auto-Accept-Agent lehnt `RequestPinCode` und `RequestPasskey` ab, was das
  Pairing von PIN-pflichtigen Geräten (z.B. Tastaturen) verhindert. Eine vollständige Implementierung würde eine `BluetoothPairingRequest`-Message auf neuem
  Topic `service.bluetooth.pairing_request` broadcasten, der Widget zeigt einen PIN-Eingabe-Dialog in `bluetooth_area`, und der Service reicht die eingegebene
  PIN an BlueZ weiter. Erfordert zusätzliche Model-Typen (`BluetoothPairingRequest`, `BluetoothPairingResponse`) und Widget-UI-Komponenten.
