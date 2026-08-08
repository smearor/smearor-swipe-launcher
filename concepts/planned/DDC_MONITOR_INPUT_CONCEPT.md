# Concept: DDC/CI Monitor Input Source Service & Atomic Widget

This document describes the concept for a **DDC/CI Monitor Input Source Service** and an **Atomic Widget** in the *Smearor Swipe Launcher*. The service
communicates with monitors using the
[`ddc`](https://crates.io/crates/ddc) and [`ddc-i2c`](https://crates.io/crates/ddc-i2c) Rust crates to switch the input source (VCP feature `0x60`) via DDC/CI
over I2C. The atomic widget provides a single-click button that switches a specific monitor (identified by serial number) to a configured input source.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/ddc`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/ddc`):** Singleton background service that enumerates monitors, reads EDID for identification, and switches input sources on
   command.
3. **Widget Crate (`plugins/ddc`):** Atomic GTK4 widget that sends a switch command to the service on click.

---

## 1. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| DDC Atomic Widget        |                 | DDC Service                |
| (configured with         |                 | (Singleton)                |
|  serial_number +         |                 |                            |
|  input_source)           |                 |  ddc_i2c::Enumerator       |
+--------------------------+                 |  -> I2cDdc per monitor     |
             |                               |  -> read_edid() -> EDID    |
             |  1. Command Message           |  -> set_vcp_feature(0x60)  |
             |  (switch input source)        |                            |
             |=============================> |                            |
             |  Topic: "service.ddc.command" |                            |
             |                               |                            |
             |                               |  2. Status Broadcast       |
             | <=============================|     Topic: "service.ddc.status"
             |                               |     Payload: DdcStatusMessage { ... }
+--------------------------+                 +----------------------------+
                             \               /
                              \             /
                          +---------------------+
                          |  DDC/CI over I2C    |
                          |  /dev/i2c-*         |
                          +---------------------+
```

The service registers **MCP tools** so that AI agents can switch monitor inputs programmatically.

---

## 2. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path            | Responsibility                                                               |
|-------------|-----------------|------------------------------------------------------------------------------|
| **Model**   | `model/ddc/`    | Shared structs, enums, topics, input source definitions, and message formats |
| **Service** | `services/ddc/` | Monitor enumeration via `ddc-i2c`, EDID parsing, VCP feature 0x60 switching  |
| **Widget**  | `plugins/ddc/`  | Atomic GTK4 widget that sends switch commands on click                       |

---

## 3. Model Crate (`model/ddc`)

### 3.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.ddc.command";
pub const TOPIC_STATUS: &str = "service.ddc.status";
```

### 3.2 Input Source Enum

The input source values correspond to VCP feature `0x60` (Input Source Select) as defined in the MCCS specification. Values can differ between monitor models —
the enum captures the standard VESA values. Monitor-specific mappings are documented in section 3.3.

```rust
/// Input source values for VCP feature 0x60 (Input Source Select).
///
/// These are the standard VESA MCCS values. Some monitors may use
/// different values for the same input — consult the monitor's
/// capabilities string or documentation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[stabby::stabby]
pub enum InputSource {
    /// Analog 15-pin (VGA) — value 0x01.
    Analog = 0x01,
    /// DVI — value 0x03.
    Dvi = 0x03,
    /// DVI-D — value 0x04.
    DviD = 0x04,
    /// DVI-A — value 0x05.
    DviA = 0x05,
    /// Component video — value 0x07.
    Component = 0x07,
    /// S-Video — value 0x08.
    SVideo = 0x08,
    /// Composite video — value 0x09.
    Composite = 0x09,
    /// Tuner — value 0x0A.
    Tuner = 0x0A,
    /// SCART — value 0x0B.
    Scart = 0x0B,
    /// DisplayPort-1 — value 0x0C.
    DisplayPort1 = 0x0C,
    /// DisplayPort-2 — value 0x0D.
    DisplayPort2 = 0x0D,
    /// DisplayPort-3 — value 0x0E.
    DisplayPort3 = 0x0E,
    /// DisplayPort-4 / DP — value 0x0F.
    DisplayPort4 = 0x0F,
    /// HDMI-1 — value 0x10.
    Hdmi1 = 0x10,
    /// HDMI-2 — value 0x11.
    Hdmi2 = 0x11,
    /// HDMI-3 — value 0x12.
    Hdmi3 = 0x12,
    /// USB-C / DisplayPort Alt Mode — value 0x1B.
    UsbC = 0x1B,
    /// Unknown or custom input source. Used when the monitor uses
    /// a non-standard value not covered by the standard enum.
    #[default]
    Unknown = 0x00,
}

impl InputSource {
    /// Returns the VCP feature value for this input source.
    pub fn vcp_value(&self) -> u16 {
        *self as u16
    }

    /// Parses an input source from a hex string (e.g. "0x0f", "0x1b").
    pub fn from_hex(hex: &str) -> Option<Self> {
        let value = u8::from_str_radix(hex.trim_start_matches("0x"), 16).ok()?;
        Self::from_value(value)
    }

    /// Creates an InputSource from a raw VCP value.
    pub fn from_value(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Analog),
            0x03 => Some(Self::Dvi),
            0x04 => Some(Self::DviD),
            0x05 => Some(Self::DviA),
            0x07 => Some(Self::Component),
            0x08 => Some(Self::SVideo),
            0x09 => Some(Self::Composite),
            0x0A => Some(Self::Tuner),
            0x0B => Some(Self::Scart),
            0x0C => Some(Self::DisplayPort1),
            0x0D => Some(Self::DisplayPort2),
            0x0E => Some(Self::DisplayPort3),
            0x0F => Some(Self::DisplayPort4),
            0x10 => Some(Self::Hdmi1),
            0x11 => Some(Self::Hdmi2),
            0x12 => Some(Self::Hdmi3),
            0x1B => Some(Self::UsbC),
            _ => None,
        }
    }

    /// Returns a human-readable name for this input source.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Analog => "Analog (VGA)",
            Self::Dvi => "DVI",
            Self::DviD => "DVI-D",
            Self::DviA => "DVI-A",
            Self::Component => "Component",
            Self::SVideo => "S-Video",
            Self::Composite => "Composite",
            Self::Tuner => "Tuner",
            Self::Scart => "SCART",
            Self::DisplayPort1 => "DisplayPort-1",
            Self::DisplayPort2 => "DisplayPort-2",
            Self::DisplayPort3 => "DisplayPort-3",
            Self::DisplayPort4 => "DisplayPort-4",
            Self::Hdmi1 => "HDMI-1",
            Self::Hdmi2 => "HDMI-2",
            Self::Hdmi3 => "HDMI-3",
            Self::UsbC => "USB-C",
            Self::Unknown => "Unknown",
        }
    }
}
```

