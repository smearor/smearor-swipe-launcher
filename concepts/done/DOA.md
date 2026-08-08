# Concept: DoA Service & Widget (ReSpeaker XVF3800)

This document describes the concept for a **Direction of Arrival (DoA) Service** and **DoA Widget** that reads the voice incidence direction from a **ReSpeaker
XVF3800 USB 4-Mic Array** (XMOS XVF3800 DSP) and displays it in the Smearor Swipe Launcher. All components follow the decoupled architecture of the project.

---

## 1. Motivation

The ReSpeaker XVF3800 is a USB 4-microphone array with an integrated XMOS XVF3800 DSP. It transmits audio via USB Audio Class (UAC) and exposes control/sensor
parameters — including the **Direction of Arrival (DoA)** angle (0°–359°) — through USB Control Transfers (Vendor Requests on Endpoint 0).

Knowing the direction of the sound source relative to the microphone array enables:

- **Table-side mapping**: When the array is placed centrally on or at a table, the 0°–359° angle maps to four table sides (North, East, South, West), enabling
  automatic menu orientation or audio focus toward the active speaker.
- **Voice Assistant integration**: The Voice Assistant service can use the DoA angle to determine which direction the user is speaking from, enabling spatial
  awareness and directional audio focusing.
- **Visual feedback**: A compass-style widget shows the current sound direction in real time, giving the user immediate feedback about the microphone array's
  active detection direction.

The DoA reading is performed via USB Vendor Requests using the `rusb` crate (a type-safe wrapper around `libusb`). No D-Bus or HID API is involved — the XVF3800
exposes its parameters exclusively through USB Control Transfers.

---

## 2. Crate Structure

| Crate       | Path            | Responsibility                                             |
|-------------|-----------------|------------------------------------------------------------|
| **Model**   | `model/doa/`    | Shared structs, enums, message formats, FFI types          |
| **Service** | `services/doa/` | USB integration via `rusb`, DoA polling, status broadcasts |
| **Widget**  | `plugins/doa/`  | GTK4 tile widget with compass view and direction display   |

---

## 3. Model Crate (`model/doa`)

### 3.1 Message Topics

```rust
pub const TOPIC_STATUS: &str = "service.doa.status";
pub const TOPIC_COMMAND: &str = "service.doa.command";
```

### 3.2 DoA Status Message

```rust
/// Status message for the DoA sensor, broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DoaStatusMessage {
    /// Whether the ReSpeaker XVF3800 device is connected and active.
    pub connected: bool,
    /// Current DoA angle in degrees (0-359). Raw angle from the DSP, before rotation offset.
    pub angle: u16,
    /// Calibrated angle after applying `rotation_offset` from service config (0-359).
    /// This is the angle relative to the table's physical orientation.
    pub calibrated_angle: u16,
    /// Mapped table side based on `calibrated_angle`. Derived via `DoaDirection::from_angle`.
    pub direction: DoaDirection,
    /// Whether speech/voice activity is currently detected by the DSP.
    /// When `false`, the `angle` and `calibrated_angle` fields represent the
    /// last detected direction (held in the register during silence).
    /// When `true`, active speech is coming from the indicated direction.
    pub speech_detected: bool,
    /// Vendor ID of the connected device (0x2886 = Seeed Studio, 0x20b1 = XMOS).
    pub vendor_id: u16,
    /// Product ID of the connected device.
    pub product_id: u16,
    /// Timestamp of the last DoA reading (ISO 8601).
    pub last_updated: StabbyString,
}
```

### 3.3 DoA Direction Enum

```rust
/// Compass direction derived from the DoA angle.
/// Default quadrant mapping (after calibration):
/// - 315°–45° → North (front)
/// - 45°–135° → East (right)
/// - 135°–225° → South (back)
/// - 225°–315° → West (left)
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum DoaDirection {
    /// North side of the table (315°–45°). Default front of the mic array.
    #[default]
    North,
    /// East side of the table (45°–135°).
    East,
    /// South side of the table (135°–225°).
    South,
    /// West side of the table (225°–315°).
    West,
}

impl DoaDirection {
    /// Maps a calibrated DoA angle (0-359) to a compass direction.
    /// Uses the default 45°-offset quadrant mapping.
    pub fn from_angle(angle: u16) -> Self {
        let angle = angle % 360;
        if angle >= 315 || angle < 45 {
            Self::North
        } else if angle < 135 {
            Self::East
        } else if angle < 225 {
            Self::South
        } else {
            Self::West
        }
    }

    /// Maps a raw DoA angle to a compass direction, applying a rotation offset.
    /// The offset compensates for the physical mounting orientation of the
    /// microphone array relative to the table's reference direction (North).
    /// Positive offsets rotate clockwise, negative offsets counter-clockwise.
    /// For example, if the DSP's 0° axis points 90° clockwise from the table's
    /// North, set `offset = -90` (or equivalently `offset = 270`).
    pub fn from_angle_with_offset(raw_angle: u16, offset: i16) -> Self {
        let calibrated_angle = (raw_angle as i16 + offset).rem_euclid(360) as u16;
        Self::from_angle(calibrated_angle)
    }

    /// Returns a human-readable label key for the direction.
    pub fn label_key(&self) -> &'static str {
        match self {
            Self::North => "doa_direction_north",
            Self::East => "doa_direction_east",
            Self::South => "doa_direction_south",
            Self::West => "doa_direction_west",
        }
    }
}
```

### 3.4 Command Message

```rust
/// Commands sent to the DoA service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DoaCommandMessage {
    /// The action to perform.
    pub action: DoaCommandAction,
    /// Target state for the action. Semantics depend on `action`:
    /// - `SetPollInterval`: new interval in milliseconds.
    pub value: u64,
}

/// Available DoA command actions.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum DoaCommandAction {
    /// Restart the USB connection and resume polling.
    #[default]
    Reconnect,
    /// Set the polling interval in milliseconds. `value` = new interval.
    SetPollInterval,
    /// Pause DoA polling (stop reading from the device).
    Pause,
    /// Resume DoA polling (continue reading from the device).
    Resume,
}
```

### 3.5 View Enum (Widget)

```rust
/// Views available in the DoA widget for tile rotation.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum DoaView {
    /// Compass view: shows the current angle as a compass needle.
    #[default]
    Compass,
    /// Direction view: shows the mapped table side (N/E/S/W) as text + icon.
    Direction,
    /// Device info view: shows connection status, vendor/product ID, speech activity.
    DeviceInfo,
}
```

### 3.6 MCP Tools Enum

```rust
use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the DoA service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoaMcpTools {
    /// Returns the current DoA angle, mapped direction, and connection status.
    GetDirection,
    /// Sets the DoA polling interval in milliseconds.
    SetPollInterval,
    /// Forces a USB reconnection to the ReSpeaker device.
    Reconnect,
}

impl AsRef<str> for DoaMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::GetDirection => "doa_get_direction",
            Self::SetPollInterval => "doa_set_poll_interval",
            Self::Reconnect => "doa_reconnect",
        }
    }
}

impl FromStr for DoaMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "doa_get_direction" => Ok(Self::GetDirection),
            "doa_set_poll_interval" => Ok(Self::SetPollInterval),
            "doa_reconnect" => Ok(Self::Reconnect),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for DoaMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
```

### 3.7 JSON Converters

All FFI-relevant message types must register JSON converters in `lib.rs` using the `impl_json_convertible!` macro. Manual `parse_*` functions are forbidden.

```rust
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(DoaStatusMessageConverter, DoaStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(DoaCommandMessageConverter, DoaCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for DoA messages.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    DoaStatusMessageConverter::register_in_host(context);
    DoaCommandMessageConverter::register_in_host(context);
}
```

### 3.7 Model Crate `Cargo.toml`

```toml
[package]
name = "smearor-doa-model"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
smearor-model-mcp = { path = "../mcp" }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
stabby = { workspace = true, features = ["serde"] }
```

### 3.8 Model Crate `lib.rs`

```rust
mod direction;
mod messages;
mod mcp_tools;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use direction::DoaDirection;
pub use messages::command::DoaCommandAction;
pub use messages::command::DoaCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::status::DoaStatusMessage;
pub use messages::status::TOPIC_STATUS;
pub use messages::view::DoaView;
pub use mcp_tools::DoaMcpTools;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DoaStatusMessageConverter, DoaStatusMessage, |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default());
smearor_swipe_launcher_plugin_api::impl_json_convertible!(DoaCommandMessageConverter, DoaCommandMessage, |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default());

/// Register all JSON converter implementations for DoA messages.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    DoaStatusMessageConverter::register_in_host(context);
    DoaCommandMessageConverter::register_in_host(context);
}
```

---

## 4. Service Crate (`services/doa`)

### 4.1 USB Integration

The service communicates with the ReSpeaker XVF3800 via USB Control Transfers using the `rusb` crate. Key USB parameters:

| Parameter           | Value    | Description                                   |
|---------------------|----------|-----------------------------------------------|
| Vendor ID (Seeed)   | `0x2886` | Seeed Studio Vendor ID                        |
| Vendor ID (XMOS)    | `0x20b1` | XMOS Vendor ID                                |
| Request Type (Read) | `0xC0`   | USB Flags: Vendor \| DeviceToHost \| Device   |
| bRequest (Read)     | `0x00`   | XMOS Parameter Request Code                   |
| DoA Angle Register  | `0x0015` | Register ID for the DoA angle                 |
| VAD Register        | `0x0016` | Register ID for Voice Activity Detection flag |
| Response Format     | `u16 LE` | 16-bit little-endian integer, 0–359 degrees   |

The service opens a `rusb::DeviceHandle`, performs periodic `read_control` transfers to read the DoA register, and broadcasts the angle as a `DoaStatusMessage`.

### 4.2 Service Struct

```rust
pub struct DoaService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: DoaServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<DoaCommandMessage>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<DoaCommandMessage>>,
    pub shared_state: Arc<Mutex<DoaSharedState>>,
}

/// Shared state between the async control loop and the MCP tool handler.
/// Updated only by the async loop — never accessed from the USB reader thread.
pub struct DoaSharedState {
    pub connected: bool,
    pub angle: u16,
    pub calibrated_angle: u16,
    pub speech_detected: bool,
    pub vendor_id: u16,
    pub product_id: u16,
    pub last_updated: String,
}

/// Result of a single DoA USB read, sent from the USB reader thread to the async loop.
enum DoaReading {
    /// Successful DoA read with angle and speech activity flag.
    Reading { angle: u16, speech_detected: bool, vendor_id: u16, product_id: u16 },
    /// USB read failed or device was lost.
    Disconnected,
}

/// Control commands forwarded from the async loop to the USB reader thread.
enum UsbControl {
    /// Pause polling (stop reading from the device, keep handle open).
    Pause,
    /// Resume polling.
    Resume,
    /// Close current handle and attempt reconnection.
    Reconnect,
    /// Change the polling interval.
    SetInterval(u64),
}
```

