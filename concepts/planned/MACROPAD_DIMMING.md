# Concept: Configurable Auto-Dimming for MacroPad Devices

This document defines the concept for **per-device configurable auto-dimming** with **smooth fading** on MacroPad devices (StreamDeck and Loupedeck). When no
user interaction occurs for a configurable idle period, the device brightness gradually fades to a dimmed level. Any button press immediately restores the full
brightness with a smooth fade.

---

## 1. Problem Statement

### 1.1 Current State

Both MacroPad services (`services/streamdeck`, `services/loupedeck`) set a fixed brightness at startup from their respective config
(`StreamDeckConfig.brightness`, `LoupedeckConfig.brightness`). The brightness can be changed at runtime via:

- `MacroPadCommand::set_brightness(percent)` — sent via `TOPIC_MACROPAD_COMMAND`
- MCP tools `streamdeck_set_brightness` and `loupedeck_set_brightness` — sent via `TOPIC_MCP_INVOKE_TOOL`

Both paths result in a `DeviceCommand::SetBrightness(u8)` being forwarded to the per-device event loop (`device_event_loop`), which calls
`device.set_brightness()` directly.

The current architecture:

```
┌─────────────────────────────────────────────────────────────────────┐
│  run_device_loop                                                    │
│  (discovers devices, spawns one thread per device)                  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  device_event_loop (per device)                               │  │
│  │                                                               │  │
│  │  tokio::select! {                                             │  │
│  │    command = command_receiver.recv() => {                     │  │
│  │      SetBrightness(percent) => device.set_brightness(percent) │  │
│  │      ... other commands ...                                   │  │
│  │    }                                                          │  │
│  │    _ = tokio::time::sleep(poll_duration) => {                 │  │
│  │      // read button events                                    │  │
│  │    }                                                          │  │
│  │  }                                                            │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 What Is Missing

- **No idle detection**: The device stays at full brightness indefinitely, even when not in use.
- **No auto-dimming**: There is no mechanism to reduce brightness after an idle period.
- **No smooth transitions**: Brightness changes are instant — no fading.
- **No per-device configuration**: Brightness config is service-wide, not per individual device.
- **Inconsistent brightness scales**: StreamDeck uses 0-100, Loupedeck uses 0-10. The MCP tools already accept 0-100, but the Loupedeck config still uses 0-10.

---

## 2. Goals

- **Per-device auto-dimming**: Each connected device can be individually configured with its own dimming behaviour.
- **Smooth fading**: Brightness transitions (both dimming and restoring) happen gradually in configurable step sizes and intervals.
- **Button-press restores brightness**: Any button press on the device immediately cancels dimming and fades back to the user's target brightness.
- **Unified 0-100 scale**: Both StreamDeck and Loupedeck use a 0-100 brightness scale in configuration and MCP. Loupedeck internally clamps to its 0-10 hardware
  scale.
- **Non-blocking**: Dimming runs inside the existing `device_event_loop` `tokio::select!` — no extra threads or channels.
- **Configurable**: All dimming parameters are optional with sensible defaults.

## 3. Non-Goals

- **Ambient light sensing**: No hardware light sensor integration. Dimming is purely time-based.
- **Per-button dimming**: Dimming applies to the entire device, not individual buttons.
- **MCP tool for dimming control**: Auto-dimming is config-only. Runtime toggling via MCP is out of scope (can be added later if needed).
- **Cross-device coordination**: Each device dims independently based on its own idle timer.
- **Dimming on GTK or Web instances**: This concept applies only to physical MacroPad devices (StreamDeck, Loupedeck).

---

## 4. Architecture

### 4.1 Unified Brightness Scale

Both services accept brightness as 0-100 in all configuration and MCP paths. The Loupedeck service internally maps 0-100 to its 0-10 hardware scale:

```rust
fn scale_to_loupedeck(percent: u8) -> u8 {
    (percent as u16 * 10 / 100) as u8
}
```

This mapping is applied at the point where `device.set_brightness()` is called — not at the config or MCP layer. The `LoupedeckConfig.brightness` field changes
from 0-10 to 0-100, with a default of 50 (matching StreamDeck).

### 4.2 Per-Device Configuration

Auto-dimming parameters are part of the service config but apply per device. A `device_overrides` map allows per-device configuration keyed by serial number:

```toml
# Default for all devices
brightness = 50
auto_dimming_enabled = true
auto_dim_timeout_ms = 30000
auto_dim_brightness = 5
auto_dim_fade_step_ms = 50
auto_dim_fade_step_percent = 5