### 3.3 Monitor-Specific Input Source Mappings

Different monitor models may use different VCP `0x60` values for the same physical input. The following table lists the mappings observed in the user's monitor
setup:

#### IIYAMA G-Master GB3261UHSCP-B1

| Input Name | VCP Value | InputSource Enum    |
|------------|-----------|---------------------|
| DP-1       | `0x0F`    | `DisplayPort4`      |
| DP-2       | `0x10`    | `Hdmi1` (mismatch!) |
| HDMI-1     | `0x11`    | `Hdmi2` (mismatch!) |
| HDMI-2     | `0x12`    | `Hdmi3` (mismatch!) |

#### DELL (model unknown)

| Input Name | VCP Value | InputSource Enum    |
|------------|-----------|---------------------|
| DP         | `0x0F`    | `DisplayPort4`      |
| HDMI       | `0x11`    | `Hdmi2` (mismatch!) |
| USB-C      | `0x1B`    | `UsbC`              |

> **Important:** The IIYAMA monitor uses `0x10` for DP-2 and `0x11` for HDMI-1, while the standard VESA mapping assigns `0x10` to HDMI-1 and `0x11` to HDMI-2.
> This means the `InputSource` enum's
> standard names may not match the actual physical input on every monitor. For this reason, the **Raw** input source mode (see 3.4) allows specifying an
> arbitrary hex value directly in the widget
> configuration, bypassing the enum mapping entirely.

### 3.4 Raw Input Source

For monitors with non-standard VCP `0x60` mappings, the widget config can specify a raw hex value instead of an `InputSource` enum variant. This provides
maximum flexibility without requiring enum changes for every monitor model.

```rust
/// Specifies an input source either by standard enum or by raw VCP value.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum InputSourceSpec {
    /// Standard VESA MCCS input source.
    Standard(InputSource),
    /// Raw VCP 0x60 value for monitors with non-standard mappings.
    /// The value is the raw hex code (e.g. 0x0F for DP-1 on IIYAMA).
    Raw(u8),
}

impl InputSourceSpec {
    /// Returns the VCP feature value to set.
    pub fn vcp_value(&self) -> u16 {
        match self {
            Self::Standard(source) => source.vcp_value(),
            Self::Raw(value) => *value as u16,
        }
    }

    /// Creates a raw input source spec from a hex string.
    pub fn raw_from_hex(hex: &str) -> Option<Self> {
        let value = u8::from_str_radix(hex.trim_start_matches("0x"), 16).ok()?;
        Some(Self::Raw(value))
    }
}
```

### 3.5 Monitor Info Struct

```rust
/// Information about a detected monitor, parsed from EDID.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct MonitorInfo {
    /// Manufacturer name (2-3 letter code from EDID).
    pub manufacturer: stabby::string::String,
    /// Monitor model name from EDID descriptor.
    pub model_name: stabby::string::String,
    /// Serial number from EDID descriptor.
    pub serial_number: stabby::string::String,
    /// I2C device path (e.g. "/dev/i2c-5").
    pub device_path: stabby::string::String,
}
```

### 3.6 Command Message (Widget -> Service)