The service struct must implement:

- `ServicePlugin` — provides `on_message` and `start`
- `MessageHandler<FfiEnvelopePayload<DoaCommandMessage>>` — dispatches commands
- `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` — handles MCP tool invocations
- `MessageBroadcaster` — empty impl for broadcasting
- `MessageTopicBroadcaster<DoaStatusMessage>` — empty impl for typed broadcasting
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`

### 4.3 `lib.rs`

```rust
pub(crate) mod config;
pub(crate) mod service;

use crate::service::DoaService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(DoaService);
```

### 4.4 `start()` Method

The `start` method spawns a thread with `tokio::runtime::Builder::new_current_thread().enable_all()` + `LocalSet`:

```rust
fn start(&mut self) {
    if let Some(ctx) = &self.core_context {
        let meta = self.meta.clone();
        let core_context = *ctx;
        let command_receiver = self.command_receiver.take();
        let config = self.config.clone();
        let shared_state = self.shared_state.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
            // ... error handling ...
            let local_set = tokio::task::LocalSet::new();
            local_set.block_on(&rt, async move {
                if let Some(receiver) = command_receiver {
                    run_doa_async(meta, core_context, receiver, config, shared_state).await;
                }
            });
        });
    }
}
```

### 4.5 Architecture: Dedicated USB Reader Thread

**Problem**: `rusb::DeviceHandle::read_control` is a synchronous, blocking I/O call with a timeout of up to 500ms. Calling it directly inside a `tokio::select!`
interval tick would block the Tokio runtime for the duration of each USB transfer, preventing command processing during that window.

**Solution**: The service uses a **dedicated OS thread** for USB reads. This thread owns the `rusb::DeviceHandle`, performs blocking `read_control` transfers at
the configured interval, and sends results via a `tokio::sync::mpsc` channel to the async control loop. The async loop remains fully responsive to commands at
all times.

```
┌─────────────────────────┐       tokio::sync::mpsc        ┌──────────────────────────┐
│   USB Reader Thread      │  ──── DoaReading ──────────▶  │   Async Control Loop      │
│   (owns DeviceHandle)    │                                │   (tokio::select!)        │
│                          │  ◀── UsbControl ────────────  │                           │
│   blocking read_control  │       tokio::sync::mpsc        │   commands + broadcast    │
└─────────────────────────┘                                └──────────────────────────┘
```

### 4.6 USB Reader Thread

The USB reader thread owns the `rusb::DeviceHandle` and performs blocking reads in a loop. It receives control commands (pause, resume, reconnect, set interval)
from the async loop via a separate channel.

```rust
/// Runs on a dedicated OS thread. Owns the USB DeviceHandle and performs blocking reads.
fn usb_reader_loop(
    config: DoaServiceConfig,
    reading_sender: tokio::sync::mpsc::UnboundedSender<DoaReading>,
    mut control_receiver: tokio::sync::mpsc::UnboundedReceiver<UsbControl>,
) {
    let mut handle = open_respeaker(&config, None);
    let mut paused = false;
    let mut poll_interval_ms = config.poll_interval_ms;
    let mut consecutive_failures: u32 = 0;

    // Broadcast initial connection state
    let _ = reading_sender.send(initial_reading(&handle));

    loop {
        // Check for control commands with a short timeout so we don't block
        // longer than the poll interval.
        match control_receiver.recv_timeout(Duration::from_millis(poll_interval_ms)) {
            Ok(UsbControl::Pause) => {
                paused = true;
                debug!("DoA USB thread: paused");
            }
            Ok(UsbControl::Resume) => {
                paused = false;
                debug!("DoA USB thread: resumed");
            }
            Ok(UsbControl::SetInterval(ms)) => {
                poll_interval_ms = ms.max(50);
                debug!("DoA USB thread: interval set to {}ms", poll_interval_ms);
            }
            Ok(UsbControl::Reconnect) => {
                debug!("DoA USB thread: reconnecting (manual)...");
                // Drop the old handle explicitly before opening a new one
                let old_handle = handle.take();
                handle = open_respeaker(&config, old_handle);
                consecutive_failures = 0;
                let _ = reading_sender.send(initial_reading(&handle));
            }
            Err(tokio::sync::mpsc::error::RecvTimeoutError::Timeout) => {
                // No command received — proceed to USB read below
            }
            Err(tokio::sync::mpsc::error::RecvTimeoutError::Disconnected) => {
                debug!("DoA USB thread: control channel closed, exiting");
                // Explicitly drop the handle before returning to ensure
                // libusb releases USB interface claims and the device
                // is available for immediate reconnection if the service
                // is reloaded.
                drop(handle.take());
                return;
            }
        }

        if paused {
            continue;
        }

        match &handle {
            Some(device_handle) => {
                let transfer_timeout = usb_transfer_timeout(poll_interval_ms);
                match read_doa_angle(device_handle, transfer_timeout) {
                    Ok(angle) => {
                        // VAD read is non-critical: if it fails, fall back to
                        // `speech_detected = false` rather than triggering
                        // reconnection. The angle read succeeding proves the
                        // device is still reachable.
                        let speech_detected = read_speech_detected(device_handle, transfer_timeout)
                            .unwrap_or_else(|e| {
                                debug!("DoA USB thread: VAD read failed ({:?}), falling back to false", e);
                                false
                            });
                        let (vid, pid) = device_vid_pid(device_handle);
                        let _ = reading_sender.send(DoaReading::Reading {
                            angle,
                            speech_detected,
                            vendor_id: vid,
                            product_id: pid,
                        });
                        consecutive_failures = 0;
                    }
                    Err(e) => {
                        classify_and_handle_usb_error(
                            &e,
                            &mut consecutive_failures,
                            &config,
                            &mut handle,
                            &reading_sender,
                        );
                    }
                }
            }
            None => {
                // No device — attempt reconnection periodically
                std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
                handle = open_respeaker(&config, None);
                if handle.is_some() {
                    consecutive_failures = 0;
                }
                let _ = reading_sender.send(initial_reading(&handle));
            }
        }
    }
}

/// Classifies a USB error and handles reconnection with appropriate logging.
///
/// Error categories:
/// - **Physical disconnect** (`NoDevice`, `NotFound`, `Io`): device is gone.
///   Drop the handle explicitly, log once at `warn!`, then reconnect.
/// - **Transient** (`Busy`, `Timeout`, `Pipe`): keep the handle open, back off
///   with exponential delay, and suppress repeated log messages to avoid spam.
/// - **Unexpected**: log at `error!`, drop handle, and attempt full reconnection.
fn classify_and_handle_usb_error(
    error: &rusb::Error,
    consecutive_failures: &mut u32,
    config: &DoaServiceConfig,
    handle: &mut Option<DeviceHandle<Context>>,
    reading_sender: &tokio::sync::mpsc::UnboundedSender<DoaReading>,
) {
    *consecutive_failures += 1;
    match error {
        rusb::Error::NoDevice | rusb::Error::NotFound | rusb::Error::Io => {
            // Device is physically gone — drop the handle explicitly
            warn!("DoA USB thread: device disconnected ({:?}), dropping handle", error);
            let old = handle.take();
            *handle = open_respeaker(config, old);
            let _ = reading_sender.send(initial_reading(handle));
            if handle.is_some() {
                *consecutive_failures = 0;
            } else {
                // No device found — wait before next attempt to avoid busy-looping
                std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
            }
        }
        rusb::Error::Busy | rusb::Error::Timeout | rusb::Error::Pipe => {
            // Transient error — keep handle, back off to avoid log spam
            let backoff_ms = config
                .reconnect_delay_ms
                .saturating_mul((*consecutive_failures).min(10) as u64);
            if *consecutive_failures <= 3 || *consecutive_failures % 10 == 0 {
                debug!(
                    "DoA USB thread: transient error ({:?}), retrying after {}ms (attempt {})",
                    error, backoff_ms, consecutive_failures
                );
            }
            std::thread::sleep(Duration::from_millis(backoff_ms));
        }
        other => {
            // Unexpected error — log and attempt full reconnection
            error!("DoA USB thread: unexpected USB error ({:?}), reconnecting", other);
            let old = handle.take();
            *handle = open_respeaker(config, old);
            let _ = reading_sender.send(initial_reading(handle));
            if handle.is_some() {
                *consecutive_failures = 0;
            } else {
                std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
            }
        }
    }
}

/// Produces an initial DoaReading reflecting the current connection state.
fn initial_reading(handle: &Option<DeviceHandle<Context>>) -> DoaReading {
    match handle {
        Some(h) => {
            let (vid, pid) = device_vid_pid(h);
            DoaReading::Reading { angle: 0, speech_detected: false, vendor_id: vid, product_id: pid }
        }
        None => DoaReading::Disconnected,
    }
}
```

### 4.7 Async Control Loop

The async loop uses `tokio::select!` with two channels:

- **Command channel** (`DoaCommandMessage`) — commands from the launcher/voice assistant
- **Reading channel** (`DoaReading`) — DoA readings from the USB reader thread

The async loop never performs blocking I/O. It only handles messages, updates shared state, and broadcasts status.

```rust
async fn run_doa_async(
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<DoaCommandMessage>,
    config: DoaServiceConfig,
    shared_state: Arc<Mutex<DoaSharedState>>,
) {
    // Channel for DoA readings from the USB thread
    let (reading_sender, mut reading_receiver) = tokio::sync::mpsc::unbounded_channel::<DoaReading>();
    // Channel for control commands to the USB thread
    let (usb_control_sender, usb_control_receiver) = tokio::sync::mpsc::unbounded_channel::<UsbControl>();

    // Spawn the dedicated USB reader thread
    std::thread::spawn(move || {
        usb_reader_loop(config, reading_sender, usb_control_receiver);
    });

    loop {
        tokio::select! {
            command = command_receiver.recv() => {
                match command {
                    Some(cmd) => handle_command(&cmd, &shared_state, &usb_control_sender),
                    None => {
                        // Command channel closed — service is shutting down.
                        // Dropping usb_control_sender closes the channel, causing
                        // the USB reader thread to exit via RecvTimeoutError::Disconnected.
                        debug!("DoA async loop: command channel closed, shutting down");
                        break;
                    }
                }
            }
            Some(reading) = reading_receiver.recv() => {
                match reading {
                    DoaReading::Reading { angle, speech_detected, vendor_id, product_id } => {
                        let calibrated_angle = (angle as i16 + config.rotation_offset).rem_euclid(360) as u16;
                        let direction = DoaDirection::from_angle(calibrated_angle);
                        let timestamp = current_timestamp();
                        {
                            let mut state = shared_state.lock();
                            state.connected = true;
                            state.angle = angle;
                            state.calibrated_angle = calibrated_angle;
                            state.speech_detected = speech_detected;
                            state.vendor_id = vendor_id;
                            state.product_id = product_id;
                            state.last_updated = timestamp.clone();
                        }
                        let status = DoaStatusMessage {
                            connected: true,
                            angle,
                            calibrated_angle,
                            direction,
                            speech_detected,
                            vendor_id,
                            product_id,
                            last_updated: StabbyString::from(timestamp),
                        };
                        broadcast_status(&meta, &core_context, status);
                    }
                    DoaReading::Disconnected => {
                        {
                            let mut state = shared_state.lock();
                            state.connected = false;
                            state.last_updated = current_timestamp();
                        }
                        let status = DoaStatusMessage {
                            connected: false,
                            ..Default::default()
                        };
                        broadcast_status(&meta, &core_context, status);
                    }
                }
            }
        }
    }
}

