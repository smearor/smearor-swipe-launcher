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
    /// Whether to enable or disable (for toggle actions).
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
    /// Airplane mode toggle (shared with Network service).
    AirplaneMode,
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
    ScanResults,
    /// Airplane mode status: on or off.
    /// Clicking the tile in this view toggles airplane mode (coordinates with Network service).
    Airplane,
    /// Battery status: shows battery level of the first connected device that reports it.
    Battery,
}
```

### 3.7 JSON Converters

The model crate includes JSON converter functions for FFI serialization, analogous to `model/network/src/json_converters.rs`:

- `parse_bluetooth_status(value: &serde_json::Value) -> BluetoothStatusMessage`
- `parse_device_status(value: &serde_json::Value) -> DeviceStatus`
- `parse_scan_results(value: &serde_json::Value) -> ScanResultsMessage`
- `parse_command_message(value: &serde_json::Value) -> BluetoothCommandMessage`

All FFI-relevant types carry `#[stabby::stabby]`.

---

## 4. Service Crate (`services/bluetooth`)

### 4.1 Overview

The Bluetooth Service is a singleton background service that communicates with **BlueZ** via D-Bus. It periodically polls the adapter state and connected
devices, publishes status updates on `TOPIC_STATUS`, and processes incoming commands on `TOPIC_COMMAND`.

### 4.2 BlueZ D-Bus Interfaces

| Interface                            | Object Path                 | Methods / Properties Used                                                                          |
|--------------------------------------|-----------------------------|----------------------------------------------------------------------------------------------------|
| `org.bluez.Adapter1`                 | `/org/bluez/hci0`           | `Powered`, `Discoverable`, `Discovering`, `StartDiscovery`, `StopDiscovery`                        |
| `org.bluez.Device1`                  | `/org/bluez/hci0/dev_XX_XX` | `Connect`, `Disconnect`, `Pair`, `RemoveDevice`, `Connected`, `Name`, `Address`, `Icon`, `Battery` |
| `org.freedesktop.DBus.ObjectManager` | `/`                         | `GetManagedObjects` — enumerate all adapters and devices                                           |
| `org.freedesktop.DBus.Properties`    | any BlueZ object            | `PropertiesChanged` signal (future: event-driven updates)                                          |

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
}
```

### 4.4 Core Functions (`dbus.rs`)

| Function                              | Description                                                   |
|---------------------------------------|---------------------------------------------------------------|
| `get_adapter(connection)`             | Returns the first available `Adapter1Proxy`                   |
| `get_adapter_state(connection)`       | Returns `(powered, discoverable, discovering, address, name)` |
| `get_all_devices(connection)`         | Enumerates all devices via `GetManagedObjects`                |
| `get_connected_devices(connection)`   | Returns `Vec<DeviceStatus>` for connected devices only        |
| `start_discovery(connection)`         | Calls `StartDiscovery` on the adapter                         |
| `stop_discovery(connection)`          | Calls `StopDiscovery` on the adapter                          |
| `connect_device(connection, addr)`    | Connects to a device by address                               |
| `disconnect_device(connection, addr)` | Disconnects from a device by address                          |
| `pair_device(connection, addr)`       | Pairs with a device by address                                |
| `remove_device(connection, addr)`     | Removes a paired device from the adapter                      |
| `set_powered(connection, powered)`    | Enables or disables the adapter                               |
| `set_discoverable(connection, on)`    | Toggles discoverable mode                                     |

### 4.5 Service Struct (`service.rs`)

```rust
pub struct BluetoothService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: BluetoothServiceConfig,
    pub command_receiver: Option<std::sync::mpsc::Receiver<BluetoothCommand>>,
    pub command_sender: std::sync::mpsc::Sender<BluetoothCommand>,
    pub shared_state: Arc<Mutex<BluetoothSharedState>>,
}

impl BluetoothService {
    /// Starts the async runtime in a separate thread.
    pub fn start(&mut self) {
        // Spawns a thread with Tokio current_thread runtime + LocalSet
        // Analogous to NetworkService::start
    }
}
```

### 4.6 Async Loop (`run_bluetooth_async`)

```rust
async fn run_bluetooth_async(
    meta: PluginMeta,
    core_context: FfiCoreContext,
    mut command_receiver: tokio::sync::mpsc::Receiver<BluetoothCommand>,
    config: BluetoothServiceConfig,
    shared_state: Arc<Mutex<BluetoothSharedState>>,
) {
    let connection = zbus::Connection::system().await.unwrap();
    let mut interval = tokio::time::interval(Duration::from_secs(config.refresh_interval_seconds));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                do_refresh(&connection, &shared_state, &status_sender).await;
            }
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
                }
                do_refresh(&connection, &shared_state, &status_sender).await;
            }
        }
    }
}
```

### 4.7 Service Config

```rust
/// Configuration for the Bluetooth service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BluetoothServiceConfig {
    /// Refresh interval in seconds for status polling.
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_seconds: u64,
    /// Whether to enable device scanning support.
    #[serde(default = "default_true")]
    pub enable_scanning: bool,
    /// Maximum number of devices to include in scan results.
    #[serde(default = "default_max_devices")]
    pub max_devices: usize,
}

