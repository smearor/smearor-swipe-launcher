# Audio Widget & Service

This document describes the architecture for the **Audio Widget Plugin** and **Audio Service Plugin** of the *Smearor Swipe Launcher*. The system runs on
Ubuntu/Linux and uses **PipeWire** (via `pulsectl-rs` / `libpulse-binding` for the PulseAudio compatibility layer) for native audio control.

The concept cleanly separates UI interactions (Touch & Mouse) from system logic using the ABI-stable plugin crate architecture.

---

## 1. Crate Structure

The audio feature is split into three separate crates following the project architecture:

| Crate       | Path              | Responsibility                                   |
|-------------|-------------------|--------------------------------------------------|
| **Model**   | `model/audio/`    | Shared structs, enums, and message formats       |
| **Service** | `services/audio/` | Backend logic, PipeWire/PulseAudio communication |
| **Widget**  | `plugins/audio/`  | GTK4 user interface, gesture handling            |

---

## 2. Model Crate (`model/audio`)

Contains all message types and shared data structures used by both the service and widget.

### 2.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.audio.command";
pub const TOPIC_STATUS: &str = "service.audio.status";
```

### 2.2 Command Messages (Widget -> Service)

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum AudioCommandAction {
    #[default]
    /// Increase the volume by a relative amount
    VolumeUp,
    /// Decrease the volume by a relative amount
    VolumeDown,
    /// Set the volume to an absolute value (0.0 - 1.0)
    SetVolume,
    /// Toggle the mute state
    ToggleMute,
    /// Switch to the next output device
    NextDevice,
    /// Switch to the previous output device
    PreviousDevice,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AudioCommandMessage {
    /// The action to execute
    pub action: AudioCommandAction,
    /// Optional absolute volume value (0.0 to 1.0, used with SetVolume)
    pub volume: Option<f32>,
    /// Optional device identifier for device switching
    pub device_id: Option<u32>,
}

impl AudioCommandMessage {
    pub fn volume_up() -> Self { ... }
    pub fn volume_down() -> Self { ... }
    pub fn set_volume(volume: f32) -> Self { ... }
    pub fn toggle_mute() -> Self { ... }
    pub fn next_device() -> Self { ... }
    pub fn previous_device() -> Self { ... }
}
```

