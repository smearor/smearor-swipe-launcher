# Macro Pad Models

This document provides an overview of macro pad hardware, the available models across three major manufacturers, and how they integrate with the Smearor Swipe
Launcher.

---

## What Are Macro Pads?

Macro pads are compact peripheral input devices designed for fast, tactile access to frequently used actions. Each key or pad is typically paired with a small
LCD display that can show a custom icon, label, or animation. Pressing a key triggers a configurable action — launching an application, toggling a setting,
executing a command, or switching to a sub-menu of further actions.

Macro pads are popular among streamers, video editors, developers, and power users who want a physical, visual shortcut panel alongside their keyboard. The key
displays allow context-sensitive labeling: the same button can show different icons and perform different actions depending on the active application or current
state.

### Core Concepts

- **LCD keys**: Physical buttons with an embedded LCD screen. Software renders custom icons or text to each key individually.
- **Dial/knob encoders**: Rotatable controls that can be turned (for incremental adjustments like volume) and pressed (for a click action).
- **Touch elements**: Capacitive touch strips or touchscreens that support tap, swipe, or multi-touch gestures.
- **Sub-menus**: Many macro pad software suites allow nesting — pressing a button can switch the entire key layout to a new set of actions, similar to a folder
  structure.
- **Brightness control**: Adjustable backlight brightness for the key displays.

---

## Available Models

### Elgato Stream Deck

Elgato's Stream Deck line is the most widely recognized macro pad family. All models use LCD keys that display custom icons rendered by the host software. The
models differ primarily in key count, physical form factor, and additional input elements.

| Model               | LCD Keys     | Dials          | Touch Elements | Additional Displays     |
|---------------------|--------------|----------------|----------------|-------------------------|
| **Mini**            | 6            | —              | —              | —                       |
| **Neo**             | 8            | —              | 2 touch points | 1 infobar               |
| **MK.2 / Standard** | 15           | —              | —              | —                       |
| **XL**              | 32           | —              | —              | —                       |
| **Stream Deck +**   | 8            | 4 (with click) | 1 touch strip  | 1 touch display         |
| **Pedal**           | 3 *(pedals)* | —              | —              | —                       |
| **Studio**          | 32           | 2 (with click) | NFC sensor     | 1 infobar + 2 LED rings |

**Key characteristics**:

- **Mini**: Entry-level, 6 keys in a 2×3 grid. Compact and portable. No extra inputs.
- **Neo**: 8 keys with a small infobar display for status text. Two touch-sensitive areas above the keys for swipe gestures.
- **MK.2 / Standard**: The classic 15-key model in a 3×5 grid. The most common Stream Deck. No extra inputs.
- **XL**: 32 keys in a 4×8 grid. Designed for users who need many actions visible simultaneously.
- **Stream Deck +**: 8 LCD keys combined with 4 clickable dials and a central touch strip plus a touch display. Designed for adjustments that benefit from
  analog rotation (volume, scrubbing, brightness).
- **Pedal**: Three foot pedals instead of LCD keys. Hands-free action triggering. No visual feedback on the device itself.
- **Studio**: 32 keys (like the XL) plus 2 clickable dials, an NFC sensor for tap-to-trigger tags, an infobar, and two LED rings around the dials. Aimed at
  professional production environments.

### Loupedeck

Loupedeck devices originated as dedicated editing controllers for Lightroom and Premiere Pro. They combine touch-sensitive buttons, physical dials, and
dedicated physical keys. All models include a central touchscreen with dial strips.

| Model                | Touch Buttons | Dials            | Physical Keys | Touch Displays / Special              |
|----------------------|---------------|------------------|---------------|---------------------------------------|
| **Loupedeck Live S** | 15            | 2 (with click)   | 4 (RGB)       | 1× touchscreen (with dial strips)     |
| **Loupedeck Live**   | 12            | 6 (with click)   | 8 (RGB)       | 1× touchscreen (with dial strips)     |
| **Loupedeck CT**     | 12            | 6 (with click)   | 20 (RGB)      | 1× touchscreen + 1× round touchscreen |
| **Loupedeck+**       | —             | 14 (dials/wheel) | 25+           | No displays (status LEDs only)        |