/// Handles a DoaCommandMessage by updating shared state and forwarding to the USB thread.
fn handle_command(
    command: &DoaCommandMessage,
    shared_state: &Arc<Mutex<DoaSharedState>>,
    usb_control: &tokio::sync::mpsc::UnboundedSender<UsbControl>,
) {
    match command.action {
        DoaCommandAction::Reconnect => {
            let _ = usb_control.send(UsbControl::Reconnect);
        }
        DoaCommandAction::Pause => {
            let _ = usb_control.send(UsbControl::Pause);
        }
        DoaCommandAction::Resume => {
            let _ = usb_control.send(UsbControl::Resume);
        }
        DoaCommandAction::SetPollInterval => {
            let interval = command.value.max(50);
            let _ = usb_control.send(UsbControl::SetInterval(interval));
        }
    }
}
```

**Mutex discipline**: The `shared_state` mutex is acquired only briefly in the async loop for writing the updated angle/connection state. The USB reader thread
never touches `shared_state` — it communicates exclusively via channels. Status broadcasting happens outside the lock.

**Why not `spawn_blocking`?** Using `tokio::task::spawn_blocking` per interval tick would require moving the `rusb::DeviceHandle` (which is `Send` but not
`Clone`) into and out of each blocking task. A dedicated thread that owns the handle for its entire lifetime is simpler, avoids repeated handle creation, and
allows the USB thread to maintain its own polling loop independently of the async runtime's scheduling.

### 4.7.1 Graceful Shutdown

The shutdown cascade ensures the USB reader thread is terminated when the service is unloaded:

```
ServiceManager::unload_service()
  │
  ├── service.destroy()          (FFI destroy function)
  │
  └── LoadedService dropped
        │
        ├── DoaService::Drop  →  command_sender dropped
        │                          │
        │                          ▼
        │                     command_receiver.recv() returns None
        │                          │
        │                          ▼
        │                     async loop breaks  →  usb_control_sender dropped
        │                                               │
        │                                               ▼
        │                                          control_receiver.recv_timeout()
        │                                          returns Disconnected
        │                                               │
        │                                               ▼
        │                                          drop(handle.take())
        │                                          releases USB interface claims
        │                                               │
        │                                               ▼
        │                                          USB reader thread exits
        │
        └── std::mem::forget(service)  (prevents .so unloading before thread exits)
```

The `DoaService` struct must implement `Drop` to trigger this cascade:

```rust
impl Drop for DoaService {
    fn drop(&mut self) {
        debug!("DoA Service: dropping, command_sender will be released");
        // command_sender is dropped here, closing the channel.
        // The async loop's command_receiver.recv() returns None,
        // causing the loop to break and drop usb_control_sender.
        // The USB reader thread then exits via RecvTimeoutError::Disconnected,
        // explicitly dropping the DeviceHandle before returning to ensure
        // libusb releases USB interface claims.
        //
        // Note: ServiceManager calls std::mem::forget(service) after destroy()
        // to prevent the .so from being unloaded before the USB thread exits.
        // The thread exits asynchronously — there is no JoinHandle to wait on.
        // This is safe because std::process::exit(0) follows shortly after.
    }
}
```

**Important**: The `ServiceManager` calls `std::mem::forget(service)` after `destroy()` to prevent the `.so` library from being unloaded while the USB reader
thread is still executing code from it. The thread exits asynchronously after the `usb_control_sender` is dropped. This mirrors the pattern used by all other
services in the codebase (see `service_manager.rs:52-57`).

### 4.8 USB Device Discovery and Reading

```rust
/// USB Vendor IDs for ReSpeaker / XMOS devices.
const VENDOR_ID_SEEED: u16 = 0x2886;
const VENDOR_ID_XMOS: u16 = 0x20b1;

/// USB Control Transfer parameters for XVF3800.
const REQUEST_TYPE_READ: u8 = 0xC0;
const B_REQUEST_READ: u8 = 0x00;
const PARAM_DOA_ANGLE: u16 = 0x0015;
const PARAM_VAD: u16 = 0x0016;

/// Searches for a ReSpeaker/XMOS USB device and opens a DeviceHandle.
/// The `old_handle` parameter ensures the previous handle is dropped before
/// opening a new one, releasing USB interface claims and letting libusb clean up.
fn open_respeaker(config: &DoaServiceConfig, old_handle: Option<DeviceHandle<Context>>) -> Option<DeviceHandle<Context>> {
    // Explicitly drop the old handle first to release USB interface claims
    // and ensure libusb cleans up before we open a new device.
    drop(old_handle);
    let context = Context::new().ok()?;
    for device in context.devices().ok()?.iter() {
        let device_desc = device.device_descriptor().ok()?;
        let vid = device_desc.vendor_id();
        if vid == VENDOR_ID_SEEED || vid == VENDOR_ID_XMOS {
            // Optionally filter by product ID if configured
            if let Some(pid) = config.product_id {
                if device_desc.product_id() != pid {
                    continue;
                }
            }
            if let Ok(handle) = device.open() {
                debug!("DoA service: connected to USB device VID={:#06x} PID={:#06x}", vid, device_desc.product_id());
                return Some(handle);
            }
        }
    }
    trace!("DoA service: no ReSpeaker XVF3800 USB device found");
    None
}

/// Reads the DoA angle (0-359 degrees) via a USB Control Transfer.
/// The timeout is derived from the current poll interval to ensure that
/// a single stalled transfer cannot block the USB thread longer than
/// one poll cycle. See `usb_transfer_timeout`.
fn read_doa_angle(handle: &DeviceHandle<Context>, timeout: Duration) -> Result<u16, rusb::Error> {
    let mut buffer = [0u8; 8];
    let bytes_read = handle.read_control(
        REQUEST_TYPE_READ,
        B_REQUEST_READ,
        PARAM_DOA_ANGLE,
        0x0000,
        &mut buffer,
        timeout,
    )?;
    if bytes_read >= 2 {
        let raw_angle = u16::from_le_bytes([buffer[0], buffer[1]]);
        Ok(raw_angle % 360)
    } else {
        Err(rusb::Error::InvalidParam)
    }
}

/// Reads the Voice Activity Detection (VAD) flag via a USB Control Transfer.
/// Returns `true` when the DSP detects active speech, `false` during silence.
/// When `false`, the DoA angle register holds the last detected direction.
/// The timeout is derived from the current poll interval (see `usb_transfer_timeout`).
fn read_speech_detected(handle: &DeviceHandle<Context>, timeout: Duration) -> Result<bool, rusb::Error> {
    let mut buffer = [0u8; 8];
    let bytes_read = handle.read_control(
        REQUEST_TYPE_READ,
        B_REQUEST_READ,
        PARAM_VAD,
        0x0000,
        &mut buffer,
        timeout,
    )?;
    if bytes_read >= 1 {
        Ok(buffer[0] != 0)
    } else {
        Err(rusb::Error::InvalidParam)
    }
}

/// Extracts vendor ID and product ID from a DeviceHandle's device descriptor.
fn device_vid_pid(handle: &DeviceHandle<Context>) -> (u16, u16) {
    match handle.device().device_descriptor() {
        Ok(desc) => (desc.vendor_id(), desc.product_id()),
        Err(_) => (0, 0),
    }
}

/// Computes the USB Control Transfer timeout from the current poll interval.
/// Set to half the poll interval, clamped to [20, 100] ms, so that two
/// consecutive stalled transfers cannot block the USB thread longer than
/// one poll cycle — even when `poll_interval_ms` is reduced to the 50 ms minimum.
fn usb_transfer_timeout(poll_interval_ms: u64) -> Duration {
    Duration::from_millis((poll_interval_ms / 2).clamp(20, 100))
}
```

**Note on block reads**: The DoA angle register (`0x0015`) and VAD register (`0x0016`) are adjacent. If the XVF3800 firmware supports contiguous multi-register
reads, a single `read_control` with a larger buffer could fetch both values in one transfer. This would halve USB traffic and eliminate the partial-failure edge
case entirely. For now, two separate reads are used with the VAD read treated as non-critical (see `usb_reader_loop`).

### 4.9 Service Config

```rust
/// Configuration for the DoA service.
#[derive(Debug, Clone, Deserialize)]
pub struct DoaServiceConfig {
    /// Polling interval for DoA reads in milliseconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    /// Whether to enable MCP tool registration for this service.
    #[serde(default = "default_mcp_enabled")]
    pub mcp_enabled: bool,
    /// Optional product ID filter. If set, only devices with this PID are matched.
    #[serde(default)]
    pub product_id: Option<u16>,
    /// Reconnection delay in milliseconds when the USB device is lost.
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_ms: u64,
    /// Rotation offset in degrees (-360 to 360) to calibrate the DoA angle to the
    /// physical table orientation. The raw DSP angle is rotated by this offset
    /// before mapping to a compass direction. Positive values rotate clockwise,
    /// negative values counter-clockwise. Values outside ±360 are wrapped via
    /// `rem_euclid(360)`. Use this when the microphone array's 0° axis does not
    /// align with the table's North/reference direction.
    /// Example: if the DSP 0° points 90° clockwise from table North, set offset = -90.
    #[serde(default = "default_rotation_offset")]
    pub rotation_offset: i16,
}

impl DoaServiceConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

fn default_poll_interval() -> u64 { 150 }
fn default_mcp_enabled() -> bool { true }
fn default_reconnect_delay() -> u64 { 2000 }
fn default_rotation_offset() -> i16 { 0 }
```

Use `#[serde(default)]` and `#[serde(default = "fn_name")]` for all fields so partial TOML configs work.

### 4.10 MCP Tools

If the service exposes MCP tools, implement `McpCapabilitiesRegistrator` and register tools during `start()`. Handle `InvokeToolMessage` via `MessageHandler`.

#### Registered MCP Resource