# Per-device overrides
[[device_overrides]]
serial = "CL21G1A12345"
brightness = 80
auto_dimming_enabled = false

[[device_overrides]]
serial = "AL32G2B67890"
auto_dim_timeout_ms = 60000
auto_dim_brightness = 10
```

### 4.3 Dimming State Machine

Each device's `device_event_loop` maintains a `DimmingState`:

```rust
/// Per-device dimming state.
struct DimmingState {
    /// Whether auto-dimming is enabled for this device.
    enabled: bool,
    /// Target brightness when active (0-100).
    target_brightness: u8,
    /// Dimmed brightness when idle (0-100).
    dim_brightness: u8,
    /// Idle timeout before dimming starts.
    idle_timeout: std::time::Duration,
    /// Fade step interval.
    fade_step_duration: std::time::Duration,
    /// Brightness change per fade step.
    fade_step_percent: u8,
    /// Current brightness (may differ from target during fading).
    current_brightness: u8,
    /// Last activity timestamp.
    last_activity: tokio::time::Instant,
    /// Current dimming phase.
    phase: DimmingPhase,
}

/// Dimming phases.
enum DimmingPhase {
    /// Device is active, full brightness.
    Active,
    /// Fading from active to dimmed brightness.
    FadingDown,
    /// Device is dimmed, waiting for activity.
    Dimmed,
    /// Fading from dimmed to active brightness.
    FadingUp,
}
```

### 4.4 Event Loop Integration

The `device_event_loop` `tokio::select!` gains a third branch for the dimming timer:

```
┌──────────────────────────────────────────────────────────────────────┐
│  device_event_loop (per device)                                       │
│                                                                      │
│  DimmingState {                                                       │
│    target_brightness, dim_brightness,                                 │
│    current_brightness, last_activity,                                 │
│    phase: Active | FadingDown | Dimmed | FadingUp                     │
│  }                                                                    │
│                                                                      │
│  loop {                                                               │
│    tokio::select! {                                                   │
│      command = command_receiver.recv() => {                           │
│        SetBrightness(percent) => {                                    │
│          dimming.target_brightness = percent;                         │
│          dimming.last_activity = now();                               │
│          dimming.phase = FadingUp;                                    │
│          // fade loop handles gradual change                          │
│        }                                                              │
│        ... other commands ...                                         │
│      }                                                                │
│      _ = tokio::time::sleep(poll_duration) => {                       │
│        // read button events                                          │
│        if button_pressed {                                            │
│          dimming.last_activity = now();                               │
│          if dimming.phase != Active {                                 │
│            dimming.phase = FadingUp;                                  │
│          }                                                            │
│        }                                                              │
│      }                                                                │
│      _ = dimming_timer => {                                           │
│        match dimming.phase {                                          │
│          Active => {                                                  │
│            if elapsed > idle_timeout {                                │
│              dimming.phase = FadingDown;                              │
│            }                                                          │
│          }                                                            │
│          FadingDown => {                                              │
│            dimming.current_brightness -= fade_step;                   │
│            device.set_brightness(dimming.current_brightness);         │
│            if dimming.current_brightness <= dim_brightness {          │
│              dimming.phase = Dimmed;                                  │
│            }                                                          │
│          }                                                            │
│          Dimmed => {                                                  │
│            // wait for activity (timer fires periodically)            │
│          }                                                            │
│          FadingUp => {                                                │
│            dimming.current_brightness += fade_step;                   │
│            device.set_brightness(dimming.current_brightness);         │
│            if dimming.current_brightness >= target_brightness {       │
│              dimming.phase = Active;                                  │
│            }                                                          │
│          }                                                            │
│        }                                                              │
│      }                                                                │
│    }                                                                  │
│  }                                                                    │
└──────────────────────────────────────────────────────────────────────┘
```

### 4.5 Dimming Timer

The dimming timer fires at the `fade_step_duration` interval (default 50 ms). On each tick:

- **Active**: Check if `now - last_activity > idle_timeout`. If so, transition to `FadingDown`.
- **FadingDown**: Decrease `current_brightness` by `fade_step_percent`. Call `device.set_brightness()`. If `current_brightness <= dim_brightness`, transition to
  `Dimmed`.
- **Dimmed**: No action (wait for button press or command).
- **FadingUp**: Increase `current_brightness` by `fade_step_percent`. Call `device.set_brightness()`. If `current_brightness >= target_brightness`, transition
  to `Active`.

The timer is always active when auto-dimming is enabled. In `Active` and `Dimmed` phases, the tick is lightweight (just a time check or no-op).

### 4.6 Button Press Handling

When a button press is detected in the polling branch:

1. `last_activity` is updated to `now()`.
2. If `phase` is `Dimmed` or `FadingDown`, transition to `FadingUp`.
3. If `phase` is `Active`, no change needed (already at target brightness).

The fade-up happens gradually via the dimming timer branch. The button event is still processed normally (broadcast to host).

### 4.7 Manual Brightness Override

When a `SetBrightness` command is received (via `MacroPadCommand` or MCP tool):

1. `target_brightness` is updated to the new value.
2. `last_activity` is updated to `now()`.
3. `phase` transitions to `FadingUp` (smooth transition to new target).

This means manual overrides are respected and the device smoothly transitions to the new brightness.

### 4.8 Loupedeck Brightness Scaling

The Loupedeck service applies the 0-100 → 0-10 mapping at the `device.set_brightness()` call site:

```rust
// In device_event_loop, when setting brightness:
let hardware_brightness = scale_to_loupedeck(dimming.current_brightness);
if let Err(e) = device.set_brightness(hardware_brightness) {
error ! ("Loupedeck service: set_brightness failed for {}: {e}", serial);
}
```

All internal dimming logic operates on the 0-100 scale. Only the final hardware call is scaled.

---

## 5. Configuration

### 5.1 StreamDeckConfig

```rust
/// Configuration for the Stream Deck service.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDeckConfig {
    /// Polling interval for reading button states in milliseconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    /// Initial brightness (0-100).
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    /// Whether to enable MCP tool registration for this service.
    #[serde(default = "default_mcp_enabled")]
    pub mcp_enabled: bool,
    /// Whether auto-dimming is enabled for all devices by default.
    #[serde(default = "default_auto_dimming_enabled")]
    pub auto_dimming_enabled: bool,
    /// Idle timeout in milliseconds before dimming starts.
    #[serde(default = "default_auto_dim_timeout_ms")]
    pub auto_dim_timeout_ms: u64,
    /// Dimmed brightness level (0-100).
    #[serde(default = "default_auto_dim_brightness")]
    pub auto_dim_brightness: u8,
    /// Fade step interval in milliseconds.
    #[serde(default = "default_auto_dim_fade_step_ms")]
    pub auto_dim_fade_step_ms: u64,
    /// Brightness change per fade step (in percent points).
    #[serde(default = "default_auto_dim_fade_step_percent")]
    pub auto_dim_fade_step_percent: u8,
    /// Per-device configuration overrides.
    #[serde(default)]
    pub device_overrides: Vec<DeviceOverride>,
}
```

### 5.2 LoupedeckConfig

Same fields as StreamDeckConfig. The `brightness` and `auto_dim_brightness` fields use the 0-100 scale (internally mapped to 0-10).

### 5.3 DeviceOverride

```rust
/// Per-device configuration override.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceOverride {
    /// Device serial number.
    pub serial: String,
    /// Initial brightness (0-100). If omitted, uses service default.
    pub brightness: Option<u8>,
    /// Whether auto-dimming is enabled for this device. If omitted, uses service default.
    pub auto_dimming_enabled: Option<bool>,
    /// Idle timeout in milliseconds. If omitted, uses service default.
    pub auto_dim_timeout_ms: Option<u64>,
    /// Dimmed brightness level (0-100). If omitted, uses service default.
    pub auto_dim_brightness: Option<u8>,
    /// Fade step interval in milliseconds. If omitted, uses service default.
    pub auto_dim_fade_step_ms: Option<u64>,
    /// Brightness change per fade step. If omitted, uses service default.
    pub auto_dim_fade_step_percent: Option<u8>,
}
```

### 5.4 Default Values

| Field                        | Default | Description                    |
|------------------------------|---------|--------------------------------|
| `poll_interval_ms`           | 50      | Polling interval               |
| `brightness`                 | 50      | Initial brightness (0-100)     |
| `mcp_enabled`                | true    | MCP tool registration          |
| `auto_dimming_enabled`       | true    | Auto-dimming on by default     |
| `auto_dim_timeout_ms`        | 30000   | 30 seconds idle before dimming |
| `auto_dim_brightness`        | 5       | Dimmed to 5% brightness        |
| `auto_dim_fade_step_ms`      | 50      | 50 ms per fade step (20 fps)   |
| `auto_dim_fade_step_percent` | 5       | 5% brightness change per step  |

### 5.5 TOML Example

```toml
# StreamDeck service config
brightness = 60
auto_dimming_enabled = true
auto_dim_timeout_ms = 30000
auto_dim_brightness = 5
auto_dim_fade_step_ms = 50
auto_dim_fade_step_percent = 5