```rust
/// Actions that the DDC service can perform.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum DdcCommandAction {
    /// Switch the input source of a specific monitor (by serial number).
    #[default]
    SwitchInputSource,
    /// Refresh the monitor list (re-enumerate via ddc_i2c).
    RefreshMonitors,
    /// Get the current input source of a specific monitor.
    GetInputSource,
}

/// Command message sent from the widget to the DDC service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DdcCommandMessage {
    /// The action to execute.
    pub action: DdcCommandAction,
    /// Serial number of the target monitor.
    pub serial_number: stabby::string::String,
    /// Input source to switch to (for SwitchInputSource action).
    pub input_source: stabby::option::Option<InputSourceSpec>,
}
```

### 3.7 Status Message (Service -> Widget)

```rust
/// DDC service status message broadcast after each operation.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DdcStatusMessage {
    /// List of detected monitors.
    pub monitors: stabby::vec::Vec<MonitorInfo>,
    /// Serial number of the last targeted monitor.
    pub target_serial: stabby::string::String,
    /// Current input source of the last targeted monitor (if known).
    pub current_input_source: stabby::option::Option<u8>,
    /// Whether the last operation succeeded.
    pub success: bool,
    /// Error message when the last operation failed.
    pub error_message: stabby::option::Option<stabby::string::String>,
    /// Timestamp of the last operation as ISO-8601 string.
    pub last_updated: stabby::string::String,
}
```

### 3.8 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::command::DdcCommandAction;
pub use messages::command::DdcCommandMessage;
pub use messages::input_source::InputSource;
pub use messages::input_source::InputSourceSpec;
pub use messages::monitor_info::MonitorInfo;
pub use messages::status::DdcStatusMessage;
pub use messages::topics::TOPIC_COMMAND;
pub use messages::topics::TOPIC_STATUS;
```

---

## 4. Service Crate (`services/ddc`)

### 4.1 File Structure

- `service.rs` - `DdcService` struct and trait implementations
- `config.rs` - `DdcServiceConfig` struct and parsing
- `ddc_handler.rs` - DDC/CI operations using `ddc-i2c` (enumerate, EDID parse, set VCP)
- `lib.rs` - `service_plugin!` macro invocation

### 4.2 Dependencies

```toml
[dependencies]
ddc = "0.3"
ddc-i2c = { version = "0.2", features = ["with-linux"] }
stabby = { workspace = true }
glib = { workspace = true }
miette = { workspace = true }
paste = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
smearor-model-mcp = { path = "../../model/mcp" }
smearor-ddc-model = { path = "../../model/ddc" }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

### 4.3 Service Implementation

```rust
pub struct DdcService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: DdcServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<DdcCommandAction>,
    pub latest_state: Arc<RwLock<DdcStatusMessage>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<DdcCommandMessage>>` - Processes switch/refresh/get commands from widgets
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 4.4 Configuration

```rust
/// Configuration for the DDC service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DdcServiceConfig {
    /// Whether to enumerate monitors on startup.
    pub enumerate_on_startup: bool,
    /// I2C bus filter. If set, only monitors on this bus are enumerated.
    /// If None, all I2C buses are scanned.
    pub i2c_bus_filter: Option<u32>,
}

impl Default for DdcServiceConfig {
    fn default() -> Self {
        Self {
            enumerate_on_startup: true,
            i2c_bus_filter: None,
        }
    }
}
```

### 4.5 DDC/CI Operations

The service uses `ddc_i2c::Enumerator` to enumerate all detected monitors and `ddc_i2c::I2cDdc` to communicate with each monitor. The `ddc::Ddc` trait provides
`set_vcp_feature` and
`get_vcp_feature` for reading and writing VCP feature values.