```rust
let doa_resource = RegisterResourceMessage::new(
"doa://status",
"DoA Sensor Status",
"Current Direction of Arrival angle, mapped direction, and device connection status.",
"application/json",
);
broadcaster.broadcast_message_to_topic(doa_resource);
```

#### Registered MCP Tools

```rust
let get_direction_tool = RegisterToolMessage::new(
"doa_get_direction",
"Returns the current DoA angle (0-359), mapped compass direction (N/E/S/W), and device connection status.",
r#"{ "type": "object", "properties": {}, "required": [] }"#,
);
broadcaster.broadcast_message_to_topic(get_direction_tool);

let set_poll_interval_tool = RegisterToolMessage::new(
"doa_set_poll_interval",
"Sets the DoA polling interval in milliseconds. Lower values give more responsive direction updates but increase USB traffic. Minimum: 50ms.",
r#"{ "type": "object", "properties": { "interval_ms": { "type": "integer", "description": "Polling interval in milliseconds (min: 50, default: 150)" } }, "required": ["interval_ms"] }"#,
);
broadcaster.broadcast_message_to_topic(set_poll_interval_tool);

let reconnect_tool = RegisterToolMessage::new(
"doa_reconnect",
"Forces a USB reconnection to the ReSpeaker XVF3800 device. Use this if the device was unplugged and reconnected.",
r#"{ "type": "object", "properties": {}, "required": [] }"#,
);
broadcaster.broadcast_message_to_topic(reconnect_tool);
```

#### InvokeToolMessage Handler

```rust
impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for DoaService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("DoA Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match DoaMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id)));
                return;
            }
        };
        match tool {
            DoaMcpTools::GetDirection => {
                let state = self.shared_state.lock();
                let json = serde_json::json!({
                    "connected": state.connected,
                    "angle": state.angle,
                    "calibrated_angle": state.calibrated_angle,
                    "rotation_offset": self.config.rotation_offset,
                    "direction": format!("{:?}", DoaDirection::from_angle(state.calibrated_angle)),
                    "speech_detected": state.speech_detected,
                    "vendor_id": format!("{:#06x}", state.vendor_id),
                    "product_id": format!("{:#06x}", state.product_id),
                    "last_updated": state.last_updated,
                });
                let response = InvokeToolResponse::success(&correlation_id, &json.to_string());
                broadcaster.broadcast_message_to_topic(response);
            }
            DoaMcpTools::SetPollInterval => {
                let args_result = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string());
                match args_result {
                    Ok(args) => {
                        let interval = args.get("interval_ms").and_then(|v| v.as_u64()).unwrap_or(150).max(50);
                        let cmd = DoaCommandMessage { action: DoaCommandAction::SetPollInterval, value: interval };
                        let _ = self.command_sender.send(cmd);
                        let response = InvokeToolResponse::success(&correlation_id, &format!("Poll interval set to {}ms", interval));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    Err(parse_error) => {
                        debug!("DoA Service: doa_set_poll_interval argument parse error: {parse_error}");
                        let response = InvokeToolResponse::error(
                            &correlation_id,
                            &format!("Invalid arguments: {parse_error}"),
                        );
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            DoaMcpTools::Reconnect => {
                let cmd = DoaCommandMessage { action: DoaCommandAction::Reconnect, value: 0 };
                let _ = self.command_sender.send(cmd);
                let response = InvokeToolResponse::success(&correlation_id, "Reconnection initiated");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
```

### 4.11 Service Crate `Cargo.toml`

```toml
[package]
name = "smearor-doa-service"
version = "0.1.0"
description = "Direction of Arrival service plugin for the ReSpeaker XVF3800 USB 4-Mic Array"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
rusb = "0.9"
stabby = { workspace = true }
glib = { workspace = true }
miette = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
smearor-doa-model = { path = "../../model/doa" }
smearor-model-mcp = { path = "../../model/mcp" }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

[package.metadata.deb]
name = "smearor-service-doa"
depends = "$auto, smearor-swipe-launcher (>= 0.1.0), libusb-1.0-0"
maintainer-scripts = "debian"
assets = [
    ["target/release/libsmearor_doa_service.so", "/usr/lib/smearor/", "644"],
    ["../../resources/udev/52-respeaker.rules", "/usr/lib/udev/rules.d/", "644"],
]
```

---

## 5. Widget Crate (`plugins/doa`)

### 5.1 Widget Struct

The widget struct must implement:

- `WidgetPlugin` (extends `PluginMetaGetter` + `WidgetBuilder`) — provides `on_message` and `start`
- `MessageHandler<FfiEnvelopePayload<DoaStatusMessage>>` — handles status updates
- `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` — handles locale
- `MessageBroadcaster` — for broadcasting commands
- `MessageTopicBroadcaster<DoaCommandMessage>` — for typed command broadcasting
- `MessageTopicBroadcaster<WidgetUpdateMessage>` — for headless/Web instance sync
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`
- `AcceptTopic<FfiEnvelope>` — filters relevant topics in `on_message`
- `GestureHandler` — provides `attach_gesture_handlers` and `DefaultFallback`
- `GraphicRenderer` — for headless instance pixel rendering
- `WebRenderer` — for web instance HTML rendering

Use `Rc<RefCell<...>>` for interior mutability and `glib::clone!` for closure ownership.

### 5.2 `lib.rs`

```rust
pub mod config;
pub mod graphic;
pub mod widget;

use crate::widget::DoaWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin_graphic;

widget_plugin_graphic!(DoaWidget);
```

### 5.3 Widget Config

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DoaWidgetConfig {
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    #[serde(flatten)]
    pub layout: WidgetLayout,
    #[serde(flatten)]
    pub icon: WidgetIcon,
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    #[serde(flatten)]
    pub mode: WidgetMode,
    #[serde(flatten)]
    pub actions: ActionBindings,
    #[serde(flatten)]
    pub icons: DoaIcons,
    pub views: Vec<DoaView>,
}

/// Feature-specific icons for the DoA widget.
#[derive(Debug, Clone, Deserialize)]
pub struct DoaIcons {
    #[serde(default = "default_icon_compass")]
    pub icon_compass: String,
    #[serde(default = "default_icon_north")]
    pub icon_north: String,
    #[serde(default = "default_icon_east")]
    pub icon_east: String,
    #[serde(default = "default_icon_south")]
    pub icon_south: String,
    #[serde(default = "default_icon_west")]
    pub icon_west: String,
    #[serde(default = "default_icon_disconnected")]
    pub icon_disconnected: String,
}

fn default_icon_compass() -> String { "nf-md-compass".to_string() }
fn default_icon_north() -> String { "nf-md-compass_north".to_string() }
fn default_icon_east() -> String { "nf-md-compass_east".to_string() }
fn default_icon_south() -> String { "nf-md-compass_south".to_string() }
fn default_icon_west() -> String { "nf-md-compass_west".to_string() }
fn default_icon_disconnected() -> String { "nf-md-compass_off".to_string() }
```

### 5.4 View Rendering

`render_view` returns a `ViewData` struct:

```rust
fn render_view(
    view: DoaView,
    status: &DoaStatusMessage,
    config: &DoaWidgetConfig,
    labels: &DoaLabel,
) -> ViewData {
    match view {
        DoaView::Compass => {
            if !status.connected {
                return ViewData::new(&config.icons.icon_disconnected, &labels.disconnected, "")
                    .with_error(true);
            }
            let icon = match status.direction {
                DoaDirection::North => &config.icons.icon_north,
                DoaDirection::East => &config.icons.icon_east,
                DoaDirection::South => &config.icons.icon_south,
                DoaDirection::West => &config.icons.icon_west,
            };
            ViewData::new(icon, &format!("{}°", status.calibrated_angle), &labels.direction_label(status.direction))
        }
        DoaView::Direction => {
            if !status.connected {
                return ViewData::new(&config.icons.icon_disconnected, &labels.disconnected, "")
                    .with_error(true);
            }
            let icon = match status.direction {
                DoaDirection::North => &config.icons.icon_north,
                DoaDirection::East => &config.icons.icon_east,
                DoaDirection::South => &config.icons.icon_south,
                DoaDirection::West => &config.icons.icon_west,
            };
            let speech_info = if status.speech_detected { &labels.speaking } else { &labels.silent };
            ViewData::new(icon, &labels.direction_label(status.direction), &format!("{}° · {}", status.calibrated_angle, speech_info))
        }
        DoaView::DeviceInfo => {
            if !status.connected {
                return ViewData::new(&config.icons.icon_disconnected, &labels.disconnected, "")
                    .with_error(true);
            }
            let speech_info = if status.speech_detected { &labels.speaking } else { &labels.silent };
            let info = format!("VID:{:#06x} PID:{:#06x} · {}", status.vendor_id, status.product_id, speech_info);
            ViewData::new(&config.icons.icon_compass, &labels.connected, &info)
        }
    }
}
```

### 5.5 Icon Handling

- Define `DoaIcons` struct with `Default` impl
- Use `#[serde(flatten)]` to embed it in the widget config
- Icon names are Nerd Font names (e.g. `nf-md-compass`, `nf-md-compass_north`)
- GTK resolves icons via `resolve_gtk_nerd_icon()` → GResource SVG paths
- Pixel/atomic rendering resolves via `resolve_icon_codepoint()` → Unicode codepoints
- For state-dependent icons, select the icon in `render_view` based on status data
- For `StabbyOption` values, use explicit `match` statements, not `.map().unwrap_or()`

### 5.6 Unified 4-Line Layout

All GTK widgets use the same vertical structure:

| Line | Height      | Content            |
|------|-------------|--------------------|
| 0    | `icon_size` | Icon               |
| 1    | 20px        | `widget-main-text` |
| 2    | 16px        | `widget-info-text` |
| 3    | 16px        | spacer/bar         |

In Compact mode with `icon_only = true`, lines 1–3 are empty but retain height for alignment.

### 5.7 Gesture Handling

Use the centralized `attach_gesture_handlers` trait method:

```rust
widget_self.attach_gesture_handlers(
& button_widget,
& config.actions,
& broadcaster,
& GestureHandlersConfiguration::default (),
);
```

#### Default Fallback Table

| View         | Click Fallback        | Long-Press Fallback   |
|--------------|-----------------------|-----------------------|
| `Compass`    | Cycle to `Direction`  | Cycle to `DeviceInfo` |
| `Direction`  | Cycle to `DeviceInfo` | Cycle to `Compass`    |
| `DeviceInfo` | Cycle to `Compass`    | Cycle to `Compass`    |

Swipe up/down cycles through `config.views` via `next_view()`/`prev_view()`.

### 5.8 Instance Type Support

All three instance types (GTK, Headless, Web) share the same `render_view` function:

