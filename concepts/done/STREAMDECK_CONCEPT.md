# Concept: MacroPad Integration (Stream Deck & Loupedeck)

Integration of MacroPad hardware — devices with programmable LCD keys — into the *Smearor Swipe Launcher*. Each MacroPad device is treated as a **headless
Launcher Instance**, the **Plugin-API** is extended with a non-GTK rendering mechanism, and the existing **Button Widget** serves as the primary interaction
surface.

The host uses **generic MacroPad terminology**. Device-specific drivers (Elgato Stream Deck, Loupedeck) are isolated in separate service crates.

---

## 1. Hardware

### 1.1 Elgato Stream Deck

| Device                         | Keys | Key Resolution | Physical Key Size  | USB VID:PID | Driver Crate        |
|--------------------------------|------|----------------|--------------------|-------------|---------------------|
| Elgato Stream Deck Original V2 | 15   | 72 × 72 px     | ca. 16.1 × 16.1 mm | `0fd9:006d` | `elgato-streamdeck` |
| Elgato Stream Deck MK.2        | 15   | 72 × 72 px     | ca. 16.1 × 16.1 mm | `0fd9:0080` | `elgato-streamdeck` |
| Elgato Stream Deck MK.2        | 15   | 72 × 72 px     | ca. 16.1 × 16.1 mm | `0fd9:0080` | `elgato-streamdeck` |

### 1.2 Loupedeck

| Device         | Keys / Controls | Key Resolution | Driver Crate       |
|----------------|-----------------|----------------|--------------------|
| Loupedeck CT   | 12+ knobs/touch | varies         | `loupedeck-driver` |
| Loupedeck Live | 6+ knobs        | varies         | `loupedeck-driver` |

Loupedeck devices use a different HID protocol and require the `loupedeck-driver` crate. They are managed by a separate service but share the same host-level
MacroPad infrastructure.

---

## 2. Architecture: MacroPad as Headless Launcher Instance

### 2.1 Core Idea

Each MacroPad device is registered as a **headless Launcher Instance** — analogous to a GTK window instance but without `ApplicationWindow`. This reuses the
existing inter-instance communication infrastructure:

- **Message Broker**: `FfiEnvelope` with `target_instance_id` routes to the correct MacroPad instance (e.g. `"macropad_1"`).
- **Area System**: Each MacroPad instance has its own `AreaManager` with logical areas. Areas are independent of instances.
- **Button Widget**: The existing `ButtonWidget` handles actions via `click_topic`, `click_payload`, and `click_instance`. A GTK button press can target a
  MacroPad instance by setting `click_instance = "macropad_1"`.
- **Cross-Instance Addressing**: The broker router in `application.rs` already supports `target_instance_id` routing and colon-separated area IDs (e.g.
  `"macropad_1:app_launcher"`).

### 2.2 Instance Comparison

| Component           | GTK Instance        | MacroPad Instance           |
|---------------------|---------------------|-----------------------------|
| Window              | `ApplicationWindow` | None (headless)             |
| `PluginManager`     | Yes                 | Yes                         |
| `AreaManager`       | Yes (GTK widgets)   | Yes (logical areas only)    |
| Widget rendering    | GTK `WidgetBuilder` | `GraphicRenderer` trait     |
| Button image output | N/A                 | Via service command channel |