```rust
use ddc::Ddc;
use ddc::Edid;
use ddc_i2c::Enumerator;
use ddc_i2c::I2cDdc;

/// VCP feature code for Input Source Select.
const VCP_INPUT_SOURCE: u8 = 0x60;

/// Enumerate all detected monitors and return their info.
fn enumerate_monitors() -> Result<Vec<MonitorInfo>, DdcError> {
    let mut monitors = Vec::new();
    for device in Enumerator::new() {
        let mut ddc = match device {
            Ok(ddc) => ddc,
            Err(_) => continue,
        };

        // Read EDID (128 bytes base + optional extension)
        let mut edid_data = [0u8; 256];
        let bytes_read = ddc.read_edid(0, &mut edid_data).unwrap_or(0);
        let edid = &edid_data[..bytes_read];

        let (manufacturer, model_name, serial_number) = parse_edid_descriptors(edid);
        let device_path = format!("/dev/i2c-{}", ddc.device_path());

        monitors.push(MonitorInfo {
            manufacturer: manufacturer.into(),
            model_name: model_name.into(),
            serial_number: serial_number.into(),
            device_path: device_path.into(),
        });
    }
    Ok(monitors)
}

/// Switch the input source of a monitor identified by serial number.
fn switch_input_source(serial_number: &str, input_source: InputSourceSpec) -> Result<(), DdcError> {
    for device in Enumerator::new() {
        let mut ddc = match device {
            Ok(ddc) => ddc,
            Err(_) => continue,
        };

        // Read EDID to identify the monitor
        let mut edid_data = [0u8; 256];
        let bytes_read = ddc.read_edid(0, &mut edid_data).unwrap_or(0);
        let edid = &edid_data[..bytes_read];
        let (_, _, sn) = parse_edid_descriptors(edid);

        if sn == serial_number {
            // Found the target monitor — set VCP feature 0x60
            ddc.set_vcp_feature(VCP_INPUT_SOURCE, input_source.vcp_value())
                .map_err(DdcError::DdcCommunication)?;
            return Ok(());
        }
    }
    Err(DdcError::MonitorNotFound(serial_number.to_string()))
}

/// Get the current input source of a monitor identified by serial number.
fn get_input_source(serial_number: &str) -> Result<u8, DdcError> {
    for device in Enumerator::new() {
        let mut ddc = match device {
            Ok(ddc) => ddc,
            Err(_) => continue,
        };

        let mut edid_data = [0u8; 256];
        let bytes_read = ddc.read_edid(0, &mut edid_data).unwrap_or(0);
        let edid = &edid_data[..bytes_read];
        let (_, _, sn) = parse_edid_descriptors(edid);

        if sn == serial_number {
            let value = ddc.get_vcp_feature(VCP_INPUT_SOURCE)
                .map_err(DdcError::DdcCommunication)?;
            return Ok(value.value());
        }
    }
    Err(DdcError::MonitorNotFound(serial_number.to_string()))
}
```

### 4.6 EDID Parsing

The EDID (Extended Display Identification Data) format contains descriptor blocks at fixed offsets. The service parses these to extract the monitor name and
serial number without requiring an external EDID parsing crate.

```rust
/// Parse EDID descriptor blocks to extract manufacturer, model name, and serial number.
///
/// EDID contains four 18-byte descriptor blocks starting at offset 0x36.
/// Each descriptor has a header type:
/// - 0xFF: Serial Number (ASCII string)
/// - 0xFE: Unspecified text (often used for model name)
/// - 0xFC: Monitor Name (ASCII string)
fn parse_edid_descriptors(edid: &[u8]) -> (String, String, String) {
    let mut manufacturer = String::new();
    let mut model_name = String::new();
    let mut serial_number = String::new();

    // Manufacturer ID is at bytes 8-9 (2-byte big-endian encoded as 3 letters)
    if edid.len() >= 10 {
        manufacturer = parse_manufacturer_id(edid[8], edid[9]);
    }

    // Parse descriptor blocks (4 descriptors, 18 bytes each, starting at offset 0x36)
    for i in 0..4 {
        let offset = 0x36 + i * 18;
        if offset + 18 > edid.len() {
            break;
        }
        let descriptor = &edid[offset..offset + 18];

        // Check if this is a text descriptor (header type at byte 3)
        if descriptor[0] == 0x00 && descriptor[1] == 0x00 {
            let descriptor_type = descriptor[3];
            let text = parse_descriptor_text(&descriptor[5..18]);
            match descriptor_type {
                0xFF => serial_number = text,
                0xFC => model_name = text,
                0xFE => {
                    // Unspecified text — often used as fallback for model name
                    if model_name.is_empty() {
                        model_name = text;
                    }
                }
                _ => {}
            }
        }
    }

    (manufacturer, model_name, serial_number)
}

/// Parse a 2-byte manufacturer ID into a 3-letter string.
fn parse_manufacturer_id(byte1: u8, byte2: u8) -> String {
    let combined = ((byte1 as u16) << 8) | (byte2 as u16);
    let c1 = b'A' + (((combined >> 10) & 0x1F) as u8);
    let c2 = b'A' + (((combined >> 5) & 0x1F) as u8);
    let c3 = b'A' + ((combined & 0x1F) as u8);
    format!("{}{}{}", c1 as char, c2 as char, c3 as char)
}

/// Parse a descriptor text field (null-terminated, space-padded ASCII).
fn parse_descriptor_text(data: &[u8]) -> String {
    let text: String = data
        .iter()
        .take_while(|&&b| b != 0x0A && b != 0x00)
        .map(|&b| b as char)
        .collect();
    text.trim().to_string()
}
```

### 4.7 Background Event Loop

On initialization, the service spawns a dedicated OS thread with a single-threaded Tokio runtime. The runtime listens for commands from widgets and performs
DDC/CI operations.