- **GTK** (`InstanceType::Gtk`): `WidgetBuilder::build_widget()` → `gtk4::Box` with icon, labels, gesture handlers. Icons via `resolve_gtk_nerd_icon()`.
- **Headless** (`InstanceType::Headless`): `GraphicRenderer::render_graphic(w, h)` → RGBA pixel buffer via `image` + `ab_glyph`. Icons via
  `resolve_icon_codepoint()`.
- **Web** (`InstanceType::Web`): `WebRenderer::render_html(instance_id, plugin_id)` → HTML fragment with inline styles.

After every UI update, broadcast `WidgetUpdateMessage` so headless/Web instances can re-render.

### 5.9 GTK Updates

Use `glib::MainContext::default().spawn_local` for all GTK updates from async message handlers. **Polling loops (`timeout_add_local`) are forbidden** — use
event-driven `recv().await` via `tokio::sync::mpsc`.

### 5.10 Personalization

Subscribe to `TOPIC_PERSONALIZATION_STATUS` for locale-aware labels. Implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` and
`AcceptTopic<FfiEnvelope>` filtering.

Define a `DoaLabel` struct with a `from_personalization(p: Option<&PersonalizationStatusMessage>)` constructor, analogous to `NetworkLabel`.

```rust
/// Locale-aware labels for the DoA widget.
pub struct DoaLabel {
    pub connected: String,
    pub disconnected: String,
    pub direction_north: String,
    pub direction_east: String,
    pub direction_south: String,
    pub direction_west: String,
    pub speaking: String,
    pub silent: String,
}

impl DoaLabel {
    pub fn from_personalization(p: Option<&PersonalizationStatusMessage>) -> Self {
        match p {
            Some(pers) if pers.locale.starts_with("de") => Self {
                connected: "Verbunden".to_string(),
                disconnected: "Getrennt".to_string(),
                direction_north: "Nord".to_string(),
                direction_east: "Ost".to_string(),
                direction_south: "Süd".to_string(),
                direction_west: "West".to_string(),
                speaking: "Spricht".to_string(),
                silent: "Stille".to_string(),
            },
            _ => Self {
                connected: "Connected".to_string(),
                disconnected: "Disconnected".to_string(),
                direction_north: "North".to_string(),
                direction_east: "East".to_string(),
                direction_south: "South".to_string(),
                direction_west: "West".to_string(),
                speaking: "Speaking".to_string(),
                silent: "Silent".to_string(),
            },
        }
    }

    pub fn direction_label(&self, direction: DoaDirection) -> String {
        match direction {
            DoaDirection::North => self.direction_north.clone(),
            DoaDirection::East => self.direction_east.clone(),
            DoaDirection::South => self.direction_south.clone(),
            DoaDirection::West => self.direction_west.clone(),
        }
    }
}
```

### 5.11 Widget Crate `Cargo.toml`

```toml
[package]
name = "smearor-doa-widget"
version = "0.1.0"
description = "DoA compass widget for the Smearor Swipe Launcher"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
gtk4 = { workspace = true }
glib = { workspace = true }
image = { workspace = true }
ab_glyph = { workspace = true }
stabby = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
smearor-doa-model = { path = "../../model/doa" }
smearor-model-personalization = { path = "../../model/personalization" }
smearor-model-widget = { path = "../../model/widget" }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
tracing = { workspace = true }
```

---

## 6. Cross-Service Coordination

### 6.1 Voice Assistant Integration

The Voice Assistant service can subscribe to `TOPIC_STATUS` (`service.doa.status`) to receive DoA angle and VAD updates. This enables:

- **Spatial awareness**: The voice assistant knows which direction the user is speaking from.
- **VAD-gated direction**: The assistant can use `speech_detected` to distinguish active speech from held-angle silence, avoiding stale direction data when no
  one is speaking.
- **Directional context**: The assistant can mention the direction in responses (e.g., "I hear you from the North side").
- **Audio focus**: Future integration with the Audio service could use DoA to steer beamforming toward the active speaker.

**Direction**: One-directional (DoA Service broadcasts → Voice Assistant subscribes).

**Message format**: `DoaStatusMessage` on `service.doa.status`.

### 6.2 VAD-Triggered Listening Mode (Hardware-VAD Activation)

The Voice Assistant uses the `speech_detected` flag in `DoaStatusMessage` as an event-driven trigger for entering and exiting Listening Mode. This leverages the
XMOS XVF3800 DSP's dedicated hardware VAD, which reacts without CPU overhead on the host and delivers the signal before audio streams need to be processed.

#### State Transitions (Edge Detection)

The Voice Assistant implements edge detection on the `speech_detected` flag:

- **Rising Edge (`false → true`)**: Immediately activates Listening Mode — microphone enablement / audio streaming to STT. Attaches the captured spatial data
  (`calibrated_angle` and `direction`) as context metadata to the voice session. Depending on configuration, the computationally expensive software wake-word
  detection can be skipped or used as an additional confirmation criterion ("Barge-In").
- **Falling Edge (`true → false`)**: Starts a holdover timer (Grace Period, recommended: 300–500 ms, configurable). Prevents premature termination of Listening
  Mode during natural speech pauses. The recording session is only closed when silence persists beyond the timer.

```rust
// In Voice Assistant's MessageHandler<DoaStatusMessage>
let was_speaking = self .previous_speech_detected;
let is_speaking = status.speech_detected;

if ! was_speaking & & is_speaking {
// Rising edge — enter listening mode immediately
self .enter_listening_mode(status.calibrated_angle, status.direction);
} else if was_speaking & & ! is_speaking {
// Falling edge — schedule exit after grace period
self .schedule_listening_exit(Duration::from_millis( self .grace_period_ms));
}

self .previous_speech_detected = is_speaking;
```

#### System Aspects & Latency

- **Polling adjustment**: To minimize the delay between hardware detection on the DSP and the Voice Assistant's reaction, the `poll_interval_ms` of the DoA
  service can be reduced from 150 ms to 50 ms. This trades USB traffic for lower activation latency.
- **False-trigger mitigation**: Impulsive environmental noises (e.g., cup clattering) can briefly trigger the DSP VAD. A minimum speech duration threshold
  (e.g., 100 ms of continuous VAD activity) prevents unwanted activations:

```rust
if ! was_speaking & & is_speaking {
self .vad_onset_timestamp = Some(Instant::now());
} else if was_speaking & & is_speaking {
// Confirm activation only after minimum duration
if let Some(onset) = self .vad_onset_timestamp {
if onset.elapsed() > = Duration::from_millis( self.min_speech_duration_ms) {
if ! self.listening_active {
self.enter_listening_mode(status.calibrated_angle, status.direction);
}
}
}
}
```

#### Advantages over Software VAD

- The XMOS DSP has dedicated hardware VAD with very low latency — it detects speech before the audio buffer reaches the Assistant.
- No CPU load for VAD on the host side.
- Works independently of the Audio service pipeline.
- The DoA angle provides additional spatial context that pure software VAD cannot offer.

#### Configuration Parameters (Voice Assistant side)

| Parameter                | Default | Description                                                           |
|--------------------------|---------|-----------------------------------------------------------------------|
| `grace_period_ms`        | 400     | Holdover time after falling edge before exiting Listening Mode        |
| `min_speech_duration_ms` | 100     | Minimum continuous VAD activity before activating Listening Mode      |
| `skip_wake_word_on_vad`  | false   | If true, skip software wake-word detection when hardware VAD triggers |

### 6.3 VAD-Controlled Audio Ducking

The Audio Service subscribes to `service.doa.status` and uses the `speech_detected` flag to duck media playback the moment the hardware VAD registers speech
activity. This leverages the near-zero latency of the XMOS DSP VAD — volume is reduced before the user finishes their first word.

#### State Transitions (Edge Detection)

The Audio Service implements edge detection on the `speech_detected` flag, analogous to the Voice Assistant's listening mode:

- **Rising Edge (`false → true`)**: Immediately reduces volume of active playback streams to a ducked level (e.g., 30% of original). Only media/music streams
  are affected — system sounds and Voice Assistant TTS output are excluded.
- **Falling Edge (`true → false`)**: Starts a Grace Period timer (configurable, default 400 ms). After the timer expires without resumed speech, volume is
  restored via a linear fade ramp (e.g., 500 ms) to avoid abrupt volume jumps.

```rust
// In Audio Service's MessageHandler<DoaStatusMessage>
if ! self .vad_ducking_enabled {
return;
}

let was_speaking = self .previous_speech_detected;
let is_speaking = status.speech_detected;

if ! was_speaking & & is_speaking {
// Rising edge — record onset timestamp, confirm duck after min duration
self .vad_onset_timestamp = Some(Instant::now());
self .cancel_volume_restore();
// Duck immediately if no chatter guard, or confirm after min_speech_duration_ms
if self .min_speech_duration_ms == 0 {
self.duck_playback_streams( self.duck_level_percent);
}
} else if was_speaking & & is_speaking {
// Continuous speech — confirm duck after min duration threshold
if ! self .is_ducked {
if let Some(onset) = self.vad_onset_timestamp {
if onset.elapsed() > = Duration::from_millis( self.min_speech_duration_ms) {
self.duck_playback_streams( self.duck_level_percent);
}
}
}
} else if was_speaking & & ! is_speaking {
// Falling edge — schedule restore after grace period with fade ramp
self .vad_onset_timestamp = None;
if self .is_ducked {
self .schedule_volume_restore(
Duration::from_millis( self .audio_grace_period_ms),
Duration::from_millis( self .fade_ramp_ms),
);
}
}

self .previous_speech_detected = is_speaking;
```

#### Stream Selection

Not all audio streams should be ducked. The Audio Service must distinguish stream types:

| Stream Type                           | Ducked?      | Rationale                                                      |
|---------------------------------------|--------------|----------------------------------------------------------------|
| Music / Media playback                | Yes          | Primary use case — music should quiet down when someone speaks |
| System sounds (notifications, alerts) | No           | Important alerts must remain audible                           |
| Voice Assistant TTS output            | No           | The assistant's own response must not duck itself              |
| Notification playback                 | Configurable | Some users may want notifications ducked, others not           |

#### Fade Ramp Behavior

Hard volume cuts are perceptually jarring. The restore path uses a linear fade ramp:

```rust
fn schedule_volume_restore(&mut self, grace: Duration, ramp: Duration) {
    // Store a cancellation token so the timer/ramp can be aborted
    // if speech resumes during the grace period or fade ramp.
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
    self.restore_cancel_token = Some(cancel_tx);

    // Wait for grace period, then linearly restore volume over ramp duration
    let target_volume = self.pre_duck_volume;
    let steps = (ramp.as_millis() / 50) as u32; // 50ms per step
    let increment = (target_volume - self.current_volume) / steps.max(1);
    // Schedule incremental volume increases via tokio interval,
    // aborting if cancel_rx receives or the sender is dropped.
}