### 2.3 Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         Single Process                            │
│                                                                   │
│  ┌────────────────┐                                              │
│  │ gtk4::Application│                                             │
│  └───────┬────────┘                                              │
│          │                                                        │
│    ┌─────┴─────┬──────────┬──────────────┐                      │
│    │           │          │              │                        │
│ ┌──▼──┐   ┌──▼──┐   ┌──▼─────────┐  ┌──▼─────────┐              │
│ │Win 1│   │Win 2│   │MacroPad 1  │  │MacroPad 2  │  ...          │
│ │GTK  │   │GTK  │   │Headless    │  │Headless    │              │
│ └──┬──┘   └──┬──┘   └──┬─────────┘  └──┬─────────┘              │
│    │         │         │               │                         │
│ ┌──▼─────────▼─────────▼───────────────▼──────────┐              │
│ │              Central Message Broker              │              │
│ │     route_message() → target_instance_id         │              │
│ │     service.macropad.connection → create instance│              │
│ │     service.macropad.input → route to instance   │              │
│ └──────────────────┬──────────────────────────────┘              │
│                    │                                              │
│ ┌──────────────────▼──────────────────────────────┐              │
│ │              Shared ServiceManager               │              │
│ │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │              │
│ │  │streamdeck│ │loupedeck │ │weather   │        │              │
│ │  │service   │ │service   │ │service   │        │              │
│ │  └──────────┘ └──────────┘ └──────────┘        │              │
│ └─────────────────────────────────────────────────┘              │
└──────────────────────────────────────────────────────────────────┘
```

### 2.4 Host Changes Summary

The host changes are **additive and generic**, building on the **Dynamic Load** concept (`DYNAMIC_LOAD_LAUNCHER_INSTANCE.md`), which already supports headless
instances via `InstanceType::Headless`:

| Change                                                                     | Location         | Description                                                                                                                                     |
|----------------------------------------------------------------------------|------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `route_message()` — `service.macropad.connection` handler                  | `application.rs` | On `Connected`: calls `load_instance(instance_id, config_path, InstanceType::Headless)`. On `Disconnected`: calls `stop_instance(instance_id)`. |
| `route_message()` — `service.macropad.input` handler                       | `application.rs` | Routes `MacroPadInputMessage` to the `LauncherInstance` with matching `instance_id` in the existing `instances` HashMap.                        |
| `LauncherInstance` gains `device_metadata: Option<MacroPadDeviceMetadata>` | `instance.rs`    | Optional metadata for headless MacroPad instances (key_count, key_width, key_height, driver). `None` for GTK instances.                         |

No separate `MacroPadInstance` struct, no separate HashMap. MacroPad instances are standard `LauncherInstance` entries with `instance_type = Headless`. The host
reuses the existing `load_instance()` / `stop_instance()` lifecycle from DYNAMIC_LOAD. The host knows nothing about Stream Deck or Loupedeck — it only knows
about **headless instances** and the generic `service.macropad.*` topics.

---

## 3. Plugin-API Extension: GraphicRenderer Trait

### 3.1 Motivation

Widgets currently implement `WidgetBuilder::build_widget() -> gtk4::Widget`. MacroPad buttons need a **pixel buffer** instead. GTK rendering (`GtkSnapshot` +
`GskRenderer`) is not viable: no `ApplicationWindow` or GDK surface for headless instances, GTK rendering must happen on the main thread, and tight coupling to
GTK internals.

A new **`GraphicRenderer`** trait allows widgets to produce RGBA pixel data without GTK. Initially implemented by the Button Widget, extensible to other widgets
later.

### 3.2 FfiGraphic Struct

```rust
/// A rendered graphic frame for non-GTK display surfaces.
#[repr(C)]
pub struct FfiGraphic {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw RGBA pixel data (width * height * 4 bytes).
    pub pixels: *mut u8,
    /// Length of the pixel buffer in bytes.
    pub pixels_len: usize,
}
```

### 3.3 GraphicRenderer Trait

```rust
/// Trait for widgets that can render to a graphic (non-GTK).
pub trait GraphicRenderer {
    /// Render the widget to a graphic with the given dimensions.
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic;
}
```

### 3.4 Plugin VTable Extension

`PluginVTable` gains an optional `render_graphic` function pointer. `PLUGIN_VTABLE_VERSION` is incremented to `2`. Existing GTK-only widgets set this to `None`.

```rust
#[repr(C)]
pub struct PluginVTable {
    pub destroy: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
    pub build_widget: unsafe extern "C" fn(instance: *mut core::ffi::c_void) -> FfiWidget,
    pub on_message: unsafe extern "C" fn(instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void),
    pub start: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
    // New in v2:
    pub render_graphic: Option<unsafe extern "C" fn(
        instance: *mut core::ffi::c_void,
        width: u32,
        height: u32,
    ) -> FfiGraphic>,
}
```

---

## 4. Button Widget Graphic Rendering

### 4.1 Rendering Pipeline

The `LauncherInstance` requests rendering at the device-specific resolution. Stream Deck: 72×72 px. Loupedeck: device-dependent (e.g. 90×90 px for Loupedeck
CT). The `GraphicRenderer` implementation handles arbitrary dimensions.

```
┌──────────────────────────┐
│      W × H px RGBA       │
│                          │
│   ┌──────────────────┐   │
│   │                  │   │
│   │  NerdFont Icon   │   │
│   │  (centered)      │   │
│   │                  │   │
│   └──────────────────┘   │
│                          │
│      "Apps" (label)      │
└──────────────────────────┘
```

### 4.2 Rendering Steps

1. **Background fill**: Solid color from config or default `#1a1a2e`.
2. **Icon rasterization**: Load NerdFont TTF from `resources/NerdFontsSymbolsOnly/SymbolsNerdFont-Regular.ttf` via `ab_glyph`, rasterize glyph for `config.icon`
   at proportional size, center on image.