```rust
async fn run_command_loop(
    config: DdcServiceConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<DdcCommandMessage>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    latest_state: Arc<RwLock<DdcStatusMessage>>,
) {
    // Initial monitor enumeration
    if config.enumerate_on_startup {
        let monitors = enumerate_monitors().unwrap_or_default();
        let status = DdcStatusMessage {
            monitors: monitors.into_iter().map(Into::into).collect(),
            success: true,
            ..Default::default()
        };
        *latest_state.write().await = status.clone();
        broadcast_status(&meta, &core_context, &status);
    }

    loop {
        if let Some(command) = command_receiver.recv().await {
            match command.action {
                DdcCommandAction::SwitchInputSource => {
                    let result = switch_input_source(
                        &command.serial_number,
                        command.input_source.unwrap_or(InputSourceSpec::Standard(InputSource::Unknown)),
                    );
                    let mut status = latest_state.write().await;
                    status.target_serial = command.serial_number.clone().into();
                    status.success = result.is_ok();
                    status.error_message = result.as_ref().err().map(|e| e.to_string().into());
                    status.last_updated = current_iso8601().into();
                    let status = status.clone();
                    drop(status);
                    broadcast_status(&meta, &core_context, &status);
                }
                DdcCommandAction::RefreshMonitors => {
                    let monitors = enumerate_monitors().unwrap_or_default();
                    let status = DdcStatusMessage {
                        monitors: monitors.into_iter().map(Into::into).collect(),
                        success: true,
                        ..Default::default()
                    };
                    *latest_state.write().await = status.clone();
                    broadcast_status(&meta, &core_context, &status);
                }
                DdcCommandAction::GetInputSource => {
                    let result = get_input_source(&command.serial_number);
                    let mut status = latest_state.write().await;
                    status.target_serial = command.serial_number.clone().into();
                    status.current_input_source = result.ok().map(|v| v as u8);
                    status.success = result.is_ok();
                    status.error_message = result.as_ref().err().map(|e| e.to_string().into());
                    status.last_updated = current_iso8601().into();
                    let status = status.clone();
                    drop(status);
                    broadcast_status(&meta, &core_context, &status);
                }
            }
        }
    }
}
```

### 4.8 MCP Tools

The service registers the following MCP tools:

| Tool                   | Description                                                       | Parameters                                                         |
|------------------------|-------------------------------------------------------------------|--------------------------------------------------------------------|
| `ddc_switch_input`     | Switch the input source of a monitor identified by serial number. | `serial_number: String`, `input_source: String` (hex or enum name) |
| `ddc_list_monitors`    | List all detected monitors with their serial numbers.             | —                                                                  |
| `ddc_get_input_source` | Get the current input source of a monitor.                        | `serial_number: String`                                            |
| `ddc_refresh_monitors` | Force a re-enumeration of connected monitors.                     | —                                                                  |

> **MCP tool naming convention:** Tool names use `snake_case` with underscores, never dots.

The `ddc_switch_input` tool accepts `input_source` as either a hex string (e.g. `"0x0f"`) or a standard enum name (e.g. `"DisplayPort4"`, `"UsbC"`). This
provides flexibility for both AI agents and human-readable configurations.

---

## 5. Widget Crate (`plugins/ddc`)

### 5.1 Overview

The DDC Atomic Widget is a single-purpose button that switches a specific monitor to a configured input source on click. It is designed for KVM
(Keyboard-Video-Mouse) switching scenarios where multiple computers share the same monitors.

The widget is an **Atomic Widget** following the pattern established by the Weather Atomic Widgets. It uses `AtomicWidgetConfig` for action bindings and renders
a simple icon + label layout.

### 5.2 File Structure

- `atomic.rs` - `DdcAtomicWidget` struct and trait implementations
- `config.rs` - `DdcAtomicWidgetConfig` struct and parsing
- `lib.rs` - `widget_factory_plugin_graphic!` macro invocation

### 5.3 Widget Configuration

```rust
use serde::Deserialize;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_swipe_launcher_plugin_api::typed_builder::TypedBuilder;

/// Configuration for the DDC atomic widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct DdcAtomicWidgetConfig {
    /// The atomic widget config (click/longpress bindings, render mode, etc.)
    #[builder(default, setter(into))]
    pub(crate) atomic: Option<AtomicWidgetConfig>,

    /// Serial number of the target monitor.
    /// Required — identifies which monitor to switch.
    pub serial_number: String,

    /// Input source to switch to on click.
    /// Can be a standard enum name (e.g. "DisplayPort4", "UsbC")
    /// or a raw hex value (e.g. "0x0f", "0x1b").
    pub input_source: String,

    /// Optional: display name for the input source (shown in the widget label).
    /// If not set, the input_source string is used.
    #[serde(default)]
    pub input_source_label: Option<String>,

    /// Optional: monitor label (shown in the widget info line).
    /// If not set, the serial number is used.
    #[serde(default)]
    pub monitor_label: Option<String>,
}
```

### 5.4 Widget Implementation

```rust
pub struct DdcAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: DdcAtomicWidgetConfig,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<DdcStatusMessage>>>,
}
```

**Trait Implementations:**

- `MessageHandler<DdcStatusMessage>` - Receives status updates from the service
- `MessageBroadcaster` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget UI
- `GraphicRenderer` - Renders headless pixel buffer for MacroPad

### 5.5 Click Action

On click, the widget constructs a `DdcCommandMessage` with `SwitchInputSource` action, the configured serial number, and the input source spec, then broadcasts
it to `TOPIC_COMMAND`.