fn default_refresh_interval() -> u64 { 5 }
fn default_true() -> bool { true }
fn default_max_devices() -> usize { 15 }
```

### 4.8 MCP Tools

The service exposes MCP tools for external automation:

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

The widget mirrors the Network Widget architecture with a compact tile, view-based rotation, and `gtk4::Image` for Nerd Font icons:

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
}
```

### 5.2 Widget Config

```rust
pub const DEFAULT_WIDTH: i32 = 100;
pub const DEFAULT_HEIGHT: i32 = 100;
pub const DEFAULT_SPACING: i32 = 0;
pub const DEFAULT_BUTTON_SIZE: i32 = 48;
pub const DEFAULT_ICON_SIZE: i32 = 36;

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

/// Configuration for the Bluetooth widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct BluetoothWidgetConfig {
    /// Width of the widget tile in pixels.
    #[builder(default = DEFAULT_WIDTH)]
    pub(crate) width: i32,

    /// Height of the widget tile in pixels.
    #[builder(default = DEFAULT_HEIGHT)]
    pub(crate) height: i32,

    /// Spacing between elements in pixels.
    #[builder(default = DEFAULT_SPACING)]
    pub(crate) spacing: i32,

    /// Button size in pixels (touch target sizing).
    #[builder(default = DEFAULT_BUTTON_SIZE)]
    pub(crate) button_size: i32,

    /// Icon size in pixels (Nerd Font icon images).
    #[builder(default = DEFAULT_ICON_SIZE)]
    pub(crate) icon_size: i32,

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

    /// Views to cycle through on swipe up/down.
    #[builder(default)]
    pub(crate) views: Vec<BluetoothView>,

    /// Maximum number of devices to show in scan results.
    #[builder(default = 10)]
    pub(crate) max_devices: usize,

    /// Message topic for single-click (opens the bluetooth menu area).
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Target instance for single-click message.
    #[serde(default)]
    pub click_instance: Option<String>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press.
    #[serde(default)]
    pub longpress_payload: Option<Value>,

    /// Target instance for long-press message.
    #[serde(default)]
    pub longpress_instance: Option<String>,
}
```

### 5.3 View Rendering

```rust
fn render_view(
    status: &BluetoothStatusMessage,
    scan: Option<&ScanResultsMessage>,
    config: &BluetoothWidgetConfig,
    view: BluetoothView,
) -> (String, String, String) {
    match view {
        BluetoothView::PowerStatus => {
            if status.powered {
                let name = status.adapter_name.to_string();
                let connected_count = status.connected_devices.len();
                (
                    config.icon_bluetooth_on.clone(),
                    "On".to_string(),
                    if connected_count > 0 {
                        format!("{connected_count} device(s)")
                    } else {
                        name
                    },
                )
            } else {
                (config.icon_bluetooth_off.clone(), "Off".to_string(), "Bluetooth".to_string())
            }
        }
        BluetoothView::ConnectedDevices => {
            match status.connected_devices.first() {
                Some(device) => {
                    let icon = if device.transferring {
                        config.icon_bluetooth_transfer.clone()
                    } else if device.device_type.contains("audio") || device.device_type.contains("headphone") {
                        config.icon_bluetooth_audio.clone()
                    } else if device.device_type.contains("speaker") {
                        config.icon_speaker.clone()
                    } else {
                        config.icon_bluetooth_on.clone()
                    };
                    let name = device.name.to_string();
                    let info = if device.connected { "Connected" } else { "Disconnected" }.to_string();
                    (icon, name, info)
                }
                None => (config.icon_bluetooth_off.clone(), "--".to_string(), "No devices".to_string()),
            }
        }
        BluetoothView::ScanResults => {
            match scan {
                Some(scan) => {
                    let count = scan.devices.len();
                    (config.icon_bluetooth_settings.clone(), format!("{count} found"), "Scan results".to_string())
                }
                None => (config.icon_bluetooth_settings.clone(), "--".to_string(), "No scan".to_string()),
            }
        }
        BluetoothView::Airplane => {
            if !status.powered {
                (config.icon_airplane_on.clone(), "ON".to_string(), "Airplane".to_string())
            } else {
                (config.icon_airplane_off.clone(), "OFF".to_string(), "Airplane".to_string())
            }
        }
        BluetoothView::Battery => {
            match status.connected_devices.iter().find(|d| d.battery_level.is_some()) {
                Some(device) => {
                    let level = device.battery_level.as_ref().map(|l| *l).unwrap_or(0);
                    let icon = if level <= 10 {
                        config.icon_battery_alert.clone()
                    } else if level <= 20 {
                        config.icon_battery_10.clone()
                    } else if level <= 30 {
                        config.icon_battery_20.clone()
                    } else if level <= 40 {
                        config.icon_battery_30.clone()
                    } else if level <= 50 {
                        config.icon_battery_40.clone()
                    } else if level <= 60 {
                        config.icon_battery_50.clone()
                    } else if level <= 70 {
                        config.icon_battery_60.clone()
                    } else if level <= 80 {
                        config.icon_battery_70.clone()
                    } else if level <= 90 {
                        config.icon_battery_80.clone()
                    } else {
                        config.icon_battery_90.clone()
                    };
                    let name = device.name.to_string();
                    (icon, format!("{level}%"), name)
                }
                None => (config.icon_bluetooth_battery.clone(), "--".to_string(), "No battery".to_string()),
            }
        }
    }
}
```