3. **Label rendering**: If `config.text` is non-empty and `icon_only` is false, render text below icon.
4. **State evaluation**: If `state_icon` is configured, evaluate expression against `internal_state` to select correct icon. Apply highlight color if
   `state_css_class` is active.
5. **Output**: `FfiGraphic` with RGBA pixel data.

### 4.3 Dependencies

| Crate       | Purpose                                 |
|-------------|-----------------------------------------|
| `image`     | Image buffer manipulation (`RgbaImage`) |
| `ab_glyph`  | Font loading and glyph rasterization    |
| `imageproc` | Drawing primitives (text, shapes, fill) |

### 4.4 Re-rendering on State Change

When a plugin receives a state update via `state_topic`, the headless `LauncherInstance` triggers a re-render of the affected button and writes the new image to
the device via the service command channel.

---

## 5. Model Crate (`model/macropad`)

### 5.1 Overview

A single model crate defines **generic MacroPad message types** used by the host and all MacroPad services. Device-specific details (e.g. `Kind` enum for Stream
Deck models) remain in their respective service crates.

### 5.2 Topics

```rust
pub const TOPIC_MACROPAD_INPUT: &str = "service.macropad.input";
pub const TOPIC_MACROPAD_CONNECTION: &str = "service.macropad.connection";
pub const TOPIC_MACROPAD_COMMAND: &str = "service.macropad.command";
```

### 5.3 Input Message

```rust
/// Input event from a MacroPad device.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct MacroPadInputMessage {
    /// Serial number or unique identifier of the source device.
    pub device_id: stabby::string::String,
    /// Instance ID associated with the device.
    pub instance_id: stabby::string::String,
    /// Button index that changed.
    pub button_index: u8,
    /// True if the button was pressed, false if released.
    pub pressed: bool,
}
```

### 5.4 Connection Status

```rust
/// Connection status of a MacroPad device.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct MacroPadConnectionStatus {
    /// Unique identifier for the device (serial number or composite ID).
    pub device_id: stabby::string::String,
    /// Instance ID assigned to the device.
    pub instance_id: stabby::string::String,
    /// Device type identifier (e.g. "streamdeck_original_v2", "streamdeck_mk2", "loupedeck_ct").
    pub device_type: stabby::string::String,
    /// Driver/service that manages this device (e.g. "streamdeck", "loupedeck").
    pub driver: stabby::string::String,
    /// Number of keys on the device.
    pub key_count: u8,
    /// Key resolution width in pixels.
    pub key_width: u32,
    /// Key resolution height in pixels.
    pub key_height: u32,
    /// True if connected, false if disconnected.
    pub connected: bool,
}
```

### 5.5 Command Message

```rust
/// Command sent to a MacroPad service to control a device.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct MacroPadCommandMessage {
    /// Device identifier (empty = all devices managed by the service).
    pub device_id: stabby::string::String,
    /// The command to execute.
    pub command: MacroPadCommand,
}

/// Commands supported by MacroPad services.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub enum MacroPadCommand {
    /// Set device brightness (0-100).
    SetBrightness(u8),
    /// Clear a specific button image.
    ClearButton(u8),
    /// Clear all button images.
    ClearAllButtons,
    /// Write an image to a button.
    SetButtonImage {
        button_index: u8,
        /// Raw RGBA pixel data.
        pixels: stabby::vec::Vec<u8>,
        width: u32,
        height: u32,
    },
    /// Reset the device (clear all buttons and reset brightness).
    Reset,
}
```