[[device_overrides]]
serial = "CL21G1A12345"
brightness = 80
auto_dimming_enabled = false

[[device_overrides]]
serial = "CL21G1A67890"
auto_dim_timeout_ms = 60000
auto_dim_brightness = 10
auto_dim_fade_step_percent = 3
```

---

## 6. Implementation Phases

### Phase 1: Unified Brightness Scale

| Task                                         | File                                         | Description                                                |
|----------------------------------------------|----------------------------------------------|------------------------------------------------------------|
| Change `LoupedeckConfig.brightness` to 0-100 | `services/loupedeck/src/config.rs`           | Default 50, add `scale_to_loupedeck()` helper              |
| Apply scaling in `run_device_loop`           | `services/loupedeck/src/service.rs`          | Scale `config.brightness` before `device.set_brightness()` |
| Apply scaling in `device_event_loop`         | `services/loupedeck/src/service.rs`          | Scale `percent` in `SetBrightness` command handler         |
| Update MCP tool description                  | `services/loupedeck/src/mcp/capabilities.rs` | Remove "0-10 scale" mention from tool description          |

### Phase 2: Dimming Infrastructure

| Task                        | File                                                                      | Description                                                                                                                                     |
|-----------------------------|---------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| Add `DeviceOverride` struct | `services/streamdeck/src/config.rs`, `services/loupedeck/src/config.rs`   | Per-device override struct                                                                                                                      |
| Add dimming config fields   | `services/streamdeck/src/config.rs`, `services/loupedeck/src/config.rs`   | `auto_dimming_enabled`, `auto_dim_timeout_ms`, `auto_dim_brightness`, `auto_dim_fade_step_ms`, `auto_dim_fade_step_percent`, `device_overrides` |
| Add `DimmingState` struct   | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Per-device dimming state machine                                                                                                                |
| Add `DimmingPhase` enum     | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Active, FadingDown, Dimmed, FadingUp                                                                                                            |

### Phase 3: Event Loop Integration

| Task                                       | File                                                                      | Description                                                 |
|--------------------------------------------|---------------------------------------------------------------------------|-------------------------------------------------------------|
| Resolve per-device config                  | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Merge `device_overrides` with defaults in `run_device_loop` |
| Pass `DimmingState` to `device_event_loop` | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Initialise state per device                                 |
| Add dimming timer branch to `select!`      | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Third `tokio::select!` branch                               |
| Reset idle timer on button press           | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Update `last_activity` and transition to `FadingUp`         |
| Handle `SetBrightness` with dimming        | `services/streamdeck/src/service.rs`, `services/loupedeck/src/service.rs` | Update `target_brightness`, transition to `FadingUp`        |
| Apply Loupedeck scaling in dimming         | `services/loupedeck/src/service.rs`                                       | Scale `current_brightness` before `device.set_brightness()` |

### Phase 4: Testing & Verification

| Task                                 | Description                                                              |
|--------------------------------------|--------------------------------------------------------------------------|
| Unit test `scale_to_loupedeck()`     | Verify 0→0, 50→5, 100→10, 100→10 (clamped)                               |
| Unit test `DimmingState` transitions | Active→FadingDown→Dimmed→FadingUp→Active                                 |
| Manual test: idle dimming            | Leave device idle, verify smooth fade to dim brightness                  |
| Manual test: button press restore    | Press button while dimmed, verify smooth fade to target                  |
| Manual test: manual override         | Send `SetBrightness` via MCP, verify smooth transition                   |
| Manual test: per-device override     | Configure different timeouts for two devices, verify independent dimming |

---

## 7. File Changes Summary

| File                                         | Change                                                                                                                  |
|----------------------------------------------|-------------------------------------------------------------------------------------------------------------------------|
| `services/streamdeck/src/config.rs`          | Add dimming config fields, `DeviceOverride` struct, default functions                                                   |
| `services/streamdeck/src/service.rs`         | Add `DimmingState`, `DimmingPhase`, dimming timer branch, idle reset on button press, `SetBrightness` override handling |
| `services/loupedeck/src/config.rs`           | Same as streamdeck + change `brightness` default to 50 (0-100 scale)                                                    |
| `services/loupedeck/src/service.rs`          | Same as streamdeck + `scale_to_loupedeck()` helper, apply scaling at hardware call sites                                |
| `services/loupedeck/src/mcp/capabilities.rs` | Update tool description (remove 0-10 mention)                                                                           |

---

## 8. Dependencies

No new external dependencies. All required types (`tokio::time::Instant`, `tokio::time::Duration`, `tokio::select!`) are already available via the existing
`tokio` workspace dependency.

---

## 9. Risks and Considerations

1. **USB Bandwidth**: Frequent `set_brightness()` calls during fading add USB traffic. At 50 ms intervals with 5% steps, a full fade takes 1 second (20 calls).
   This is negligible compared to button image updates (which transfer full pixel buffers).

2. **Device-Specific Behaviour**: Some StreamDeck models may handle rapid brightness changes differently. The fade step interval (50 ms) is conservative enough
   to avoid flickering. If issues arise, the step size and interval are configurable.

3. **Thread Safety**: `DimmingState` is owned exclusively by the per-device `device_event_loop`. No shared state or locking is needed — the dimming timer branch
   and the polling branch both access `DimmingState` within the same `tokio::select!` iteration, which is single-threaded.

4. **Power Cycling**: Rapidly changing brightness on OLED displays (Loupedeck) may cause minor wear. The default fade step of 5% over 1 second is gentle enough
   to avoid concerns.

5. **Config Backward Compatibility**: The Loupedeck `brightness` field changes from 0-10 to 0-100. Existing configs with `brightness = 5` will now set 5%
   instead of 50%. This is a **breaking change** for Loupedeck configs. A migration note should be included in the changelog. Alternatively, a
   `brightness_scale` field could be added for backward compatibility, but this adds complexity for little benefit.

6. **MCP Tool Consistency**: The MCP tools already accept 0-100. After this change, both config and MCP use the same scale, simplifying the mental model.

7. **Dimming During Animation**: If the MacroPad Animation Engine (see `MACRO_PAD_ANIMATIONS_AND_BACKGROUND.md`) is sending frequent `SetButtonImage` commands,
   the dimming timer still runs independently. Button image commands do not reset the idle timer — only button presses and explicit `SetBrightness` commands do.
   This is intentional: animations running without user interaction should still trigger dimming.

---

## 10. Open Questions

1. **Should `SetButtonImage` commands reset the idle timer?** — *Proposed: No. Only physical button presses and explicit brightness commands reset the idle
   timer. Animations running in the background do not count as user activity.*

2. **Should the dimming be pauseable via MCP at runtime?** — *Out of scope for this concept. Can be added later as an MCP tool (e.g.
   `streamdeck_set_auto_dimming` with `enabled` and `timeout_ms` parameters).*

3. **Should there be a "wake on touch" mode that only dims the display but does not process button events?** — *Out of scope. Button events are always
   processed; dimming only affects brightness.*

4. **Should the Loupedeck brightness config migration be automatic?** — *Proposed: No. Document the breaking change. Users update their config manually. The
   default value changes from 5 to 50, which is equivalent.*

5. **Should fading use linear interpolation or an easing curve?** — *Proposed: Linear for simplicity. Easing curves (e.g. ease-in-out) can be added later by
   replacing the constant step with a computed step based on the current position in the fade.*

---

## 11. References

- `services/streamdeck/src/service.rs` — `device_event_loop`, `DeviceCommand`, `run_device_loop`
- `services/loupedeck/src/service.rs` — `device_event_loop`, `DeviceCommand`, `run_device_loop`
- `services/streamdeck/src/config.rs` — `StreamDeckConfig`
- `services/loupedeck/src/config.rs` — `LoupedeckConfig`
- `model/macropad/src/command_message.rs` — `MacroPadCommand::set_brightness()`
- `services/streamdeck/src/mcp/handler/tools.rs` — `streamdeck_set_brightness` MCP tool handler
- `services/loupedeck/src/mcp/handler/tools.rs` — `loupedeck_set_brightness` MCP tool handler
- `concepts/planned/MACRO_PAD_ANIMATIONS_AND_BACKGROUND.md` — Animation engine concept (interacts with dimming)
- `AGENTS.md` — Project conventions (config structs with `parse` method, `tokio` usage, error handling)