**Key characteristics**:

- **Live S**: Entry-level model with 15 touch buttons in a 3×5 grid, 2 clickable dials, and 4 RGB-backlit physical keys. The central touchscreen includes
  interactive dial strips.
- **Live**: 12 touch buttons, 6 clickable dials, and 8 RGB physical keys. The touchscreen provides additional context-sensitive controls with dial strips.
- **CT**: The flagship model. 12 touch buttons, 6 clickable dials, 20 RGB physical keys, a main touchscreen, and a distinctive round touchscreen that can
  function as a navigation wheel or color selector.
- **Loupedeck+**: The original model. No touch buttons or displays — relies on 14 dials (including a large wheel) and 25+ physical keys with status LEDs.
  Designed for photo editing workflows.

### Razer

Razer's Stream Controller line rebranded and evolved from the Loupedeck Live form factor, adding Razer's Chroma RGB ecosystem and dual-side LCD panels.

| Model                   | LCD Touch Buttons | Physical Click-LCD Buttons | Dials (with click) | Extra Keys | Displays                      |
|-------------------------|-------------------|----------------------------|--------------------|------------|-------------------------------|
| **Stream Controller**   | 12                | —                          | 6                  | 8 (RGB)    | 1× touchscreen + 2× side LCDs |
| **Stream Controller X** | —                 | 15                         | —                  | —          | 15× individual LCDs (in keys) |

**Key characteristics**:

- **Stream Controller**: 12 capacitive touch buttons, 6 clickable dials, 8 Chroma RGB physical keys, a central touchscreen, and two LCD panels on the left and
  right sides for additional context display. The most input-rich device in this category.
- **Stream Controller X**: 15 physical buttons, each with its own embedded LCD (similar to a Stream Deck MK.2). No dials, no touch buttons, no extra displays —
  a streamlined, key-only design.

### Comparison Summary

| Feature         | Stream Deck                   | Loupedeck                    | Razer                             |
|-----------------|-------------------------------|------------------------------|-----------------------------------|
| LCD keys        | Yes (all models except Pedal) | Touch buttons (not LCD keys) | SC X: LCD keys; SC: touch buttons |
| Clickable dials | SD+, Studio                   | All models except Loupedeck+ | Stream Controller only            |
| Physical keys   | Pedal only                    | Live S, Live, CT, Loupedeck+ | SC (8 RGB), SC X (15 LCD)         |
| Touchscreen     | SD+, Neo (infobar)            | Live S, Live, CT             | Stream Controller                 |
| RGB lighting    | —                             | Physical keys (RGB)          | Chroma RGB (keys)                 |
| NFC             | Studio only                   | —                            | —                                 |
| Pedal input     | Pedal only                    | —                            | —                                 |

---

## Integration with Smearor Swipe Launcher

The Smearor Swipe Launcher treats macro pads as **headless launcher instances** — full launcher instances that run without a GTK window. Instead of rendering
widgets to a screen, the launcher renders each widget's graphic to a pixel buffer and sends it to the macro pad device for display on its keys. Button presses
on the device are routed back to the launcher as tool invocations.

This design means macro pads share the same plugin, area, and message-broker architecture as the on-screen launcher, with adaptations for the hardware form
factor.

### Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Smearor Swipe Launcher                        │
│                                                                      │
│  ┌──────────────┐   ┌──────────────────┐   ┌────────────────────┐   │
│  │ GTK Instance │   │ Headless Instance│   │ Web Instance       │   │
│  │ (on-screen)  │   │ (MacroPad)       │   │ (browser)          │   │
│  └──────┬───────┘   └────────┬─────────┘   └─────────┬──────────┘   │
│         │                    │                        │              │
│         └────────────────────┼────────────────────────┘              │
│                              │                                       │
│                    ┌─────────┴──────────┐                            │
│                    │   Message Broker   │                            │
│                    └─────────┬──────────┘                            │
│                              │                                       │
│         ┌────────────────────┼────────────────────┐                  │
│         │                    │                    │                  │
│  ┌──────┴───────┐   ┌───────┴────────┐  ┌───────┴────────┐         │
│  │ StreamDeck   │   │ Loupedeck      │  │ Other services │         │
│  │ Service      │   │ Service        │  │                │         │
│  └──────┬───────┘   └───────┬────────┘  └────────────────┘         │
│         │                   │                                        │
└─────────┼───────────────────┼────────────────────────────────────────┘
          │                   │
     ┌────┴────┐        ┌────┴────┐
     │ Stream  │        │ Loupedeck│
     │ Deck    │        │ Device   │
     │ Device  │        │          │
     └─────────┘        └──────────┘
```

### How It Works

1. **Device discovery**: A macro pad service (e.g. `streamdeck`, `loupedeck`) discovers connected hardware via USB HID or serial port enumeration.
2. **Connection broadcast**: The service broadcasts a `MacroPadConnectionStatus` message on the `service.macropad.connection` topic, including the device ID,
   key count, key pixel dimensions, and driver name.
3. **Instance creation**: The host receives the connection status and loads a **headless instance** from a config file (e.g.
   `configs/launcher/<instance_id>.toml`). The instance is built without a GTK window via `build_headless()`, which sets up the area manager and loads all
   plugins from the config.
4. **Button rendering**: The host calls `render_graphic(key_width, key_height)` on each plugin in the currently visible area. The returned RGBA pixel buffer is
   sent as a `SetButtonImage` command to the macro pad service, which forwards it to the physical device.
5. **Input handling**: When a button is pressed, the macro pad service sends a `MacroPadInputMessage` on the `service.macropad.input` topic. The host measures
   press duration to distinguish **click** (< 500 ms) from **longpress** (>= 500 ms), then dispatches an `InvokeToolMessage` with the corresponding action to
   the plugin at that button index.
6. **Area switching**: If a button's action opens or closes a sub-area (e.g. `area.open`), the host re-renders all buttons to reflect the new visible area's
   plugins. Stale buttons beyond the new plugin count are cleared.
7. **Disconnection**: When the device is disconnected, the host stops the headless instance and cleans up.

### Similarities with the On-Screen Launcher

- **Same plugin system**: Macro pad instances load the same widget plugins (`.so` files) as GTK instances. Plugins are shared across instance types.
- **Same area model**: Areas, sub-menus, and the area manager work identically. A macro pad config defines areas with plugins just like the on-screen config.
- **Same message broker**: All instances communicate through the same `FfiEnvelope` message broker. A macro pad instance can send messages to services (e.g.
  `service.hyprland.dispatch`) and receive state updates.
- **Same tool invocation**: Button clicks and longpresses are dispatched as `InvokeToolMessage` — the same mechanism used for on-screen button widgets. The
  `click_topic`, `click_payload`, `longpress_topic`, and `longpress_payload` config fields work the same way.
- **Same config format**: Headless instances use the same TOML config structure (`SwipeLauncherConfig`) as GTK instances, including `[defaults.*]` templates,
  area definitions, and per-plugin configuration sections.
- **Same services**: Background services (audio, hyprland, power, weather, etc.) are shared across all instances. A macro pad button can control volume, switch
  workspaces, or query the weather just like an on-screen button.

### Differences from the On-Screen Launcher

| Aspect                 | On-Screen (GTK)                                             | Macro Pad (Headless)                                                                  |
|------------------------|-------------------------------------------------------------|---------------------------------------------------------------------------------------|
| **Rendering**          | GTK widgets rendered to a window via `WidgetBuilder`        | Pixel buffers rendered via `GraphicRenderer::render_graphic()` and sent to the device |
| **Window**             | GTK4 layer-shell window with animations                     | No window; `HeadlessContainer` no-op backend                                          |
| **Input**              | Mouse clicks, touch, scroll via GTK events                  | Physical button presses via `MacroPadInputMessage`                                    |
| **Press detection**    | GTK gesture handling (click, longpress)                     | Host measures press duration manually (500 ms threshold)                              |
| **Display size**       | Arbitrary window dimensions                                 | Fixed key dimensions (e.g. 72×72 px for Stream Deck, 90×90 px for Loupedeck CT)       |
| **Layout**             | Flexible GTK layout with scroll bands, overlays, animations | Linear button grid; plugins map to button indices in visible-area order               |
| **Visual feedback**    | CSS styling, transitions, animations                        | Static pixel buffer per button; updated on state/area change                          |
| **Multi-view widgets** | GTK popovers, revealers, dialogs                            | View switching via `longpress` action; each view renders a different pixel buffer     |
| **Touch/scroll**       | Full touch and scroll support                               | Not supported (hardware limitation; only button press/release)                        |
| **Dials/knobs**        | Not applicable                                              | Received as events but not yet routed to plugins (future work)                        |
| **Instance lifecycle** | Created at startup from `config.toml`                       | Created dynamically on device connection; stopped on disconnection                    |
| **Config file**        | `configs/launcher/config.toml` (main instance)              | `configs/launcher/<instance_id>.toml` (per device)                                    |

### Supported Drivers

The launcher currently includes two macro pad driver services:

- **Stream Deck** (`services/streamdeck`): Uses the `elgato-streamdeck` crate for USB HID communication. Supports all Stream Deck models that the underlying
  library recognizes. Renders RGBA pixel data and resizes to each key's native resolution.
- **Loupedeck** (`services/loupedeck`): Uses the `loupedeck-driver` crate for serial communication. Supports Loupedeck Live, Live S, and CT models. Converts
  RGBA pixel data to RGB565 format for the device's display.

Both drivers implement the same `MacroPadCommand` protocol (set brightness, clear buttons, set button image, reset) and emit the same `MacroPadInputMessage`
events. The host treats them identically — the driver name is only used to route commands to the correct service.

### Configuration

A macro pad service is registered in `services.toml`:

```toml
[[services]]
id = "streamdeck"
path = "target/release/libsmearor_streamdeck_service.so"