---

## 6. Stream Deck Service (`services/streamdeck`)

### 6.1 Overview

Singleton background service managing all connected Elgato Stream Deck devices: discovery, connection lifecycle, event reading, button image output. Uses the
`elgato-streamdeck` crate. No GTK code. Broadcasts generic `MacroPadConnectionStatus` and `MacroPadInputMessage` on the shared `service.macropad.*` topics.

### 6.2 Service Struct

```rust
pub struct StreamDeckService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: StreamDeckServiceConfig,
    pub command_sender: tokio::sync::mpsc::Sender<MacroPadCommand>,
    pub devices: Arc<Mutex<Vec<StreamDeckDeviceHandle>>>,
}
```

### 6.3 Device Handle

```rust
pub struct StreamDeckDeviceHandle {
    pub serial: String,
    pub kind: Kind,
    pub device: Arc<StreamDeck>,
    pub button_states: Vec<bool>,
    pub command_receiver: tokio::sync::mpsc::Receiver<MacroPadCommand>,
}
```

One thread per device (since `StreamDeck` is `!Sync`). Each thread runs:

1. `device.read_input(Some(Duration::from_millis(100)))` — blocks briefly.
2. Match `StreamDeckInput` variant, edge-detect against `button_states`.
3. On button press/release: broadcast `MacroPadInputMessage` on `service.macropad.input`.
4. Check `command_receiver.try_recv()` for pending commands.
5. Auto-reconnect on disconnect.

### 6.4 Device Discovery

On `start()`: create `HidApi`, enumerate `StreamDeck::list_devices(&hid)`, connect each `(kind, serial)`, spawn event loop thread, broadcast
`MacroPadConnectionStatus::Connected` with `driver: "streamdeck"` and `device_type` derived from `Kind`.

### 6.5 Configuration (`services.toml`)

```toml
[[services]]
id = "streamdeck"
path = "target/release/libsmearor_streamdeck_service.so"

[streamdeck]
brightness = 50
auto_reconnect = true
reconnect_interval_ms = 3000

[[streamdeck.devices]]
serial = "ELGATO_SERIAL_V2"
instance_id = "macropad_1"
brightness = 35

[[streamdeck.devices]]
serial = "ELGATO_MK2_SERIAL_1"
instance_id = "macropad_2"

[[streamdeck.devices]]
serial = "ELGATO_MK2_SERIAL_2"
instance_id = "macropad_3"
```

---

## 7. Loupedeck Service (`services/loupedeck`)

### 7.1 Overview

Singleton background service managing all connected Loupedeck devices. Uses the `loupedeck-driver` crate. Same generic message types and topics as the Stream
Deck service. No GTK code.

### 7.2 Service Struct

```rust
pub struct LoupedeckService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: LoupedeckServiceConfig,
    pub command_sender: tokio::sync::mpsc::Sender<MacroPadCommand>,
    pub devices: Arc<Mutex<Vec<LoupedeckDeviceHandle>>>,
}
```

### 7.3 Device Handle

```rust
pub struct LoupedeckDeviceHandle {
    pub serial: String,
    pub device: Arc<Loupedeck>,
    pub button_states: Vec<bool>,
    pub command_receiver: tokio::sync::mpsc::Receiver<MacroPadCommand>,
}
```

The Loupedeck event loop is analogous to the Stream Deck one but uses `loupedeck-driver` APIs for reading input and writing button images. The key resolution
may differ (e.g. 90×90 px for Loupedeck CT), which is communicated via `MacroPadConnectionStatus::key_width` / `key_height`.

### 7.4 Configuration (`services.toml`)

```toml
[[services]]
id = "loupedeck"
path = "target/release/libsmearor_loupedeck_service.so"

[loupedeck]
brightness = 50
auto_reconnect = true

[[loupedeck.devices]]
serial = "LOUPEDECK_CT_SERIAL"
instance_id = "macropad_4"
```

---

## 8. MacroPad Instance (Host Integration)

### 8.1 Overview