### 5.4 Click Handling (View-Dependent)

| View               | Click Action                                               |
|--------------------|------------------------------------------------------------|
| `PowerStatus`      | Toggle Bluetooth power via `TogglePower` command           |
| `ConnectedDevices` | Disconnect first connected device via `DisconnectDevice`   |
| `ScanResults`      | Start a scan via `StartScan` command                       |
| `Airplane`         | Toggle airplane mode via `AirplaneMode` command            |
| `Battery`          | Broadcast `click_topic` (typically opens `bluetooth_area`) |

### 5.5 View Switching

View switching uses `GestureDrag` (swipe up/down), identical to the Network and Weather widgets:

```rust
let drag_gesture = GestureDrag::new();
drag_gesture.set_propagation_phase(PropagationPhase::Capture);
drag_gesture.connect_drag_end( move | gesture, offset_x, offset_y| {
const SWIPE_THRESHOLD: f64 = 50.0;
if offset_y.abs() > offset_x.abs() & & offset_y.abs() > SWIPE_THRESHOLD {
gesture.set_state(EventSequenceState::Claimed);
if offset_y < 0.0 {
widget_self.next_view();
} else {
widget_self.prev_view();
}
}
});
outer_box.add_controller(drag_gesture);
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
// Bluetooth: toggle power (is_on = current airplane state, send as enabled = is_on to turn radios off)
let bt_command = BluetoothCommandMessage {
action: BluetoothCommandAction::AirplaneMode,
address: StabbyOption::None(),
enabled: is_on,
};
broadcaster.broadcast_message_to_topic(bt_command);
}
```

---

## 7. Config Integration

### 7.1 Service Config (`config.toml`)

```toml
[services.bluetooth]
refresh_interval_seconds = 5
enable_scanning = true
max_devices = 15
```

### 7.2 Widget Config (`config.toml`)

```toml
[[plugins]]
id = "bluetooth_widget"
type = "bluetooth"

[plugins.config]
width = 100
height = 100
spacing = 0
button_size = 48
icon_size = 36
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
max_devices = 10
views = ["PowerStatus", "ConnectedDevices", "ScanResults", "Airplane", "Battery"]
longpress_topic = "area.open"
longpress_payload = { area_id = "bluetooth_area" }
```

### 7.3 Bluetooth Area (`config.toml`)

The `bluetooth_area` scroll menu contains app launchers and detailed Bluetooth controls:

```toml
[[areas]]
id = "bluetooth_area"
# Contains: close button, bluetoothctl launcher, blueman-manager launcher, etc.
```

---

## 8. Implementation Phases

### Phase 1: Model Crate (`model/bluetooth`)

**Order:** First — no dependencies.

**Tasks:**

- Create `model/bluetooth/Cargo.toml` with `stabby`, `serde` dependencies
- Implement `messages/mod.rs` with topic constants
- Implement `messages/bluetooth_status.rs` with `BluetoothStatusMessage`, `DeviceStatus`
- Implement `messages/scan_results.rs` with `ScanResultsMessage`
- Implement `messages/command.rs` with `BluetoothCommandMessage`, `BluetoothCommandAction`
- Implement `messages/view.rs` with `BluetoothView` enum
- Implement `json_converters.rs` with parse functions
- Implement `lib.rs` with `pub use` re-exports
- Add `#[stabby::stabby]` to all FFI-relevant types
- Add `impl_json_convertible!` macro invocations for FFI registration