[streamdeck]
brightness = 50
poll_interval_ms = 50
```

A headless instance config (e.g. `configs/launcher/streamdeck.toml`) uses the same format as the main launcher config:

```toml
areas = ["scroll_band"]

[scroll_band]
area_type = "scroll"
plugins = [
    { id = "previous_workspace", path = "target/release/libsmearor_button_widget.so" },
    { id = "toggle_floating", path = "target/release/libsmearor_button_widget.so" },
    { id = "close_active_window", path = "target/release/libsmearor_button_widget.so" },
]

[previous_workspace]
defaults = "menu_button"
text = "Prev WS"
icon = "nf-md-skip_previous"
click_topic = "service.hyprland.dispatch"
click_payload = { kind = "Workspace", identifier = { kind = "Relative", id = -1 } }
```

The number of plugins in the visible area determines how many buttons are rendered. If the config defines more plugins than the device has keys, excess plugins
are ignored. If the config defines fewer plugins, remaining keys are cleared.

### Message Topics

| Topic                         | Direction      | Purpose                                                         |
|-------------------------------|----------------|-----------------------------------------------------------------|
| `service.macropad.connection` | Service → Host | Device connected/disconnected with metadata                     |
| `service.macropad.input`      | Service → Host | Button press/release events                                     |
| `service.macropad.command`    | Host → Service | Commands: set brightness, set button image, clear button, reset |