MacroPad instances are **standard `LauncherInstance` entries** with `instance_type = InstanceType::Headless`. They are created and destroyed via the existing
`load_instance()` / `stop_instance()` methods from the Dynamic Load concept (`DYNAMIC_LOAD_LAUNCHER_INSTANCE.md`).

No separate `MacroPadInstance` struct or separate HashMap is needed. The existing `instances: Arc<Mutex<HashMap<String, LauncherInstance>>>` holds both GTK and
headless instances.

`LauncherInstance` gains one optional field for MacroPad device metadata:

```rust
/// Metadata for headless MacroPad instances. None for GTK instances.
pub device_metadata: Option<MacroPadDeviceMetadata>,
```

```rust
/// Device-specific metadata for a MacroPad instance.
#[derive(Clone, Debug)]
pub struct MacroPadDeviceMetadata {
    pub device_id: String,
    pub key_count: u8,
    pub key_width: u32,
    pub key_height: u32,
    pub driver: String,
}
```

### 8.2 Creation Flow

1. A MacroPad service (Stream Deck or Loupedeck) discovers a device, broadcasts `MacroPadConnectionStatus::Connected`.
2. `LauncherHost::route_message()` matches `service.macropad.connection` topic.
3. `LauncherHost` calls `load_instance(instance_id, config_path, InstanceType::Headless)` — reuses the DYNAMIC_LOAD infrastructure.
4. After load, `LauncherHost` attaches `MacroPadDeviceMetadata` to the instance (key_count, key_width, key_height from the connection message).
5. Instance loads config, plugins, and areas (same as any instance).
6. For each button in area layout, instance calls `render_graphic(key_width, key_height)` on the plugin and sends `MacroPadCommandMessage::SetButtonImage` to
   the service.

### 8.3 Destruction Flow

1. Service detects device disconnect, broadcasts `MacroPadConnectionStatus::Disconnected`.
2. `LauncherHost::route_message()` matches `service.macropad.connection` topic.
3. `LauncherHost` calls `stop_instance(instance_id)` — reuses the DYNAMIC_LOAD infrastructure.
4. Plugins are unloaded, MCP tools are unregistered, instance is removed from `instances` HashMap.

### 8.4 Button Press Flow

1. Service reads hardware input.
2. Edge detection identifies pressed button index.
3. Service broadcasts `MacroPadInputMessage { device_id, instance_id, button_index, pressed }` on `service.macropad.input`.
4. `LauncherHost::route_message()` matches `service.macropad.input`, routes to the `LauncherInstance` with matching `instance_id` in the existing `instances`
   HashMap.
5. Instance maps button index to plugin, calls plugin's `on_message()` with simulated click event.
6. Plugin executes `click_topic` / `click_payload` / `click_instance` — exactly as a GTK button would.

### 8.5 Cross-Instance Communication

- **GTK → MacroPad**: Button with `click_instance = "macropad_1"` sends message to headless instance.
- **MacroPad → GTK**: Button with `click_instance = "main"` sends message to GTK launcher.
- **Area addressing**: `area.macropad_1:app_launcher.open` opens area on `macropad_1`.

All cross-instance messaging uses the existing broker routing — no changes needed.

### 8.6 MacroPad Config Example

```toml
# config-macropad-1.toml
[launcher]
instance_id = "macropad_1"

[[areas]]
id = "main"
area_type = "fixed"
width = 432

[[areas.plugins]]
id = "button_apps"
path = "target/release/libsmearor_button_widget.so"
text = "Apps"
icon = "nf-md-apps"
click_topic = "area.app_launcher.open"
click_instance = "macropad_1"

[[areas.plugins]]
id = "button_weather"
path = "target/release/libsmearor_button_widget.so"
text = "Weather"
icon = "nf-md-weather-partly-cloudy"
click_topic = "area.weather.open"
click_instance = "main"
```

---

## 9. MCP Tools

Each MacroPad service registers its own MCP tools, namespaced by driver name:

### 9.1 Stream Deck MCP Tools