```rust
fn on_click(&self) {
    let input_source_spec = if self.config.input_source.starts_with("0x") {
        InputSourceSpec::raw_from_hex(&self.config.input_source)
    } else {
        self.config.input_source.parse::<InputSource>()
            .ok()
            .map(InputSourceSpec::Standard)
    };

    let command = DdcCommandMessage {
        action: DdcCommandAction::SwitchInputSource,
        serial_number: self.config.serial_number.clone().into(),
        input_source: input_source_spec.map(|spec| spec.into()),
    };

    let broadcaster = self.get_broadcaster();
    broadcaster.broadcast_message_to_topic(command);
}
```

### 5.6 Rendering

The widget renders three elements:

| Element    | Content                                    |
|------------|--------------------------------------------|
| Icon       | `nf-md-monitor` or `nf-md-monitor_shimmer` |
| Main Label | Input source label (e.g. "DP-1", "USB-C")  |
| Info Label | Monitor label or serial number             |

When a status message arrives indicating success, the widget briefly shows a check icon (`nf-md-check`). On failure, it shows an error icon (`nf-md-alert`).

### 5.7 `lib.rs`

```rust
pub(crate) mod atomic;
pub(crate) mod config;

use crate::atomic::DdcAtomicWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "ddc_input" => ddc_input_widget => DdcAtomicWidget,
}
```

---

## 6. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
| DDC Atomic Widget |<--------|                   |-------->| DDC Service       |
| (tile in scroll   |  Status |   Event Broker    | Command | (Singleton)       |
|  band or MacroPad)| Broadcast                  Broadcast +-------------------+
+---------+---------+         +-------------------+         |                   |
          |                                                 | ddc_i2c::         |
          | Click: send DdcCommandMessage                   | Enumerator        |
          |       (SwitchInputSource)                       | I2cDdc            |
          v                                                 | set_vcp_feature   |
+-------------------+                               +-------------------+
| Widget updates    |                               | DDC/CI over I2C   |
| label on status   |                               | /dev/i2c-*        |
+-------------------+                               +-------------------+
```

---

## 7. Configuration Example

### 7.1 Service Registration in `services.toml`

```toml
[[services]]
id = "ddc"
path = "target/release/libsmearor_ddc_service.so"

[ddc]
enumerate_on_startup = true
```

### 7.2 Atomic Widget Configuration in `config.toml`

#### Example: Switch left monitor (IIYAMA, SN 1262853400794) to DP-1

```toml
[[scroll_band.plugins]]
id = "ddc_input_left_dp1"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[ddc_input_left_dp1]
serial_number = "1262853400794"
input_source = "0x0f"
input_source_label = "DP-1"
monitor_label = "Left"
```

#### Example: Switch right monitor (IIYAMA, SN 1262853400787) to HDMI-1

```toml
[[scroll_band.plugins]]
id = "ddc_input_right_hdmi1"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[ddc_input_right_hdmi1]
serial_number = "1262853400787"
input_source = "0x11"
input_source_label = "HDMI-1"
monitor_label = "Right"
```

#### Example: Switch left monitor to USB-C (Laptop)

```toml
[[scroll_band.plugins]]
id = "ddc_input_left_usbc"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[ddc_input_left_usbc]
serial_number = "1262853400794"
input_source = "0x1b"
input_source_label = "USB-C"
monitor_label = "Left"
```

### 7.3 KVM Switch Configuration (Computer 1)

The following configuration replicates the user's "Computer 1" shell script, switching both monitors to specific inputs:

```toml
# Left monitor -> DP-1 (Computer 1)
[[scroll_band.plugins]]
id = "kvm_pc1_left"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[kvm_pc1_left]
serial_number = "1262853400794"
input_source = "0x0f"
input_source_label = "DP-1"
monitor_label = "Left"

# Right monitor -> HDMI-1 (Computer 1)
[[scroll_band.plugins]]
id = "kvm_pc1_right"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[kvm_pc1_right]
serial_number = "1262853400787"
input_source = "0x11"
input_source_label = "HDMI-1"
monitor_label = "Right"
```

### 7.4 MacroPad Configuration

```toml
# KVM: Switch to Computer 1 (both monitors)
[[macropad.buttons]]
id = "kvm_pc1_left"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[kvm_pc1_left]
serial_number = "1262853400794"
input_source = "0x0f"
input_source_label = "PC1"
monitor_label = "L"

[[macropad.buttons]]
id = "kvm_pc1_right"
path = "target/release/libsmearor_ddc_widget.so"
widget = "ddc_input"