### 2.3 Status Messages (Service -> Widget)

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AudioDevice {
    /// Unique device identifier
    pub id: u32,
    /// Human-readable device name
    pub name: String,
    /// Whether this is the default/active device
    pub is_default: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AudioStatusMessage {
    /// Current master volume (0.0 to 1.0, may exceed 1.0 if overdrive is enabled)
    pub volume: f32,
    /// Whether the audio is currently muted
    pub is_muted: bool,
    /// List of available output devices
    pub output_devices: Vec<AudioDevice>,
    /// List of available input devices
    pub input_devices: Vec<AudioDevice>,
    /// The currently active output device
    pub active_device: Option<AudioDevice>,
}
```

---

## 3. Service Plugin (`services/audio`)

The service plugin is a singleton that runs in the background. It handles all audio system communication and broadcasts state updates to all subscribed widgets.

### 3.1 File Structure

- `service.rs` - `AudioService` struct and trait implementations
- `config.rs` - `AudioServiceConfig` struct and parsing
- `lib.rs` - `service_plugin!` macro invocation

### 3.2 Service Implementation

```rust
pub struct AudioService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AudioServiceConfig,
    pub state: Arc<RwLock<AudioStatusMessage>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<AudioCommandMessage>>` - Processes commands from widgets
- `MessageBroadcaster<AudioStatusMessage>` - Broadcasts status updates
- `MessageTopicBroadcaster<AudioStatusMessage>` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to core context

### 3.3 Command Handling

The service listens on `service.audio.command` and handles:

| Command          | Action                                           |
|------------------|--------------------------------------------------|
| `VolumeUp`       | Increases volume by configured step (default 5%) |
| `VolumeDown`     | Decreases volume by configured step (default 5%) |
| `SetVolume`      | Sets absolute volume to the provided value       |
| `ToggleMute`     | Toggles mute state on/off                        |
| `NextDevice`     | Cycles to the next available output device       |
| `PreviousDevice` | Cycles to the previous available output device   |

After each command, the service updates the system audio state and broadcasts an `AudioStatusMessage` to all widgets.

### 3.4 PipeWire / PulseAudio Integration

The service communicates with the audio server asynchronously:

1. **Initialization:** Subscribes to the audio server event stream on startup
2. **State Queries:** Reads the default sink and available sinks/sources
3. **Command Execution:**
    - **Volume:** `pactl set-sink-volume @DEFAULT_SINK@ <value>`
    - **Mute:** `pactl set-sink-mute @DEFAULT_SINK@ toggle`
    - **Device Switch:** `pactl set-default-sink <device_name>`

---

## 4. Widget Plugin (`plugins/audio`)

The widget plugin provides the GTK4 user interface. Multiple audio widget instances can exist, all synchronized via the service's status broadcasts.

### 4.1 File Structure

- `widget.rs` - `AudioWidget` struct and trait implementations
- `config.rs` - `AudioWidgetConfig` struct and parsing
- `lib.rs` - `widget_plugin!` macro invocation

### 4.2 Widget Implementation

```rust
pub struct AudioWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AudioWidgetConfig,
    pub volume_bar: Arc<RwLock<Option<gtk4::Box>>>,
    pub icon_image: Arc<RwLock<Option<gtk4::Image>>>,
    pub device_label: Arc<RwLock<Option<gtk4::Label>>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<AudioStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster<AudioCommandMessage>` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 4.3 UI Layout

- **Volume Bar:** A vertical or horizontal progress bar filling proportionally to the volume (0-100%)
- **Icon Indicator:** Dynamic speaker icon based on volume level and mute state
- **Device Label:** Small text showing the current output device name

### 4.4 Interaction Mapping

| Action            | Touch Input        | Mouse Input  | Generated Message                    |
|-------------------|--------------------|--------------|--------------------------------------|
| **Volume Up**     | Swipe up / right   | Scroll up    | `AudioCommandMessage::volume_up()`   |
| **Volume Down**   | Swipe down / left  | Scroll down  | `AudioCommandMessage::volume_down()` |
| **Mute / Unmute** | Short tap (center) | Middle click | `AudioCommandMessage::toggle_mute()` |
| **Next Device**   | Long press (2s)    | Right click  | `AudioCommandMessage::next_device()` |

When an interaction occurs, the widget creates the appropriate `AudioCommandMessage` and broadcasts it via the message broker to the service.

### 4.5 State Synchronization

All audio widgets subscribe to `service.audio.status`. When the service broadcasts a status update, all widgets update their UI simultaneously:

- Volume bar position updates to the new volume
- Icon changes based on mute state and volume level
- Device label updates to show the active device

### 4.6 Icons

- `audio-speakers-symbolic`
- `audio-volume-low-symbolic`
- `audio-volume-medium-symbolic`
- `audio-volume-high-symbolic`
- `audio-volume-muted-symbolic`
- `audio-volume-overamplified-symbolic`
- `audio-speakers-bluetooth-symbolic`

---

## 5. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
|   Audio Widget 1  |<--------|                   |-------->|   Audio Widget 2  |
| (Slider in Panel) |  Status |   Event Broker    |  Status | (Mute in Top Bar) |
+---------+---------+  Broadcast               Broadcast  +---------+---------+
          |                                                 ^
          | Command                                         | Status
          v                                                 |
+---------+-------------------+                             |
|      AudioService           |                             |
|   (PipeWire/PulseAudio)     |-----------------------------+
+-----------------------------+       Status Broadcast
```

---

## 6. Configuration

### 6.1 Widget Configuration (`config.rs`)

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AudioWidgetConfig {
    /// Widget width in pixels
    #[serde(default = "default_width")]
    pub width: i32,
    /// Widget height in pixels
    #[serde(default = "default_height")]
    pub height: i32,
    /// Volume change step (0.01 to 0.1)
    #[serde(default = "default_volume_step")]
    pub volume_step: f32,
    /// Whether to show the volume bar
    #[serde(default = "default_show_volume_bar")]
    pub show_volume_bar: bool,
    /// Whether to show the device name label
    #[serde(default = "default_show_device_label")]
    pub show_device_label: bool,
    /// Whether to allow volume over 100%
    #[serde(default)]
    pub allow_overdrive: bool,
}
```

### 6.2 Service Configuration

```rust
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct AudioServiceConfig {
    /// Path to the pulseaudio/pipewire control binary
    #[builder(default, setter(into))]
    pub pactl_path: Option<String>,
}
```

---

## 7. Technical Considerations

- **Scroll Event Debouncing:** Mouse wheel events can fire rapidly. The widget should accumulate events within a frame (approx. 16ms) and send a single command
  to avoid flooding the message broker.
- **Overdrive Support:** When `allow_overdrive` is enabled, the volume may exceed 1.0 (up to ~1.5 / 150%). The UI should visually indicate overdrive state (
  e.g., orange color above 100%).
- **Async Audio Communication:** All PipeWire/PulseAudio operations run asynchronously to avoid blocking the GTK4 main thread.
- **Singleton Service:** Only one `AudioService` instance exists, shared by all audio widgets, preventing redundant audio server connections.