fn cancel_volume_restore(&mut self) {
    // Drop the cancellation token to abort the pending grace timer
    // and any in-progress fade ramp. This is called on rising edge
    // to prevent the volume from rising while speech has resumed.
    if let Some(cancel) = self.restore_cancel_token.take() {
        let _ = cancel.send(());
    }
}
```

The duck path (rising edge) applies immediately without a ramp — the near-zero latency of the hardware VAD ensures the duck happens before the user's word
completes, making the transition feel natural.

**Re-triggering during restore**: If speech resumes (`false → true`) while the Grace Period timer is running or the fade ramp is in progress,
`cancel_volume_restore()` immediately aborts the pending task. The volume is then re-ducked to `duck_level_percent` without any ramp. This prevents perceptually
jarring volume oscillations (duck → partial restore → duck) during conversational speech with natural pauses.

#### Manual Volume Changes During Ducking

When the user manually adjusts the volume (e.g., via volume slider or hardware keys) while music is ducked, the Audio Service must update `pre_duck_volume`
accordingly. Otherwise, after the Grace Period the fade ramp would restore to a stale value, overriding the user's manual adjustment.

The recommended approach is **relative scaling**: instead of storing an absolute `pre_duck_volume`, the duck level is applied as a scale factor. Manual volume
changes during ducking update the base volume, and the ducked volume is always derived from it.

```rust
/// Applies the duck scale factor to the current base volume.
fn duck_playback_streams(&mut self, duck_level_percent: u8) {
    self.is_ducked = true;
    self.duck_scale = duck_level_percent as f32 / 100.0;
    let ducked_volume = (self.base_volume as f32 * self.duck_scale) as u8;
    self.set_playback_volume(ducked_volume);
}

/// Called when the user manually changes volume (from any source).
fn on_manual_volume_change(&mut self, new_volume: u8) {
    self.base_volume = new_volume;
    if self.is_ducked {
        // Re-apply duck scale to the new base volume
        let ducked_volume = (new_volume as f32 * self.duck_scale) as u8;
        self.set_playback_volume(ducked_volume);
    } else {
        self.set_playback_volume(new_volume);
    }
}

/// Restores volume after grace period — uses current base_volume, not a stale snapshot.
fn restore_volume(&mut self) {
    self.is_ducked = false;
    // Fade ramp targets base_volume, which reflects any manual changes made during ducking
    let target_volume = self.base_volume;
    // ... linear fade ramp to target_volume ...
}
```

**Key points:**

- `base_volume` is the single source of truth — always reflects the user's intended volume level
- `duck_scale` (e.g., 0.3 for 30%) is applied multiplicatively on top of `base_volume`
- Manual changes during ducking update `base_volume` and immediately re-apply the duck scale
- The fade ramp targets `base_volume` at restore time, so it always respects the latest manual adjustment
- No stale `pre_duck_volume` snapshot is stored or restored

#### Advantages over Software-Based Ducking

- **Latency**: Hardware VAD detects speech in real-time on the DSP. Software-based ducking via audio stream analysis adds 100–300 ms of processing latency,
  causing the first word to overlap with full-volume music.
- **CPU overhead**: No audio analysis pipeline needed on the host for duck detection.
- **Reliability**: The DSP VAD is tuned for speech detection and is less susceptible to music-triggered false positives (a common problem with software
  energy-based ducking).

#### Configuration Parameters (Audio Service side)

| Parameter                  | Default | Description                                                                           |
|----------------------------|---------|---------------------------------------------------------------------------------------|
| `vad_ducking_enabled`      | false   | Master switch to enable/disable VAD-controlled audio ducking                          |
| `min_speech_duration_ms`   | 80      | Minimum continuous VAD activity before ducking activates (chatter guard, 0 = instant) |
| `duck_level_percent`       | 30      | Target volume percentage during active speech (0–100)                                 |
| `audio_grace_period_ms`    | 400     | Holdover time after falling edge before starting restore ramp                         |
| `fade_ramp_ms`             | 500     | Duration of linear volume restore ramp                                                |
| `duck_notification_sounds` | false   | Whether to also duck notification sounds                                              |

### 6.4 Acoustic Echo Cancellation (AEC) & TTS Self-Triggering

When the Voice Assistant produces TTS output, the audio plays through speakers and is picked up by the microphone array. Without echo cancellation, the hardware
VAD would detect the assistant's own voice as `speech_detected = true`, causing self-triggering (Voice Assistant enters Listening Mode) and unwanted audio
ducking.

#### Preferred Solution: PipeWire AEC Mirroring

The XVF3800 is hardware- and firmware-designed for Acoustic Echo Cancellation. When the TTS audio signal is mirrored to the XVF3800's USB playback interface via
PipeWire, the XMOS DSP processes this stream internally as a Far-End Reference for its hardware AEC block.

**How the XVF3800 processes the mirrored audio:**

1. **UAC2 Downstream buffer**: The XMOS XVF3800 reads the audio signal from the USB playback interface (UAC2 Downstream).
2. **DSP evaluation**: Even if no physical speaker is connected to the 3.5mm jack or JST speaker connector of the ReSpeaker board, the DSP routes the digital
   PCM signal internally to the AEC module.
3. **Echo compensation & VAD suppression**: The AEC module computes the adaptive filter and subtracts the mirrored output signal in real-time from the raw data
   of the 4 PDM microphones. This keeps the processed USB capture stream clean, and the VAD register (`0x0016` / `speech_detected`) does not trigger on the
   assistant's own TTS output.

**PipeWire combine-stream configuration (PipeWire 1.6+):**

The deprecated `libpipewire-module-combine-sink` was replaced by `libpipewire-module-combine-stream` in PipeWire 1.6+. The new module uses `stream.rules` with
match-based routing instead of `combine.children`.

```conf
# ~/.config/pipewire/pipewire.conf.d/99-respeaker-aec.conf

context.modules = [
    {   name = libpipewire-module-combine-stream
        args = {
            combine.mode = sink
            node.name = "aec_speaker_combined"
            node.description = "Audio Output with ReSpeaker XVF3800 AEC Feed"
            combine.props = {
                audio.channels = 2
                audio.position = [ FL FR ]
            }
            stream.props = {}
            stream.rules = [
                {
                    matches = [
                        { media.class = "Audio/Sink" node.name = "alsa_output.pci-0000_03_00.1.hdmi-stereo" }
                    ]
                    actions = { create-stream = {} }
                }
                {
                    matches = [
                        { media.class = "Audio/Sink" node.name = "alsa_output.usb-Seeed_Studio_reSpeaker_XVF3800_4-Mic_Array_114993701262100698-00.analog-stereo" }
                    ]
                    actions = { create-stream = {} }
                }
            ]
        }
    }
]
```

**Setup steps:**

1. Ensure the ReSpeaker XVF3800 card profile is set to a duplex profile with playback (e.g., `output:analog-stereo+input:analog-stereo`), otherwise no sink node
   is created:
   ```bash
   pactl set-card-profile alsa_card.usb-Seeed_Studio_reSpeaker_XVF3800_4-Mic_Array_<serial>-00 output:analog-stereo+input:analog-stereo
   ```
2. Verify both sinks appear in `pactl list short sinks`.
3. Place the config file in `~/.config/pipewire/pipewire.conf.d/99-respeaker-aec.conf`.
4. Restart PipeWire: `systemctl --user restart pipewire pipewire-pulse`.
5. Set the combine sink as default: `pactl set-default-sink aec_speaker_combined`.
6. Set `aec_mirroring_enabled = true` in the Voice Assistant config to disable the software TTS mute window fallback.

**Practical notes for PipeWire mirroring:**

- **Independent of physical output**: It makes no difference to the XMOS DSP whether sound is actually played through the ReSpeaker's jack or through external
  monitors (DP/HDMI). As long as the audio data reaches the XVF3800's USB endpoint, the mathematical cancellation works.
- **Latency synchronicity**: The signal sent to the main output (e.g., monitor) and the mirrored signal to the XVF3800 should arrive at the DSP largely
  synchronously. PipeWire's `libpipewire-module-combine-stream` provides precise buffer synchronization by default.
- **Undistorted reference signal**: The mirrored reference signal must not be altered by software equalizers or dynamic compressors, as the XMOS firmware's
  adaptive filter model would deviate and fail to fully resolve the echo.

#### Fallback: Software-Side TTS-Mute-Window

If AEC mirroring is not configured (e.g., TTS routes through a different output device without PipeWire combine-sink), the Voice Assistant must suppress
VAD-triggered actions during TTS playback using a software-side mute window.

The Voice Assistant sets a `tts_active` flag during TTS output plus a holdover period (e.g., 300 ms after TTS ends). While `tts_active` is true,
`speech_detected` edges are ignored:

```rust
// In Voice Assistant's MessageHandler<DoaStatusMessage>
if self .tts_active {
// Ignore VAD during TTS playback — AEC may not be available
// if TTS routes through a different output device.
self .previous_speech_detected = status.speech_detected;
return;
}

let was_speaking = self .previous_speech_detected;
let is_speaking = status.speech_detected;

if ! was_speaking & & is_speaking {
self .enter_listening_mode(status.calibrated_angle, status.direction);
}
// ... normal edge detection logic ...

self .previous_speech_detected = is_speaking;
```

The `tts_active` flag is set when TTS synthesis begins and cleared 300 ms after the TTS audio buffer finishes playback, accounting for output latency and room
reverb decay.

#### Audio Ducking During TTS

TTS-triggered ducking is generally acceptable — music should be quieter while the assistant speaks. However, if the user prefers music to continue at full
volume during TTS, the `tts_active` flag can be used to suppress ducking:

```rust
// In Audio Service's MessageHandler<DoaStatusMessage>
if ! self .vad_ducking_enabled {
return;
}

// If TTS is active and user configured no-duck-during-TTS, skip ducking
if self .tts_active & & ! self .duck_during_tts {
self .previous_speech_detected = status.speech_detected;
return;
}