| Tool Name                   | Description            | Arguments                                                    |
|-----------------------------|------------------------|--------------------------------------------------------------|
| `streamdeck_get_devices`    | List connected devices | `{}`                                                         |
| `streamdeck_set_brightness` | Set device brightness  | `{ "device": "serial_or_instance_id", "brightness": 0-100 }` |
| `streamdeck_clear_button`   | Clear button image     | `{ "device": "...", "button": 0-14 }`                        |
| `streamdeck_clear_all`      | Clear all buttons      | `{ "device": "..." }`                                        |
| `streamdeck_trigger_button` | Simulate button press  | `{ "device": "...", "button": 0-14 }`                        |

### 9.2 Loupedeck MCP Tools

| Tool Name                  | Description            | Arguments                                                    |
|----------------------------|------------------------|--------------------------------------------------------------|
| `loupedeck_get_devices`    | List connected devices | `{}`                                                         |
| `loupedeck_set_brightness` | Set device brightness  | `{ "device": "serial_or_instance_id", "brightness": 0-100 }` |
| `loupedeck_clear_button`   | Clear button image     | `{ "device": "...", "button": 0-N }`                         |
| `loupedeck_clear_all`      | Clear all buttons      | `{ "device": "..." }`                                        |
| `loupedeck_trigger_button` | Simulate button press  | `{ "device": "...", "button": 0-N }`                         |

Registration via `RegisterToolMessage`, handling via `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` — same pattern as `WeatherService`.

---

## 10. Implementation Phases

### Phase 1: Plugin-API Extension — `GraphicRenderer` Trait

**Order**: First. All other phases depend on this.

**Changes**:

- Add `FfiGraphic` and `GraphicRenderer` to `plugin-api/src/widget.rs`.
- Extend `PluginVTable` with `render_graphic: Option<...>`.
- Increment `PLUGIN_VTABLE_VERSION` to `2`.
- Add `image` dependency to `plugin-api`.

**Exit Criteria**: Trait compiles, existing widgets still load with VTable fallback, `FfiGraphic` converts to/from `image::RgbaImage`.

### Phase 2: Button Widget — `GraphicRenderer` Implementation

**Order**: After Phase 1.

**Changes**:

- Add `image`, `ab_glyph`, `imageproc` to `plugins/button/Cargo.toml`.
- Implement `GraphicRenderer` for `ButtonWidget`.
- Rendering: background fill, NerdFont icon rasterization, label text, state evaluation.
- Must handle arbitrary dimensions (not hardcoded 72×72).

**Exit Criteria**: `render_graphic(w, h)` produces valid `FfiGraphic` for any reasonable w/h. Pure Rust, no GTK calls.

### Phase 3: Model Crate — `model/macropad`

**Order**: After Phase 1.

**Changes**:

- Create `model/macropad/` with generic message types, topics, `register_json_converters()`.
- All FFI types `#[stabby::stabby]`.
- Add to workspace `Cargo.toml`.

**Exit Criteria**: Crate compiles, exports all types, `register_json_converters()` callable.

### Phase 4: Stream Deck Service — `services/streamdeck`

**Order**: After Phase 3.

**Changes**:

- Create `services/streamdeck/` with `elgato-streamdeck`, `hidapi`, `image`, `tokio`.
- Implement `StreamDeckService` with all traits.
- Multi-device discovery, event loop, edge detection, auto-reconnect.
- Broadcast generic `MacroPadConnectionStatus` / `MacroPadInputMessage` on `service.macropad.*` topics.
- Handle `MacroPadCommandMessage` for brightness, button images, clearing.
- MCP tool registration and handling.
- Add to workspace `Cargo.toml` and `services.toml`.

**Exit Criteria**: Service discovers all 3 Stream Deck devices, button presses broadcast `MacroPadInputMessage`, brightness/image commands work, MCP tools
invocable.

### Phase 5: MacroPad Instance — Host Integration

**Order**: After Phase 4 and Phase 2. Depends on DYNAMIC_LOAD concept (`DYNAMIC_LOAD_LAUNCHER_INSTANCE.md`) being implemented first.

**Changes**:

- Add `MacroPadDeviceMetadata` struct and `device_metadata: Option<MacroPadDeviceMetadata>` field to `LauncherInstance` in `instance.rs`.
- `LauncherHost::route_message()` handles `service.macropad.connection`:
    - On `Connected`: calls `load_instance(instance_id, config_path, InstanceType::Headless)`, then attaches `MacroPadDeviceMetadata`.
    - On `Disconnected`: calls `stop_instance(instance_id)`.