**Exit Criteria:** `cargo build -p smearor_bluetooth_model` succeeds.

### Phase 2: Service Crate (`services/bluetooth`)

**Order:** Second — depends on Phase 1.

**Tasks:**

- Create `services/bluetooth/Cargo.toml` with `zbus`, `tokio`, `tracing`, `plugin-api` dependencies
- Implement `dbus.rs` with `Adapter1Proxy`, `Device1Proxy`, `ObjectManagerProxy` traits
- Implement `dbus.rs` with `get_adapter_state`, `get_all_devices`, `get_connected_devices`, `start_discovery`, `stop_discovery`, `connect_device`,
  `disconnect_device`, `pair_device`, `remove_device`, `set_powered`, `set_discoverable`
- Implement `service.rs` with `BluetoothService` struct
- Implement `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `start()` with Tokio runtime + LocalSet
- Implement `run_bluetooth_async` with `tokio::select!` for interval + command channel
- Implement `do_refresh` to poll adapter state and connected devices, broadcast `BluetoothStatusMessage`
- Implement `BluetoothServiceConfig` with serde defaults
- Use `service_plugin!(BluetoothService);` macro in `lib.rs`
- Use `tokio::sync::mpsc` for command channel (not `std::sync::mpsc`)
- Register MCP tools

**Exit Criteria:** `cargo build -p smearor_bluetooth_service` succeeds. Service loads and broadcasts status.

### Phase 3: Widget Crate (`plugins/bluetooth`)

**Order:** Third — depends on Phase 1 and Phase 2.

**Tasks:**

- Create `plugins/bluetooth/Cargo.toml` with `gtk4`, `glib`, `plugin-api`, `model/bluetooth` dependencies
- Implement `config.rs` with `BluetoothWidgetConfig` struct and `parse` method
- Implement `widget.rs` with `BluetoothWidget` struct
- Implement `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `build_widget` with `gtk4::Image`, `Label`, `GestureDrag`, `GestureClick`, `GestureLongPress`
- Implement `render_view` function for all `BluetoothView` variants
- Implement `update_ui` with MainContext::spawn_local for GTK updates
- Implement view-dependent click handling
- Implement `start_listeners` subscribing to `TOPIC_STATUS` and `TOPIC_SCAN_RESULTS`
- Use `glib::MainContext::spawn_local` for GTK updates
- Use `tokio::sync::mpsc` for message reception
- Use `widget_plugin!(BluetoothWidget);` macro in `lib.rs`
- No polling loops (`timeout_add_local`); use event-driven `recv().await`

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
- Add service loading to `smearor-swipe-launcher/src/service.rs` or service discovery
- Add plugin loading to `smearor-swipe-launcher/src/plugin.rs` or plugin discovery
- Add default config entries to `config.toml`
- Add `bluetooth_area` to area configuration

**Exit Criteria:** Launcher starts with Bluetooth service and widget loaded. `config.toml` contains Bluetooth entries.

### Phase 6: Integration and Tests

**Order:** Sixth — depends on all previous phases.

**Tasks:**

- Verify Bluetooth power toggle works (icon changes, status updates)
- Verify device connect/disconnect works
- Verify scan starts and results appear
- Verify airplane mode coordination between Network and Bluetooth
- Verify battery level display for devices that report it
- Verify view rotation (swipe up/down)
- Verify long-press opens `bluetooth_area`
- Test with no Bluetooth adapter (graceful degradation)
- Test with adapter powered off (correct icon and status)

**Exit Criteria:** All tests pass. Bluetooth widget is fully functional.

---

## 9. Dependencies

| Crate                | Dependencies                                                         |
|----------------------|----------------------------------------------------------------------|
| `model/bluetooth`    | `stabby`, `serde`, `serde_json`                                      |
| `services/bluetooth` | `zbus`, `tokio`, `tracing`, `plugin-api`, `model/bluetooth`          |
| `plugins/bluetooth`  | `gtk4`, `glib`, `plugin-api`, `model/bluetooth`, `qrcode` (optional) |

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

---

## 12. Future Enhancements

- **Event-driven updates**: Subscribe to BlueZ `PropertiesChanged` and `InterfacesAdded`/`InterfacesRemoved` signals instead of polling
- **Device type icons**: Map BlueZ device icons (e.g., `audio-headphones`, `input-keyboard`, `phone`) to dedicated Nerd Font icons
- **Signal strength**: Display RSSI for connected devices (if reported by BlueZ)
- **Multiple adapter support**: Handle systems with more than one Bluetooth adapter
- **Bluetooth LE (BLE)**: Support for BLE-specific operations (advertising, GATT services)
- **File transfer**: Integrate with `obexd` for Bluetooth file transfer UI