// ... normal ducking edge detection logic ...
```

#### Configuration Parameters

| Parameter               | Default | Description                                                                                                 |
|-------------------------|---------|-------------------------------------------------------------------------------------------------------------|
| `aec_mirroring_enabled` | false   | Indicates whether PipeWire AEC mirroring to XVF3800 is configured (disables software mute window when true) |
| `tts_mute_holdover_ms`  | 300     | Holdover time after TTS ends before re-enabling VAD edge detection                                          |
| `duck_during_tts`       | true    | Whether to duck media playback during Voice Assistant TTS output                                            |

### 6.5 Audio Service Integration (Future)

The Audio service could also subscribe to DoA status to apply directional audio processing based on the detected speaker direction. This includes input gain
adjustment, beamforming steering toward the active speaker, and noise suppression tuned for the detected direction. This is a future enhancement and not part of
the initial implementation.

---

## 7. Config Integration

### 7.1 Service Config (`configs/services/doa.toml`)

```toml
[services.doa]
poll_interval_ms = 150
mcp_enabled = true
reconnect_delay_ms = 2000
rotation_offset = 0  # Calibration offset in degrees (-360 to 360) to align DSP 0° with table North
# product_id = 0x0021  # Optional: filter by specific product ID
```

### 7.2 Widget Config (in `config.toml` or area config)

```toml
[[plugins]]
plugin_id = "doa_widget"
display_name = "Direction"
icon_name = "nf-md-compass"
width = 100
height = 100
icon_size = 36
views = ["Compass", "Direction", "DeviceInfo"]

[plugins.icons]
icon_compass = "nf-md-compass"
icon_north = "nf-md-compass_north"
icon_east = "nf-md-compass_east"
icon_south = "nf-md-compass_south"
icon_west = "nf-md-compass_west"
icon_disconnected = "nf-md-compass_off"

[plugins.actions]
click_mode = "supplement"
# click = { topic = "service.doa.command", payload = { action = "Reconnect", value = 0 } }
```

### 7.3 Udev Rules (`resources/udev/52-respeaker.rules`)

```udev
# ReSpeaker XVF3800 USB 4-Mic Array udev rules
# Copy to /etc/udev/rules.d/ and reload with:
#   sudo udevadm control --reload-rules && sudo udevadm trigger

# Seeed Studio Vendor ID
SUBSYSTEM=="usb", ATTR{idVendor}=="2886", TAG+="uaccess", MODE="0666"

# XMOS Vendor ID
SUBSYSTEM=="usb", ATTR{idVendor}=="20b1", TAG+="uaccess", MODE="0666"
```

---

## 8. Implementation Phases

### Phase 1: Model Crate (`model/doa`)

**Order:** First — no dependencies.

**Tasks:**

- Create `model/doa/Cargo.toml` with `stabby` (with `serde` feature), `serde`, `serde_json`, `smearor-model-mcp`, `plugin-api` dependencies
- Create `model/doa/src/direction.rs` with `DoaDirection` enum, `from_angle()`, `from_angle_with_offset()`, `label_key()` methods
- Create `model/doa/src/mcp_tools.rs` with `DoaMcpTools` enum, `AsRef<str>`, `FromStr`, `Display` impls (following the pattern from `WeatherMcpTools` /
  `AppLauncherMcpTools`)
- Create `model/doa/src/messages/status.rs` with `DoaStatusMessage` struct (carries `#[stabby::stabby]`, uses `StabbyString`)
- Create `model/doa/src/messages/command.rs` with `DoaCommandMessage` struct and `DoaCommandAction` enum (both `#[stabby::stabby]`)
- Create `model/doa/src/messages/view.rs` with `DoaView` enum
- Create `model/doa/src/lib.rs` with module declarations, `pub use` re-exports, `impl_json_convertible!` macros, `register_json_converters()` function
- Add `#[stabby::stabby]` on all FFI-relevant types
- Add `stabby` with `serde` feature in `Cargo.toml`
- All message types derive `Serialize, Deserialize` from `serde`
- All structs used as deserialization fallbacks derive `Default`

**Exit Criteria:** `cargo build -p smearor-doa-model` succeeds.

### Phase 2: Service Crate (`services/doa`)

**Order:** Second — depends on Phase 1.

**Tasks:**

- Create `services/doa/Cargo.toml` with `rusb`, `tokio`, `tracing`, `plugin-api`, `model/doa`, `model/mcp` dependencies
- Create `services/doa/src/config.rs` with `DoaServiceConfig` struct using `#[serde(default)]` for all fields
- Create `services/doa/src/service/loaded_service.rs` with `DoaService` struct
- Implement `ServicePlugin` trait (`on_message`, `start`)
- Implement `MessageHandler<FfiEnvelopePayload<DoaCommandMessage>>` trait
- Implement `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` trait for MCP tools
- Implement `MessageBroadcaster`, `MessageTopicBroadcaster<DoaStatusMessage>`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `McpCapabilitiesRegistrator` trait with tool and resource registration
- Implement USB device discovery (`open_respeaker`) matching VID `0x2886` and `0x20b1`
- Implement USB Control Transfer reading (`read_doa_angle`) with `REQUEST_TYPE_READ = 0xC0`, `B_REQUEST_READ = 0x00`, `PARAM_DOA_ANGLE = 0x0015`
- Implement VAD flag reading (`read_speech_detected`) via `PARAM_VAD = 0x0016` to distinguish active speech from held-angle silence
- Implement dedicated USB reader thread (`usb_reader_loop`) that owns the `rusb::DeviceHandle` and performs blocking `read_control` transfers
- Implement `DoaReading` and `UsbControl` channel types for thread-to-async-loop communication
- Implement async control loop with `tokio::select!` (command channel + reading channel) — never performs blocking I/O
- Implement reconnection logic with configurable delay (in USB reader thread)
- Implement `classify_and_handle_usb_error` to distinguish `NoDevice`/`NotFound`/`Io` (physical disconnect) from `Busy`/`Timeout`/`Pipe` (transient) with
  backoff and log suppression
- Ensure `open_respeaker` drops the old `DeviceHandle` before opening a new one to release USB interface claims
- Implement pause/resume functionality (via `UsbControl` channel forwarding)
- Use `tokio::sync::mpsc::unbounded_channel` for command channel
- Implement `Drop` for `DoaService` to trigger graceful shutdown cascade (command_sender drop → async loop exit → usb_control_sender drop → USB thread exit)
- Handle `None` from `command_receiver.recv()` in async loop to break on shutdown
- Use `service_plugin!(DoaService);` macro in `lib.rs`
- Create `services/doa/debian/` maintainer scripts directory
- Create `resources/udev/52-respeaker.rules` udev rules file
- No `unwrap()` or `expect()` in production code

**Exit Criteria:** `cargo build -p smearor-doa-service` succeeds. Service loads, connects to USB device, and broadcasts DoA status.

### Phase 3: Widget Crate (`plugins/doa`)

**Order:** Third — depends on Phase 1 and Phase 2.

**Tasks:**

- Create `plugins/doa/Cargo.toml` with `gtk4`, `glib`, `plugin-api`, `model/doa`, `model/personalization`, `model/widget` dependencies
- Implement `config.rs` with `DoaWidgetConfig` struct using shared config structs (`WidgetDimensions`, `WidgetLayout`, `WidgetIcon`, `WidgetTextColors`,
  `WidgetMode`) via `#[serde(flatten)]`
- Implement `DoaIcons` struct with all direction-specific icon fields and `Default` impl, used via `#[serde(flatten)]` in `DoaWidgetConfig`
- Use `ActionBindings` via `#[serde(flatten)]` for gesture bindings
- Support `BindingMode` (`replace`/`supplement`) per binding
- Implement `widget.rs` with `DoaWidget` struct
- Implement `WidgetPlugin` trait (`on_message`, `start`)
- Implement `WidgetBuilder` trait (`build_widget`)
- Implement `MessageHandler<FfiEnvelopePayload<DoaStatusMessage>>` trait
- Implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` trait for locale-aware labels
- Implement `MessageBroadcaster` trait
- Implement `MessageTopicBroadcaster<DoaCommandMessage>` trait
- Implement `MessageTopicBroadcaster<WidgetUpdateMessage>` trait for headless/Web instance sync
- Implement `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `DefaultFallback` trait for view-dependent click behavior
- Implement `AcceptTopic<FfiEnvelope>` trait for topic filtering
- Implement `GestureHandler` trait and call `attach_gesture_handlers` in `build_widget`
- Implement `render_view` returning `ViewData` for all `DoaView` variants
- Implement `GraphicRenderer::render_graphic` for headless instance pixel rendering
- Implement `WebRenderer::render_html` for web instance HTML fragment rendering
- Use `resolve_gtk_nerd_icon()` for GTK icon resolution and `resolve_icon_codepoint()` for pixel/atomic rendering
- Implement `DoaLabel` struct for locale-aware labels (analogous to `NetworkLabel`)
- Implement `update_ui` with `glib::MainContext::default().spawn_local` for GTK updates
- Implement `broadcast_widget_update` after every UI update
- Implement `start_listeners` subscribing to `TOPIC_STATUS` and `TOPIC_PERSONALIZATION_STATUS`
- Use `glib::MainContext::default().spawn_local` for GTK updates
- Use `tokio::sync::mpsc` for message reception
- Use `widget_plugin_graphic!(DoaWidget);` macro in `lib.rs`
- No polling loops (`timeout_add_local`); use event-driven `recv().await`
- No `unwrap()` or `expect()` in production code

**Exit Criteria:** `cargo build -p smearor-doa-widget` succeeds. Widget displays DoA direction and responds to clicks.

### Phase 4: Cross-Service Coordination (Voice Assistant)

**Order:** Fourth — depends on Phase 2 and Phase 3.

**Tasks:**

- Voice Assistant service subscribes to `service.doa.status` topic
- Voice Assistant stores latest DoA angle and `speech_detected` flag in its state
- Voice Assistant can reference the speaker direction in responses when relevant
- Implement edge detection on `speech_detected` flag (rising/falling edge)
- Implement VAD-triggered Listening Mode activation on rising edge with `calibrated_angle` and `direction` as context metadata
- Implement Grace Period holdover timer on falling edge (configurable, default 400 ms)
- Implement false-trigger mitigation via minimum speech duration threshold (configurable, default 100 ms)
- Add Voice Assistant config parameters: `grace_period_ms`, `min_speech_duration_ms`, `skip_wake_word_on_vad`, `aec_mirroring_enabled`, `tts_mute_holdover_ms`
- Implement TTS-Mute-Window fallback: `tts_active` flag suppresses VAD edge detection during TTS playback + holdover when AEC mirroring is not configured
- Document the `DoaStatusMessage` consumption, VAD-triggered listening mode, and AEC/TTS-mute strategy in the Voice Assistant service

**Exit Criteria:** Voice Assistant receives DoA status updates, enters Listening Mode on rising edge with spatial context, and exits after Grace Period on
falling edge. False-trigger mitigation is active. TTS-Mute-Window suppresses self-triggering when AEC mirroring is not configured.

### Phase 4b: Cross-Service Coordination (Audio Service Ducking)

**Order:** Fourth — depends on Phase 2. Can run in parallel with Phase 4.

**Tasks:**