[kvm_pc1_right]
serial_number = "1262853400787"
input_source = "0x11"
input_source_label = "PC1"
monitor_label = "R"
```

---

## 8. Input Source Reference Table

### 8.1 Standard VESA MCCS VCP 0x60 Values

| Hex Value | InputSource Enum | Description                  |
|-----------|------------------|------------------------------|
| `0x01`    | `Analog`         | Analog 15-pin (VGA)          |
| `0x03`    | `Dvi`            | DVI                          |
| `0x04`    | `DviD`           | DVI-D                        |
| `0x05`    | `DviA`           | DVI-A                        |
| `0x07`    | `Component`      | Component video              |
| `0x08`    | `SVideo`         | S-Video                      |
| `0x09`    | `Composite`      | Composite video              |
| `0x0A`    | `Tuner`          | Tuner                        |
| `0x0B`    | `Scart`          | SCART                        |
| `0x0C`    | `DisplayPort1`   | DisplayPort-1                |
| `0x0D`    | `DisplayPort2`   | DisplayPort-2                |
| `0x0E`    | `DisplayPort3`   | DisplayPort-3                |
| `0x0F`    | `DisplayPort4`   | DisplayPort-4 / DP           |
| `0x10`    | `Hdmi1`          | HDMI-1                       |
| `0x11`    | `Hdmi2`          | HDMI-2                       |
| `0x12`    | `Hdmi3`          | HDMI-3                       |
| `0x1B`    | `UsbC`           | USB-C / DisplayPort Alt Mode |

### 8.2 Monitor-Specific Mappings

#### IIYAMA G-Master GB3261UHSCP-B1

| Input Name | Hex Value | Note                        |
|------------|-----------|-----------------------------|
| DP-1       | `0x0F`    | Standard DP                 |
| DP-2       | `0x10`    | Non-standard (VESA: HDMI-1) |
| HDMI-1     | `0x11`    | Non-standard (VESA: HDMI-2) |
| HDMI-2     | `0x12`    | Non-standard (VESA: HDMI-3) |

#### DELL Monitors

| Input Name | Hex Value | Note                        |
|------------|-----------|-----------------------------|
| DP         | `0x0F`    | Standard DP                 |
| HDMI       | `0x11`    | Non-standard (VESA: HDMI-2) |
| USB-C      | `0x1B`    | Standard USB-C              |

> **Recommendation:** Always use the raw hex value (`"0x0f"`, `"0x11"`, `"0x1b"`) in the widget configuration rather than the enum name, because
> monitor-specific mappings often do not align with the
> standard VESA values. The enum is primarily useful for AI agents and documentation.

---

## 9. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the DDC/CI Monitor Input Source feature.

### Phase 1: Foundation — Model Crate (`model/ddc`)

**Goal:** Define all shared messages, topics, input source types, and configuration types.

**Order:**

1. Create the crate `model/ddc` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_COMMAND` and `TOPIC_STATUS`.
3. Create one file per message struct:
    - `src/messages/input_source.rs` -> `InputSource` enum, `InputSourceSpec` enum
    - `src/messages/monitor_info.rs` -> `MonitorInfo` struct
    - `src/messages/command.rs` -> `DdcCommandAction` and `DdcCommandMessage`
    - `src/messages/status.rs` -> `DdcStatusMessage`
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.
- `InputSource::from_hex` and `InputSourceSpec::raw_from_hex` correctly parse hex strings.

---

### Phase 2: Backend — Service Crate (`services/ddc`)

**Goal:** Enumerate monitors, parse EDID, and switch input sources via DDC/CI.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/ddc` with a `Cargo.toml` that depends on the `model/ddc` crate, the project plugin API, `ddc`, `ddc-i2c`, `tokio`, and `tracing`.
2. Create `src/config.rs` with `DdcServiceConfig` and its default values.
3. Create `src/ddc_handler.rs` and implement:
    - `enumerate_monitors` using `ddc_i2c::Enumerator`.
    - `parse_edid_descriptors` to extract manufacturer, model name, and serial number from raw EDID bytes.
    - `switch_input_source` to find a monitor by serial number and call `set_vcp_feature(0x60, value)`.
    - `get_input_source` to read the current VCP 0x60 value.
4. Create `src/service/loaded_service.rs` with `DdcService` and all required trait implementations.
5. Implement `run_command_loop` to handle incoming commands and broadcast status.
6. Register MCP tools (`ddc_switch_input`, `ddc_list_monitors`, `ddc_get_input_source`, `ddc_refresh_monitors`).
7. Wire `service_plugin!` in `src/lib.rs`.
8. Add unit tests for EDID parsing with a sample EDID blob.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for EDID parsing correctly extract manufacturer, model name, and serial number.
- `enumerate_monitors` detects connected monitors and returns their info.
- `switch_input_source` successfully switches the input source on a target monitor.
- MCP tools are registered and return valid JSON when queried.
- Error handling covers: monitor not found, I2C communication failure, permission denied.

---

### Phase 3: Display — Widget Crate (`plugins/ddc`)

**Goal:** Provide an atomic widget that switches a monitor's input source on click.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/ddc` with a `Cargo.toml` that depends on `model/ddc`, the project plugin API, `gtk4`, `glib`, and `typed-builder`.
2. Create `src/config.rs` with `DdcAtomicWidgetConfig` including `serial_number`, `input_source`, and optional labels.
3. Create `src/atomic.rs` with `DdcAtomicWidget` and all required trait implementations.
4. Implement click handling: construct `DdcCommandMessage` with `SwitchInputSource` and broadcast to `TOPIC_COMMAND`.
5. Subscribe to `TOPIC_STATUS` and update the widget display on status changes.
6. Implement `GraphicRenderer` for headless MacroPad rendering.
7. Wire `widget_factory_plugin_graphic!` in `src/lib.rs`.
8. Add an integration test that verifies the widget sends the correct command on click.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- Click sends a `DdcCommandMessage` with the configured serial number and input source.
- The widget displays the input source label and monitor label.
- Status updates from the service update the widget display (success/error indicator).
- The widget works both in GTK scroll band and MacroPad (headless) contexts.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/ddc`, `services/ddc`, and `plugins/ddc` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add sample widget configurations for KVM switching in `config.toml` or area config files.
4. Document the udev rules required for I2C access (user must be in the `i2c` group).

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The DDC atomic widget can be configured and triggers input source switches on click.
- The user has documented I2C permissions setup instructions.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that monitor enumeration works on startup.
2. Verify the `ddc_switch_input` MCP tool switches a monitor's input source.
3. Verify the `ddc_list_monitors` MCP tool returns all connected monitors with serial numbers.
4. Verify the atomic widget switches the correct monitor on click.
5. Verify error handling: disconnect a monitor and confirm graceful error messages.
6. Run `cargo test` for all three crates.
7. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.
- KVM switching works correctly with both IIYAMA and DELL monitors.
- Error states (monitor not found, I2C permission denied) are handled gracefully.

---

### Summary of Order

```
Phase 1: model/ddc
    |
    v