- `LauncherHost::route_message()` handles `service.macropad.input`: routes to the `LauncherInstance` with matching `instance_id` in the existing `instances`
  HashMap.
- Instance maps button indices to plugins, calls `render_graphic()`, writes images via service.
- Config loading uses the same `validate_config_path()` / `validate_instance_id()` from DYNAMIC_LOAD.

**Exit Criteria**: Physical button press triggers configured action, button images displayed, cross-instance messaging works, state updates trigger
re-rendering, device disconnect removes instance automatically.

### Phase 6: Loupedeck Service — `services/loupedeck`

**Order**: After Phase 5 (host already supports generic MacroPad instances).

**Changes**:

- Create `services/loupedeck/` with `loupedeck-driver`, `image`, `tokio`.
- Implement `LoupedeckService` with all traits, analogous to `StreamDeckService`.
- Broadcast same generic `MacroPadConnectionStatus` / `MacroPadInputMessage` on `service.macropad.*` topics.
- Handle `MacroPadCommandMessage`.
- MCP tool registration and handling.
- Add to workspace `Cargo.toml` and `services.toml`.

**Exit Criteria**: Loupedeck device connects, button presses broadcast `MacroPadInputMessage`, host creates headless `LauncherInstance` automatically via
`load_instance()`, button images displayed.

### Phase 7: Polish and udev Rules

**Order**: After Phase 6.

**Changes**:

- Add udev rules for both Elgato and Loupedeck devices.
- Document setup in README.
- Error handling, config validation, integration tests.

**Exit Criteria**: All devices work without `sudo`, documentation complete, Stream Deck and Loupedeck devices operate simultaneously.

---

## 11. Dependencies

### New Workspace Dependencies

```toml
elgato-streamdeck = "0.13"
loupedeck-driver = "0.1"
hidapi = "2.6"
ab_glyph = "0.2"
imageproc = "0.25"
```

### Per-Crate

| Crate                 | Additional Dependencies                                                                                                                                                         |
|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `plugin-api`          | `image`                                                                                                                                                                         |
| `plugins/button`      | `image`, `ab_glyph`, `imageproc`                                                                                                                                                |
| `model/macropad`      | `serde`, `serde_json`, `stabby`, `smearor-swipe-launcher-plugin-api`                                                                                                            |
| `services/streamdeck` | `elgato-streamdeck`, `hidapi`, `image`, `tokio`, `stabby`, `serde`, `serde_json`, `smearor-model-macropad`, `smearor-model-mcp`, `smearor-swipe-launcher-plugin-api`, `tracing` |
| `services/loupedeck`  | `loupedeck-driver`, `image`, `tokio`, `stabby`, `serde`, `serde_json`, `smearor-model-macropad`, `smearor-model-mcp`, `smearor-swipe-launcher-plugin-api`, `tracing`            |

---

## 12. udev Rules

File: `40-macropad.rules`

```
# Elgato Stream Deck Original V2
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="006d", MODE="0666"
# Elgato Stream Deck MK.2
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0080", MODE="0666"
# Loupedeck devices (VID 2ec2)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="2ec2", MODE="0666"
```

Install: `sudo cp 40-macropad.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules && sudo udevadm trigger`

---

## 13. Open Questions

1. **Config format**: Separate TOML files (`config-macropad-1.toml`) or inline in main config under `[macropad.instances]`?
2. **Button index mapping**: Positional (plugins in area order → indices 0, 1, 2, ...) or explicitly configurable per plugin?
3. **Re-render debouncing**: Should state-triggered re-renders be debounced during rapid state changes?
4. **Font rendering**: Is `ab_glyph` sufficient for NerdFont glyph rasterization, or is `rusttype` preferred?
5. **Loupedeck knob/touch support**: Should the initial Loupedeck service support only buttons, or also rotary encoders and touchscreens?
6. **Mixed-device layouts**: Can a single `MacroPadInstance` config be shared across devices with different key resolutions (e.g. 72px Stream Deck + 90px
   Loupedeck)?