- Audio Service subscribes to `service.doa.status` topic
- Audio Service stores `speech_detected` flag and tracks edge transitions
- Implement immediate ducking on rising edge (reduce media playback volume to `duck_level_percent`)
- Implement Grace Period holdover timer on falling edge (configurable, default 400 ms)
- Implement linear fade ramp for volume restore (configurable, default 500 ms)
- Implement stream type filtering — only duck music/media streams, exclude system sounds and Voice Assistant TTS
- Add Audio Service config parameters: `vad_ducking_enabled`, `min_speech_duration_ms`, `duck_level_percent`, `audio_grace_period_ms`, `fade_ramp_ms`,
  `duck_notification_sounds`, `duck_during_tts`
- Implement TTS-aware ducking suppression: skip ducking during TTS when `duck_during_tts = false`
- Document the `DoaStatusMessage` consumption, VAD-controlled ducking, and TTS ducking behavior in the Audio Service

**Exit Criteria:** Audio Service ducks media playback on rising edge, restores volume via fade ramp after Grace Period on falling edge. Stream type filtering
excludes system sounds and TTS. TTS-aware ducking suppression works when configured.

### Phase 5: Workspace Wiring

**Order:** Fifth — depends on all previous phases.

**Tasks:**

- Add `model/doa`, `services/doa`, `plugins/doa` to workspace `Cargo.toml`
- Add service loading to `smearor-swipe-launcher/src/service/loaded_service.rs` or service discovery
- Add plugin loading to `smearor-swipe-launcher/src/plugin/loaded_plugin.rs` or plugin discovery
- Add default config entries to `config.toml`
- Add udev rules to `resources/udev/52-respeaker.rules`
- Add debian packaging assets for the service crate

**Exit Criteria:** Launcher starts with DoA service and widget loaded. `config.toml` contains DoA entries. Udev rules are installed.

### Phase 6: Integration and Tests

**Order:** Sixth — depends on all previous phases.

**Tasks:**

- Verify DoA angle is read correctly from the ReSpeaker XVF3800 (angle updates in real time)
- Verify `speech_detected` flag is `true` during active speech and `false` during silence
- Verify angle is held (not reset to 0) during silence when `speech_detected = false`
- Verify widget Direction and DeviceInfo views show "Speaking"/"Silent" indicator based on `speech_detected`
- Verify direction mapping (N/E/S/W) is correct for all quadrants
- Verify rotation_offset calibration: raw DSP angle + offset produces correct calibrated_angle and direction (test positive, negative, and zero offsets)
- Verify widget displays calibrated_angle (not raw DSP angle) in Compass and Direction views
- Verify widget compass view displays the correct direction icon
- Verify widget direction view displays the correct text label
- Verify widget device info view displays VID/PID
- Verify view rotation (swipe up/down) cycles through views
- Verify click fallback cycles through views
- Verify long-press fallback cycles through views
- Test with no ReSpeaker device connected (graceful degradation: widget shows "Disconnected")
- Test with device disconnected mid-operation (reconnection logic triggers, old handle dropped before reopening)
- Test reconnection after physical replug
- Test that `NoDevice`/`NotFound` errors log at `warn!` level (not `error!`) and trigger immediate reconnection
- Test that `Busy`/`Timeout` errors keep the handle open, use backoff, and suppress repeated log messages after 3 attempts
- Test that VAD read failure falls back to `speech_detected = false` without triggering reconnection (angle read still succeeds)
- Test that consecutive successful reads reset the failure counter
- Test pause/resume commands (forwarded via `UsbControl` channel to USB thread)
- Test `SetPollInterval` command (interval changes take effect in USB thread)
- Test that USB transfer timeout scales with poll interval (50ms → 25ms timeout, 150ms → 75ms timeout, 200ms → 100ms timeout cap)
- Test that two consecutive USB timeouts at 50ms poll interval do not block the USB thread longer than one poll cycle
- Test command responsiveness while USB read is in progress (async loop must not block)
- Test graceful shutdown: service unload closes command channel → async loop exits → USB reader thread exits (no leaked threads)
- Test MCP tools: `doa_get_direction`, `doa_set_poll_interval`, `doa_reconnect`
- Test config parsing with partial TOML (defaults applied)
- Test Voice Assistant receives DoA status updates
- Test Voice Assistant enters Listening Mode on rising edge of `speech_detected` (false → true)
- Test Voice Assistant exits Listening Mode after Grace Period on falling edge (true → false)
- Test that brief speech pauses shorter than Grace Period do not exit Listening Mode
- Test that impulsive noise (VAD active < `min_speech_duration_ms`) does not trigger Listening Mode
- Test that `calibrated_angle` and `direction` are attached as context metadata on activation
- Test `skip_wake_word_on_vad` configuration behavior
- Test Audio Service ducks media playback on rising edge of `speech_detected`
- Test Audio Service restores volume via fade ramp after Grace Period on falling edge
- Test that brief speech pauses shorter than Grace Period do not trigger volume restore
- Test that speech resuming during Grace Period cancels the pending restore timer and re-ducks immediately
- Test that speech resuming during fade ramp cancels the in-progress ramp and re-ducks immediately
- Test that no volume oscillation occurs during conversational speech with natural pauses
- Test that manual volume increase during ducking updates `base_volume` and re-applies duck scale
- Test that manual volume decrease during ducking updates `base_volume` and re-applies duck scale
- Test that fade ramp after Grace Period targets the updated `base_volume` (not a stale pre-duck snapshot)
- Test that impulsive noise (VAD active < `min_speech_duration_ms`) does not trigger ducking
- Test that continuous speech exceeding `min_speech_duration_ms` triggers ducking
- Test that `min_speech_duration_ms = 0` ducks instantly on rising edge (no chatter guard)
- Test that system sounds and Voice Assistant TTS output are not ducked
- Test that `vad_ducking_enabled = false` completely disables ducking (no volume changes on speech detection)
- Test that `vad_ducking_enabled = true` activates ducking as expected
- Test that `duck_notification_sounds` config controls whether notifications are ducked
- Test fade ramp produces smooth volume transition (no abrupt jumps)
- Test that Voice Assistant does not enter Listening Mode during TTS playback when `aec_mirroring_enabled = false` (TTS-Mute-Window active)
- Test that Voice Assistant resumes VAD edge detection after `tts_mute_holdover_ms` following TTS end
- Test that Voice Assistant processes VAD normally during TTS when `aec_mirroring_enabled = true` (mute window disabled)
- Test that Audio Service ducks media during TTS when `duck_during_tts = true`
- Test that Audio Service does not duck media during TTS when `duck_during_tts = false`
- Test PipeWire AEC mirroring configuration (documented setup, not automated)
- Test locale-aware labels (German and English)
- Test headless instance pixel rendering
- Test web instance HTML rendering
- No `unwrap()` or `expect()` in production code paths

**Exit Criteria:** All tests pass. DoA widget is fully functional.

### Phase 7: Documentation

**Order:** Seventh — depends on all previous phases.

**Tasks:**

- Update `book/src/SUMMARY.md` with DoA-related chapters
- Add `book/src/features/doa.md` describing the DoA widget, views, and configuration
- Add `book/src/architecture/doa.md` describing the service architecture, USB integration, and event-driven updates
- Update `book/src/configuration/` with DoA service and widget config examples
- Update `README.md` feature list to include DoA widget and service
- Document udev rules setup in the book
- Document Voice Assistant DoA integration in the book

**Exit Criteria:** `mdbook build` succeeds. README.md lists DoA as a feature. Book contains DoA documentation.

---

## 9. Dependencies

| Crate          | Dependencies                                                                                            |
|----------------|---------------------------------------------------------------------------------------------------------|
| `model/doa`    | `stabby` (with `serde` feature), `serde`, `serde_json`, `plugin-api`                                    |
| `services/doa` | `rusb`, `tokio`, `tracing`, `plugin-api`, `model/doa`, `model/mcp`                                      |
| `plugins/doa`  | `gtk4`, `glib`, `image`, `ab_glyph`, `plugin-api`, `model/doa`, `model/personalization`, `model/widget` |

**System dependencies:**

- `libusb-1.0-0` (for `rusb`)
- Linux udev rules for USB access without root

---

## 10. Error Handling

- All USB calls use `Result<T, E>` with proper error logging via `error!`
- Missing ReSpeaker device: service broadcasts `connected: false` status, widget shows "Disconnected"
- USB read failures: logged with `error!`, reconnection attempted with configurable delay
- No `unwrap()` or `expect()` in production code
- Graceful degradation when device is unplugged mid-operation
- `rusb::Error` variants are matched explicitly for meaningful error messages

---

## 11. Icon Reference

| Icon Name           | Nerd Font Icon        | Usage                |
|---------------------|-----------------------|----------------------|
| `icon_compass`      | `nf-md-compass`       | Default compass icon |
| `icon_north`        | `nf-md-compass_north` | Direction: North     |
| `icon_east`         | `nf-md-compass_east`  | Direction: East      |
| `icon_south`        | `nf-md-compass_south` | Direction: South     |
| `icon_west`         | `nf-md-compass_west`  | Direction: West      |
| `icon_disconnected` | `nf-md-compass_off`   | Device disconnected  |

---

## 12. Personalization Integration

The DoA widget subscribes to `TOPIC_PERSONALIZATION_STATUS` (from the Personalization service) to receive locale updates. When a `PersonalizationStatusMessage`
arrives, the widget stores it in `latest_personalization` and triggers a UI re-render.

The `DoaLabel` struct (see Section 5.10) uses the locale from `PersonalizationStatusMessage` to select appropriate label strings for all view text. This is
analogous to `NetworkLabel` in the Network widget.

The widget must implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` and `AcceptTopic<FfiEnvelope>` filtering for
`TOPIC_PERSONALIZATION_STATUS`.

---

## 13. Future Enhancements

- **Beamforming control**: Use DoA to steer the XVF3800's beamforming toward the detected speaker direction via additional USB Control Transfers.
- **Multi-device support**: Handle systems with multiple ReSpeaker arrays, each with independent DoA readings.
- **DoA history graph**: Display a mini time-series chart of recent DoA angles in a dedicated widget view.
- **Speaker tracking**: Combine DoA with voice activity detection to track which table side is actively speaking and highlight it in the UI.
- **Configurable direction thresholds**: Allow custom angle ranges for N/E/S/W mapping via config (e.g., asymmetric quadrant sizes for non-square tables). The
  current `rotation_offset` handles physical mounting rotation; this would extend to custom quadrant boundaries.
- **Audio service integration**: Feed DoA angle to the Audio service for directional gain control or noise suppression.
- **Calibration mode**: Interactive calibration wizard where the user speaks from each table side to verify the angle-to-direction mapping.
- **XVF3800 parameter control**: Expose additional XVF3800 DSP parameters (AEC, AGC, noise suppression) via MCP tools.
- **Raw multichannel audio access**: Access the 4-channel raw microphone data for custom DSP processing in Rust.