Phase 2: services/ddc
    |
    v
Phase 3: plugins/ddc
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

---

## 10. Technical Notes

### 10.1 I2C Permissions

DDC/CI communication requires access to `/dev/i2c-*` devices. The user running the launcher must be a member of the `i2c` group:

```bash
sudo usermod -aG i2c $USER
```

Alternatively, a udev rule can be created to grant access to specific I2C buses used by monitors. The existing udev rule at `resources/udev/52-streamdeck.rules`
can serve as a template.

### 10.2 DDC/CI Timing Constraints

The DDC specification mandates delays between consecutive commands (typically 50-200 ms). The `ddc` crate's `DdcHost::sleep` method handles this automatically.
The service must not issue rapid-fire commands to the same monitor — the `ddc` crate enforces this internally.

### 10.3 EDID Parsing Without External Dependencies

The service parses EDID descriptor blocks manually rather than depending on an external EDID parsing crate. This avoids adding a dependency on a
rarely-maintained crate and keeps the parsing minimal — we only need the monitor name (descriptor type `0xFC`) and serial number (descriptor type `0xFF`).

### 10.4 Monitor Identification by Serial Number

In multi-monitor setups with identical monitor models (e.g. two IIYAMA G-Master GB3261UHSCP-B1), the monitor name alone is not sufficient to identify a specific
display. The EDID serial number provides a unique identifier for each physical monitor. The service matches monitors by serial number when executing commands.

### 10.5 Raw Hex Input Source Values

Because monitor manufacturers use non-standard VCP `0x60` values, the widget configuration uses raw hex values (e.g. `"0x0f"`, `"0x1b"`) by default. The
`InputSource` enum is provided for documentation and AI agent use, but the `InputSourceSpec::Raw` variant allows arbitrary values for monitors that deviate from
the standard.

### 10.6 No Polling

The service does not poll monitor states periodically. It enumerates monitors on startup and on explicit `RefreshMonitors` commands. Input source switches are
performed only on explicit
`SwitchInputSource` commands. This minimizes I2C bus traffic and avoids interfering with other DDC/CI tools.

### 10.7 GTK Widget Ownership

GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget references are captured in
`glib::clone!` closures or
`glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.

### 10.8 MCP Tool Naming

Tool names use `snake_case` with underscores, never dots. Dots in tool names cause schema validation failures in LLM gateways.

### 10.9 FFI String Types

All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and `stabby::option::Option<stabby::string::String>`
respectively, to maintain ABI stability across compiler invocations.

### 10.10 ddcutil Compatibility

The `ddc` + `ddc-i2c` Rust crates provide the same DDC/CI functionality as the `ddcutil` command-line tool. The VCP feature `0x60` (Input Source Select) is the
same feature used by
`ddcutil setvcp 0x60 <value>`. The Rust crates communicate directly via I2C, avoiding the overhead of spawning a shell process.

---

## 11. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/ddc`, `services/ddc`, and `plugins/ddc`.
- **One struct per file:** Each message struct and enum lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, and `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String` and optional strings use
  `stabby::option::Option<stabby::string::String>`.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `ddc`, `ddc-i2c`, `tokio`, and `tracing`; the widget uses `gtk4` and `glib`.
- **Atomic widget pattern:** The widget follows the atomic widget pattern established by the Weather Atomic Widgets, using `AtomicWidgetConfig` and
  `widget_factory_plugin_graphic!`.
- **Rust library over shell commands:** The service uses the `ddc` and `ddc-i2c` Rust crates for DDC/CI communication instead of spawning `ddcutil` shell
  commands.

---

*End of document.*
