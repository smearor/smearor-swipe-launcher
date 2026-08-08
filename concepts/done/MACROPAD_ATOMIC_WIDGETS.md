# Concept: MacroPad Atomic Widgets

This document defines the concept for **Atomic Widgets** — single-view, single-purpose widget variants that correspond to exactly one view of a Multi-View
Widget. On MacroPad devices, where Swipe-Up and Swipe-Down gestures are not available, Atomic Widgets allow each view to occupy its own dedicated button with
independent Click and Longpress actions. Additionally, this document covers **Multi-Span Widgets** (logical widgets spanning multiple buttons) and new **Input
Triggers** for MacroPad devices.

---

## 1. Problem Statement

### 1.1 Current State

In the Swipe Launcher, Multi-View Widgets (e.g. Weather, Audio, MPRIS) use Swipe-Up and Swipe-Down gestures to switch between views. On a touch table, this is
natural — the user swipes within a single widget area. On a MacroPad, there is no touch surface and no swipe gesture. The only inputs are physical button
presses.

The current MacroPad approach (described in `HEADLESS_WIDGETS_CONCEPT.md`) maps Longpress to `toggle_view`, cycling between Compact and Expanded views on a
single button. This works but has limitations:

- Only one view is visible at a time — the user must remember which view is active.
- Click and Longpress actions are shared across all views on the same button — they cannot be customised per view.
- The user cannot place individual views on different buttons — all views are bound to one button slot.

### 1.2 What Is Missing

- **Atomic Widgets**: Dedicated, single-purpose widget variants that each show exactly one view of a Multi-View Widget. Each Atomic Widget occupies one button
  and has its own Click and Longpress actions.
- **Additional Input Triggers**: Swipe-Up and Swipe-Down are unavailable on MacroPads. New triggers (Click/Release, Longpress, Hold/Push-to-Talk, Double Press)
  are needed to provide equivalent interaction richness.
- **Multi-Span Widgets**: Logical widgets that span two or more physical buttons, enabling wider displays (e.g. volume slider across two buttons) and compound
  actions (e.g. two-button Longpress for mute toggle).

---

## 2. Goals

- Provide **Atomic Widgets** for every Multi-View Widget, allowing each view to be placed on a separate MacroPad button.
- Define **Input Triggers** that replace Swipe-Up/Swipe-Down on MacroPad devices.
- Support **Multi-Span Widgets** that occupy multiple buttons for wider output or compound actions.
- Provide a **File Browser Widget** for directory navigation and file launching. *Moved to `concepts/inprogress/FILE_BROWSER_WIDGET_CONCEPT.md`.*
- Maintain full compatibility with the existing plugin architecture (`widget_plugin!` macro, `GraphicRenderer`, `MessageHandler`, `MessageBroadcaster`).
- Ensure Atomic Widgets share service connections with their parent Multi-View Widget — no duplicate D-Bus or network connections.
- Allow Atomic Widgets to be used in GTK instances as well (e.g. in a sub-area showing all weather split-widgets side by side).

## 3. Non-Goals

- Removing or replacing Multi-View Widgets — they remain the primary widget type for GTK and Web instances.
- Changing the `PluginVTable` structure or the `widget_plugin!` macro.
- Supporting Atomic Widgets on Web instances (Web uses HTML fragments with view switching; Atomic Widgets are primarily for MacroPad and GTK).
- Implementing touch/scroll gestures on MacroPad hardware (hardware limitation).

---

## 4. Input Triggers

Since Swipe-Up and Swipe-Down are not available on MacroPad devices, additional input triggers are required to provide equivalent interaction richness. The host
already distinguishes **click** (press duration < 500 ms) from **longpress** (press duration >= 500 ms). This concept extends the trigger model with two
additional patterns.

### 4.1 Trigger Types

| Trigger                 | Detection                                                                           | Use Case                                                                                                                                               |
|-------------------------|-------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Click (Release)**     | Button is pressed and released within 500 ms; action fires on release               | Default action — most common trigger. Firing on release allows the user to cancel by holding and sliding off the button (if the hardware supports it). |
| **Longpress**           | Button is held for >= 500 ms and then released                                      | Secondary action, view switching, or context menu.                                                                                                     |
| **Hold / Push-to-Talk** | Button is pressed and held; action starts on press and remains active until release | Continuous actions: microphone mute toggle, Push-to-Talk, temporary overlay. The host sends a `start` action on press and a `stop` action on release.  |
| **Double Press**        | Two consecutive clicks within 300 ms                                                | Rarely used or critical commands to prevent accidental triggering (e.g. power off, force kill).                                                        |

### 4.2 Detection Logic

The host's MacroPad input handler (`host/mod.rs`) extends the existing press-duration measurement:

```
1. Button pressed (pressed: true)
   → Record press_start_time
   → If Hold trigger configured: dispatch InvokeToolMessage { action: "hold_start" }

2. Button released (pressed: false)
   → Calculate duration = now - press_start_time
   → If duration < 300 ms:
     → If this is the second press within 300 ms: dispatch "double_press"
     → Else: record as potential first press of a double press, start 300 ms timer
   → If duration < 500 ms (and not double press):
     → dispatch "click"
   → If duration >= 500 ms:
     → dispatch "longpress"
   → If Hold trigger was active: dispatch "hold_stop"
```

### 4.3 Configuration

Each button/widget config can specify which triggers it supports and what actions to dispatch:

```toml
[my_button]
defaults = "menu_button"
text = "Push to Talk"
icon = "nf-md-microphone"

# Click action (fires on release, < 500 ms)
click_topic = "tool.invoke"
click_payload = { tool = "voice_assistant", action = "toggle_listen" }

# Longpress action (fires on release, >= 500 ms)
longpress_topic = "tool.invoke"
longpress_payload = { tool = "voice_assistant", action = "open_overlay" }

# Hold action (fires on press, active until release)
hold_topic = "tool.invoke"
hold_payload = { tool = "voice_assistant", action = "push_to_talk" }

# Double press action (two clicks within 300 ms)
double_press_topic = "tool.invoke"
double_press_payload = { tool = "voice_assistant", action = "force_stop" }
```

If a trigger field is not configured, that trigger is ignored. The host only dispatches triggers that have both a topic and payload defined.

### 4.4 Action Mapping Summary

| Config Field                                  | Trigger           | `InvokeToolMessage` Action |
|-----------------------------------------------|-------------------|----------------------------|
| `click_topic` / `click_payload`               | Click (Release)   | `"click"`                  |
| `longpress_topic` / `longpress_payload`       | Longpress         | `"longpress"`              |
| `hold_topic` / `hold_payload`                 | Hold (on press)   | `"hold_start"`             |
| — (same topic)                                | Hold (on release) | `"hold_stop"`              |
| `double_press_topic` / `double_press_payload` | Double Press      | `"double_press"`           |

---

## 5. Atomic Widgets

### 5.1 Concept

An **Atomic Widget** is a single-view, single-purpose widget that corresponds to exactly one view of a Multi-View Widget. For example, the Weather Multi-View
Widget has views for Today, Tomorrow, Forecast, UV-Index, Sunrise, and Sunset. Each of these views can be extracted into a separate Atomic Widget:

| Multi-View Widget | Atomic Widget    | Description                            |
|-------------------|------------------|----------------------------------------|
| Weather           | Weather-Today    | Current weather: icon + temperature    |
| Weather           | Weather-Tomorrow | Tomorrow's weather: icon + temperature |
| Weather           | Weather-Forecast | Multi-day forecast summary             |
| Weather           | Weather-UV-Index | UV-Index value and risk level          |
| Weather           | Weather-Sunrise  | Sunrise time                           |
| Weather           | Weather-Sunset   | Sunset time                            |

Each Atomic Widget:

- Occupies one MacroPad button.
- Renders exactly one view — no view switching, no `toggle_view` action.
- Has its own Click and Longpress actions, independent of other Atomic Widgets from the same parent.
- Shares the same service connection and state subscription as its parent Multi-View Widget (no duplicate D-Bus or network connections).

### 5.2 Advantages over Multi-View Widgets on MacroPad

| Aspect             | Multi-View Widget (single button)           | Atomic Widgets (one per view)                         |
|--------------------|---------------------------------------------|-------------------------------------------------------|
| Visibility         | Only one view visible at a time             | All views visible simultaneously on different buttons |
| Layout flexibility | Fixed to one button slot                    | Each view can be placed anywhere on the grid          |
| Actions            | Click and Longpress shared across all views | Each view has its own Click and Longpress             |
| View switching     | Requires Longpress to cycle views           | No switching needed — each view is always visible     |
| Button cost        | Uses 1 button                               | Uses N buttons (one per view)                         |

### 5.3 Implementation Strategy

Atomic Widgets are implemented within the **same crate** as their parent Multi-View Widget. A single crate (e.g. `plugins/weather/`) provides both the
Multi-View Widget and all its Atomic Widget variants via the `widget_factory_plugin_graphic!` macro. This avoids crate explosion and dynamic library explosion —
one `.so` file per domain serves all widget variants.

Each Atomic Widget variant within the crate:

1. Shares the same `model/<domain>` dependency as its parent Multi-View Widget (shared message types, state topics, actions).
2. Subscribes to the same `state_topic` as the parent widget — it receives the full state update but only renders the portion relevant to its view.
3. Implements `GraphicRenderer` for headless rendering (MacroPad pixel buffer).
4. Implements `WidgetBuilder` for GTK rendering (so it can also be used in GTK sub-areas).
5. Is registered in the `widget_factory_plugin_graphic!` macro in `lib.rs` alongside the parent widget.

The host selects which variant to instantiate via the `widget` field in the TOML plugin entry:

```toml
{ id = "weather_today", path = "target/release/libsmearor_weather_widget.so", widget = "weather_today" }
```

This pattern applies to all Atomic Widget phases — each domain (Weather, Audio, MPRIS, SysInfo, Clock, etc.) ships as a single crate with all its widget
variants.

### 5.4 Shared Service Connection

All Atomic Widgets from the same parent (e.g. all Weather Atomic Widgets) share a single service connection. The service (e.g. `services/weather`) broadcasts
state updates on a topic (e.g. `service.weather.state`). Each Atomic Widget subscribes to this topic and extracts the relevant data for its view. No Atomic
Widget creates its own service connection.

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Service (e.g. weather)                         │
│                                                                      │
│  Broadcasts: service.weather.state → { today, tomorrow, forecast,   │
│              uv_index, sunrise, sunset, ... }                        │
└──────────────────────────┬───────────────────────────────────────────┘
                           │
              ┌────────────┼────────────┬────────────┬────────────┐
              │            │            │            │            │
              ▼            ▼            ▼            ▼            ▼
         Weather-     Weather-     Weather-     Weather-     Weather-
         Today        Tomorrow     Forecast     UV-Index     Sunrise
         (Atomic)     (Atomic)     (Atomic)     (Atomic)     (Atomic)
              │            │            │            │            │
              ▼            ▼            ▼            ▼            ▼
         Renders      Renders       Renders      Renders      Renders
         today data   tomorrow      forecast     uv_index     sunrise
```

### 5.5 Atomic Widget Catalogue

The following table lists all planned Atomic Widgets, their parent Multi-View Widget, display content, and default actions. The list is **not exhaustive** —
additional Atomic Widgets can be derived from any Multi-View Widget as needed.

#### Weather Atomic Widgets

| Atomic Widget       | Icon                         | Text                        | Click            | Longpress                    |
|---------------------|------------------------------|-----------------------------|------------------|------------------------------|
| Weather-Today       | Icon depends on weather code | Temperature in °C           | Open weather app | Open forecast detail area    |
| Weather-Tomorrow    | Icon depends on weather code | Temperature in °C           | Open weather app | Open forecast detail area    |
| Weather-Forecast    | Forecast icon                | "Forecast"                  | Open weather app | Open forecast detail area    |
| Weather-UV-Index    | UV-Index icon (colour-coded) | UV-Index value + risk level | Open weather app | Open UV detail area          |
| Weather-Sunrise     | Sunrise icon                 | Sunrise time                | Open weather app | Open sunrise detail area     |
| Weather-Sunset      | Sunset icon                  | Sunset time                 | Open weather app | Open sunset detail area      |
| Weather-Humidity    | Humidity icon                | Humidity percentage         | Open weather app | Open humidity detail area    |
| Weather-Wind        | Wind icon                    | Wind speed + direction      | Open weather app | Open wind detail area        |
| Weather-Air-Quality | AQI icon (colour-coded)      | AQI value + category        | Open weather app | Open air quality detail area |

#### Audio Atomic Widgets

| Atomic Widget          | Icon                                                           | Text                    | Click                       | Longpress                                      |
|------------------------|----------------------------------------------------------------|-------------------------|-----------------------------|------------------------------------------------|
| Audio-Volume-Up        | Volume up icon                                                 | —                       | Volume up (+5%)             | Volume up (+10%)                               |
| Audio-Volume-Down      | Volume down icon                                               | —                       | Volume down (-5%)           | Volume down (-10%)                             |
| Audio-Volume           | Icon changes based on volume level and mute state              | Volume percentage       | Toggle mute                 | Open audio settings area                       |
| Audio-Rotate-Device    | Icon based on device form-factor (speaker, headphone, headset) | Current device name     | Cycle to next output device | Open device selection area                     |
| Audio-Select-Device    | Icon based on device form-factor                               | Configured device name  | Select configured device    | Open device selection area                     |
| Audio-Mute             | Mute icon (changes based on mute state)                        | "Muted" or "Unmuted"    | Toggle mute                 | —                                              |
| Audio-Mic-Mute         | Microphone icon (changes based on mute state)                  | "Mic Muted" or "Mic On" | Toggle mic mute             | —                                              |
| Audio-Mic-Push-to-Talk | Microphone icon                                                | "PTT"                   | —                           | Hold: unmute mic while held; release: mute mic |

#### MPRIS Atomic Widgets

| Atomic Widget       | Icon                                     | Text                | Click                | Longpress                  |
|---------------------|------------------------------------------|---------------------|----------------------|----------------------------|
| MPRIS-Song          | Music note icon                          | Current song name   | Play / Pause         | Open player area           |
| MPRIS-Artist        | Artist icon                              | Current artist name | Play / Pause         | Open player area           |
| MPRIS-Album         | Album art thumbnail                      | Current album name  | Play / Pause         | Open player area           |
| MPRIS-Next          | Skip forward icon                        | —                   | Next track           | —                          |
| MPRIS-Previous      | Skip backward icon                       | —                   | Previous track       | —                          |
| MPRIS-Play-Pause    | Play/Pause icon (toggles based on state) | —                   | Play / Pause         | —                          |
| MPRIS-Stop          | Stop icon                                | —                   | Stop playback        | —                          |
| MPRIS-Switch-Player | Player switch icon                       | Current player name | Cycle to next player | Open player selection area |
| MPRIS-Seek-Forward  | Fast-forward icon                        | —                   | Seek forward 10s     | Seek forward 30s           |
| MPRIS-Seek-Backward | Rewind icon                              | —                   | Seek backward 10s    | Seek backward 30s          |
| MPRIS-Shuffle       | Shuffle icon (toggles based on state)    | —                   | Toggle shuffle       | —                          |
| MPRIS-Repeat        | Repeat icon (toggles based on state)     | —                   | Toggle repeat mode   | —                          |

#### SysInfo Atomic Widgets

| Atomic Widget            | Icon                                     | Text                    | Click                 | Longpress |
|--------------------------|------------------------------------------|-------------------------|-----------------------|-----------|
| SysInfo-CPU-Percent      | CPU icon                                 | CPU usage percentage    | Open system monitor   | —         |
| SysInfo-CPU-Temperature  | CPU icon (colour-coded by temperature)   | CPU temperature in °C   | Open system monitor   | —         |
| SysInfo-CPU-Frequency    | CPU icon                                 | CPU frequency in GHz    | Open system monitor   | —         |
| SysInfo-GPU-Percent      | GPU icon                                 | GPU usage percentage    | Open system monitor   | —         |
| SysInfo-GPU-Temperature  | GPU icon (colour-coded by temperature)   | GPU temperature in °C   | Open system monitor   | —         |
| SysInfo-Memory-Percent   | Memory icon                              | Memory usage percentage | Open system monitor   | —         |
| SysInfo-Memory-Used      | Memory icon                              | Used memory in GB       | Open system monitor   | —         |
| SysInfo-Swap-Percent     | Swap icon                                | Swap usage percentage   | Open system monitor   | —         |
| SysInfo-Battery-Percent  | Battery icon (changes based on level)    | Battery percentage      | Open power settings   | —         |
| SysInfo-Battery-State    | Battery icon (charging/discharging/full) | Battery state text      | Open power settings   | —         |
| SysInfo-Disk-Usage       | Disk icon                                | Disk usage percentage   | Open file manager     | —         |
| SysInfo-Network-Download | Download icon                            | Download speed          | Open network settings | —         |
| SysInfo-Network-Upload   | Upload icon                              | Upload speed            | Open network settings | —         |
| SysInfo-Uptime           | Clock icon                               | Uptime duration         | —                     | —         |
| SysInfo-Load-Average     | Activity icon                            | Load average (1 min)    | Open system monitor   | —         |

#### Clock Atomic Widgets

| Atomic Widget                            | Icon              | Text                                                | Click                  | Longpress                               |
|------------------------------------------|-------------------|-----------------------------------------------------|------------------------|-----------------------------------------|
| Clock-Time-Digital                       | —                 | Current time (HH:MM)                                | —                      | Open clock settings                     |
| Clock-Date-Digital                       | —                 | Current date (DD.MM)                                | —                      | Open calendar                           |
| Clock-Time-Analog                        | Analog clock face | —                                                   | —                      | Open clock settings                     |
| Clock-Big-Digital-Clock (Multi-Span 1×5) | —                 | Shows one digit of the time "11:11" per button      | —                      | —                                       |
| Clock-Big-Digital-Date (Multi-Span 1×5)  | —                 | Shows one digit of the date "24.12" per button      | —                      | —                                       |
| Clock-Countdown (Multi-Span 1×5)         | —                 | Shows one digit of the countdown "05:00" per button | Start / stop countdown | Increase digit (e.g. "04:00" → "05:00") |
| Clock-Timer (Multi-Span 1×5)             | —                 | Shows one digit of the timer "00:00" per button     | Start / stop timer     | Reset timer to "00:00"                  |

#### Wallpaper Atomic Widgets

| Atomic Widget      | Icon                        | Text                   | Click                       | Longpress                     |
|--------------------|-----------------------------|------------------------|-----------------------------|-------------------------------|
| Wallpaper-Selector | Preview image (thumbnail)   | Wallpaper name         | Select / apply wallpaper    | Open wallpaper selection area |
| Wallpaper-Next     | Next icon                   | —                      | Cycle to next wallpaper     | Open wallpaper selection area |
| Wallpaper-Previous | Previous icon               | —                      | Cycle to previous wallpaper | Open wallpaper selection area |
| Wallpaper-Random   | Shuffle icon                | —                      | Set random wallpaper        | Open wallpaper selection area |
| Wallpaper-Current  | Current wallpaper thumbnail | Current wallpaper name | Open wallpaper settings     | Open wallpaper selection area |

#### Hyprland Atomic Widgets

| Atomic Widget                     | Icon                    | Text                     | Click                          | Longpress                                |
|-----------------------------------|-------------------------|--------------------------|--------------------------------|------------------------------------------|
| Hyprland-Next-Window              | Window switch icon      | —                        | Cycle to next window           | —                                        |
| Hyprland-Previous-Window          | Window switch icon      | —                        | Cycle to previous window       | —                                        |
| Hyprland-Kill-Active              | Close / kill icon       | —                        | Kill active window             | Force kill active window                 |
| Hyprland-Toggle-Float             | Float icon              | —                        | Toggle floating mode           | —                                        |
| Hyprland-Fullscreen               | Fullscreen icon         | —                        | Toggle fullscreen              | Toggle fullscreen (maximised)            |
| Hyprland-Maximize                 | Maximize icon           | —                        | Toggle maximise                | —                                        |
| Hyprland-Next-Workspace           | Workspace next icon     | Current workspace number | Go to next workspace           | Move active window to next workspace     |
| Hyprland-Previous-Workspace       | Workspace previous icon | Current workspace number | Go to previous workspace       | Move active window to previous workspace |
| Hyprland-Move-Window-Right        | Arrow right icon        | —                        | Move active window right       | —                                        |
| Hyprland-Move-Window-Left         | Arrow left icon         | —                        | Move active window left        | —                                        |
| Hyprland-Move-Window-Down         | Arrow down icon         | —                        | Move active window down        | —                                        |
| Hyprland-Move-Window-Up           | Arrow up icon           | —                        | Move active window up          | —                                        |
| Hyprland-Cycle-Window             | Cycle icon              | —                        | Cycle to next window (alt+tab) | Reverse cycle                            |
| Hyprland-Toggle-Special-Workspace | Special workspace icon  | —                        | Toggle special workspace       | —                                        |
| Hyprland-Toggle-Group             | Group icon              | —                        | Toggle window group            | —                                        |
| Hyprland-Cycle-Group              | Group cycle icon        | —                        | Cycle within group             | —                                        |
| Hyprland-Reload                   | Reload icon             | —                        | Reload Hyprland config         | —                                        |
| Hyprland-Lock-Screen              | Lock icon               | —                        | Lock screen                    | —                                        |
| Hyprland-DPMS-Toggle              | Monitor icon            | —                        | Toggle display power           | —                                        |
| Hyprland-Focus-Monitor-Left       | Monitor left icon       | —                        | Focus monitor to the left      | —                                        |
| Hyprland-Focus-Monitor-Right      | Monitor right icon      | —                        | Focus monitor to the right     | —                                        |

#### Power Atomic Widgets

| Atomic Widget   | Icon                | Text        | Click              | Longpress      |
|-----------------|---------------------|-------------|--------------------|----------------|
| Power-Standby   | Standby / moon icon | "Standby"   | Enter standby      | —              |
| Power-Hibernate | Hibernate icon      | "Hibernate" | Hibernate          | —              |
| Power-Lock      | Lock icon           | "Lock"      | Lock screen        | —              |
| Power-Reboot    | Reboot icon         | "Reboot"    | Reboot system      | Force reboot   |
| Power-Shutdown  | Power off icon      | "Shutdown"  | Shutdown system    | Force shutdown |
| Power-Logout    | Logout icon         | "Logout"    | Log out of session | —              |
| Power-Suspend   | Suspend icon        | "Suspend"   | Suspend system     | —              |

#### Notifications Atomic Widgets

| Atomic Widget        | Icon                          | Text                                 | Click                   | Longpress                   |
|----------------------|-------------------------------|--------------------------------------|-------------------------|-----------------------------|
| Notifications-Count  | Bell icon                     | Notification count                   | Open notifications area | Clear all notifications     |
| Notifications-Latest | Bell icon                     | Latest notification text (truncated) | Open notifications area | Dismiss latest notification |
| Notifications-DND    | Do-not-disturb icon (toggles) | "DND" or "On"                        | Toggle do-not-disturb   | —                           |

#### Voice Assistant Atomic Widgets

| Atomic Widget                | Icon                                 | Text        | Click                        | Longpress                                      |
|------------------------------|--------------------------------------|-------------|------------------------------|------------------------------------------------|
| Voice-Assistant-Listen       | Microphone icon                      | "Listen"    | Start listening              | Open voice assistant overlay                   |
| Voice-Assistant-Push-to-Talk | Microphone icon                      | "PTT"       | —                            | Hold: start listening; release: stop listening |
| Voice-Assistant-Stop         | Stop icon                            | "Stop"      | Stop current response        | —                                              |
| Voice-Assistant-Status       | Status icon (changes based on state) | Status text | Open voice assistant overlay | —                                              |

#### Network Atomic Widgets

| Atomic Widget           | Icon                                | Text                          | Click                    | Longpress   |
|-------------------------|-------------------------------------|-------------------------------|--------------------------|-------------|
| Network-WiFi-Status     | WiFi icon (changes based on signal) | SSID or "Disconnected"        | Open network settings    | Toggle WiFi |
| Network-WiFi-Connect    | WiFi icon                           | "Connect"                     | Open WiFi selection area | —           |
| Network-Ethernet-Status | Ethernet icon                       | "Connected" or "Disconnected" | Open network settings    | —           |
| Network-VPN-Toggle      | VPN icon (toggles)                  | "VPN On" or "VPN Off"         | Toggle VPN               | —           |

#### App Launcher Atomic Widgets

| Atomic Widget    | Icon     | Text     | Click      | Longpress               |
|------------------|----------|----------|------------|-------------------------|
| App-Launcher-App | App icon | App name | Launch app | Kill app (send SIGTERM) |

The App Launcher is already atomic by nature — each button is a separate app. This is listed for completeness.

#### Workspace Switcher Atomic Widgets

| Atomic Widget      | Icon                                               | Text                                | Click                                       | Longpress                           |
|--------------------|----------------------------------------------------|-------------------------------------|---------------------------------------------|-------------------------------------|
| Workspace-Next     | Next icon (e.g. `nf-md-chevron_right`)             | —                                   | Switch to next workspace                    | Create new workspace after current  |
| Workspace-Previous | Previous icon (e.g. `nf-md-chevron_left`)          | —                                   | Switch to previous workspace                | Create new workspace before current |
| Workspace-Name     | Workspace icon (from `icon_map` or `default_icon`) | Current workspace name              | Configurable (e.g. open workspace overview) | Configurable                        |
| Workspace-Select   | Configurable icon                                  | Workspace name at `workspace_index` | Switch to workspace at `workspace_index`    | Configurable                        |

**Configuration**:

- **Workspace-Next** / **Workspace-Previous**: No additional config needed. Click cycles to next/previous workspace, longpress creates a new workspace at the
  end/beginning.
- **Workspace-Name**: Displays the current workspace name and icon. Supports `icon_map`, `default_icon`, and `show_label` config fields. Click/longpress actions
  are configurable via standard `click_topic`/`longpress_topic` fields.
- **Workspace-Select**: Switches to a specific workspace by index. Requires `workspace_index` (0-based) in the config. The icon and label are resolved from the
  workspace at that index. Supports `icon_map`, `default_icon`, and `show_label` config fields.

```toml
[ws_next]
click_topic = ""  # Built-in: switches to next workspace

[ws_select_2]
workspace_index = 2
icon = "nf-md-numeric-3"
default_icon = "nf-md-monitor"
icon_map = { "1" = "nf-md-numeric-1", "2" = "nf-md-numeric-2", "3" = "nf-md-numeric-3" }
```

---

## 6. Atomic Widgets in GTK Context

Atomic Widgets are not limited to MacroPad devices. In a GTK instance, they can be used to build **detail areas** that show multiple views of a Multi-View
Widget side by side.

### 6.1 Example: Weather Detail Area

A Multi-View Weather Widget in the main menu can use Longpress to open a sub-area that shows all Weather Atomic Widgets in a grid:

```
Main Menu (GTK)
┌──────────────────────────────────────────────┐
│  [Weather Multi-View Widget]                  │
│  Icon: ☀️  Text: 22°C                         │
│  Click: Open weather area                     │
│  Longpress: Open weather app                  │
│  Swipe-Up/Down: Switch view                   │
└──────────────────────────────────────────────┘
         │ Longpress / Click
         ▼
Weather Detail Area (GTK sub-area)
┌────────┬────────┬────────┬────────┬────────┐
│ Today  │Tomorr. │Forecast│ UV-Idx │Sunrise │
│  ☀️    │  ⛅    │  📊   │  ☀️    │  🌅   │
│ 22°C   │ 18°C   │ 3-day  │  6.2   │ 06:42  │
└────────┴────────┴────────┴────────┴────────┘
┌────────┐
│ Sunset │
│  🌇    │
│ 20:15  │
└────────┘
```

Each cell in the detail area is an Atomic Widget. Click on an Atomic Widget in the detail area can open a further sub-area or trigger the widget's Click action.

### 6.2 Configuration

```toml
[weather_detail]
area_type = "scroll"
plugins = [
    { id = "weather_today", path = "target/release/libsmearor_weather_widget.so", widget = "weather_today" },
    { id = "weather_tomorrow", path = "target/release/libsmearor_weather_widget.so", widget = "weather_tomorrow" },
    { id = "weather_forecast", path = "target/release/libsmearor_weather_widget.so", widget = "weather_forecast" },
    { id = "weather_uv_index", path = "target/release/libsmearor_weather_widget.so", widget = "weather_uv_index" },
    { id = "weather_sunrise", path = "target/release/libsmearor_weather_widget.so", widget = "weather_sunrise" },
    { id = "weather_sunset", path = "target/release/libsmearor_weather_widget.so", widget = "weather_sunset" },
]
```

---

## 7. Multi-Span Widgets

### 7.1 Concept

A **Multi-Span Widget** is a logical widget that occupies two or more physical buttons on the MacroPad grid. This is useful for:

- **Wider output**: A volume slider that spans two buttons horizontally, with the bar drawn across both buttons.
- **Compound actions**: Two buttons with independent Click actions and a shared Longpress action (e.g. Button 1 = volume down, Button 2 = volume up, Button 1+2
  Longpress = toggle mute).
- **Multi-digit displays**: A clock that spans 5 buttons, showing one digit per button (e.g. "11:11" across 5 buttons).

### 7.2 Span Configuration

A Multi-Span Widget is configured by placing multiple plugin entries that share the same `span_group` identifier. The host groups them together and renders them
as a single logical unit.

```toml
# Volume slider spanning 2 buttons horizontally (1×2)
[scroll_band]
plugins = [
    { id = "vol_left", path = "target/release/libsmearor_audio_volume_span_widget.so", span_group = "volume_slider", span_index = 0 },
    { id = "vol_right", path = "target/release/libsmearor_audio_volume_span_widget.so", span_group = "volume_slider", span_index = 1 },
]

[vol_left]
defaults = "menu_button"
# Button 1: Click = volume down, shared Longpress = toggle mute
click_topic = "tool.invoke"
click_payload = { tool = "audio", action = "volume_down" }
longpress_topic = "tool.invoke"
longpress_payload = { tool = "audio", action = "toggle_mute" }

[vol_right]
defaults = "menu_button"
# Button 2: Click = volume up, shared Longpress = toggle mute
click_topic = "tool.invoke"
click_payload = { tool = "audio", action = "volume_up" }
longpress_topic = "tool.invoke"
longpress_payload = { tool = "audio", action = "toggle_mute" }
```

### 7.3 Rendering

For Multi-Span Widgets, the host renders the logical widget once at the combined dimensions and then splits the pixel buffer into per-button segments:

```
1. Host identifies all plugins with the same span_group
2. Host sorts them by span_index
3. Host calculates combined dimensions:
   - Horizontal span (1×N): width = key_width * N, height = key_height
   - Vertical span (N×1): width = key_width, height = key_height * N
   - Grid span (M×N): width = key_width * M, height = key_height * N
4. Host calls render_graphic(combined_width, combined_height) on the first plugin in the group
5. Host splits the resulting pixel buffer into per-button segments
6. Host sends each segment as a SetButtonImage to the corresponding button index
```

Only the first plugin in the span group (lowest `span_index`) is responsible for rendering. All other plugins in the group delegate rendering to the first. This
avoids duplicate rendering and ensures visual consistency across the span.

### 7.4 Action Routing

Each button in a Multi-Span Widget has its own Click and Longpress actions (configured per plugin entry). Additionally, **compound actions** can be triggered by
pressing multiple buttons simultaneously:

| Action Type              | Detection                                                                            | Use Case                                              |
|--------------------------|--------------------------------------------------------------------------------------|-------------------------------------------------------|
| **Per-button Click**     | Single button pressed and released < 500 ms                                          | Independent action per button (e.g. volume up / down) |
| **Per-button Longpress** | Single button held >= 500 ms                                                         | Independent longpress per button                      |
| **Compound Longpress**   | Two or more buttons in the same span group pressed simultaneously and held >= 500 ms | Shared action across the span (e.g. toggle mute)      |

Compound action detection: the host tracks all currently pressed buttons. If two or more buttons from the same `span_group` are pressed within 100 ms of each
other and both are held for >= 500 ms, the host dispatches the compound action (configured via `compound_longpress_topic` / `compound_longpress_payload` on the
first plugin in the group).

### 7.5 Multi-Span Widget Examples

#### 7.5.1 Volume Slider (1×2 Horizontal)

```
┌────────┬────────┐
│        │        │
│  🔊   │  ████  │
│  60%   │  ░░░░  │
│        │        │
└────────┴────────┘
 Button 0  Button 1

Click (0): Volume down
Click (1): Volume up
Longpress (0 or 1): Toggle mute
Compound Longpress (0+1): Open audio settings area
```

The bar is drawn across both buttons — the fill level spans the full width of the combined buffer.

#### 7.5.2 Volume Slider (2×1 Vertical)

```
┌────────┐
│  🔊   │  Button 0
│  60%   │
├────────┤
│  ████  │  Button 1
│  ░░░░  │
└────────┘

Click (0): Volume up
Click (1): Volume down
Longpress (0 or 1): Toggle mute
```

#### 7.5.3 Big Digital Clock (1×5)

```
┌──┬──┬──┬──┬──┐
│ 1│ 1│ :│ 1│ 1│
└──┴──┴──┴──┴──┘
 0  1  2  3  4

Each button shows one character of "11:11".
The colon (button 2) blinks every second.
No individual click actions — display only.
```

#### 7.5.4 Countdown Timer (1×5)

```
┌──┬──┬──┬──┬──┐
│ 0│ 5│ :│ 0│ 0│
└──┴──┴──┴──┴──┘
 0  1  2  3  4

Click (any button): Start / stop countdown
Longpress (any button): Increase that digit
  - Longpress button 0: increase tens of minutes
  - Longpress button 1: increase minutes
  - Longpress button 3: increase tens of seconds
  - Longpress button 4: increase seconds
Compound Longpress (all): Reset to "00:00"
```

#### 7.5.5 Grid Span (2×2)

```
┌────────┬────────┐
│        │        │
│  🔊   │  📊   │   Button 0: Volume icon + percentage
│  60%   │  CPU   │   Button 1: CPU usage
│        │  45%   │
├────────┼────────┤
│        │        │
│  🌡️   │  💾   │   Button 2: CPU temperature
│  52°C  │  70%   │   Button 3: Memory usage
│        │        │
└────────┴────────┘
 0  1
 2  3

Each button has its own Click action.
Compound Longpress (all 4): Open system monitor area.
```

---

## 9. Architecture

### 9.1 Atomic Widget Plugin Structure

All Atomic Widget variants for a domain live in the same crate as their parent Multi-View Widget. Each variant has its own struct in a separate file (following
the one-struct-per-file rule), but they share a single `lib.rs` that registers all variants via the `widget_factory_plugin_graphic!` macro:

```
plugins/weather/
├── Cargo.toml
└── src/
    ├── lib.rs              # widget_factory_plugin_graphic! { "weather" => ..., "weather_today" => ..., ... }
    ├── widget.rs           # WeatherWidget struct + trait impls (Multi-View)
    ├── atomic.rs           # WeatherAtomicWidget struct + trait impls (Atomic)
    ├── atomic_graphic.rs   # GraphicRenderer impl for WeatherAtomicWidget
    ├── graphic.rs          # GraphicRenderer impl for WeatherWidget
    ├── html.rs             # HTML renderer for WeatherWidget
    └── config.rs           # WeatherWidgetConfig + WeatherAtomicConfig structs
```

```
plugins/audio/
├── Cargo.toml
└── src/
    ├── lib.rs              # widget_factory_plugin_graphic! { "audio" => ..., "audio_volume" => ..., ... }
    ├── widget.rs           # AudioWidget struct + trait impls (Multi-View)
    ├── atomic.rs           # AudioAtomicWidget struct + trait impls (Atomic)
    ├── atomic_graphic.rs   # GraphicRenderer impl for AudioAtomicWidget
    ├── graphic.rs          # GraphicRenderer impl for AudioWidget
    └── config.rs           # AudioWidgetConfig + AudioAtomicConfig structs
```

```
plugins/mpris/
├── Cargo.toml
└── src/
    ├── lib.rs              # widget_factory_plugin_graphic! { "mpris" => ..., "mpris_song" => ..., ... }
    ├── widget.rs           # MprisWidget struct + trait impls (Multi-View)
    ├── atomic.rs           # MprisAtomicWidget struct + trait impls (Atomic)
    ├── atomic_graphic.rs   # GraphicRenderer impl for MprisAtomicWidget
    ├── graphic.rs          # GraphicRenderer impl for MprisWidget
    └── config.rs           # MprisWidgetConfig + MprisAtomicConfig structs
```

The `widget_factory_plugin_graphic!` macro in `lib.rs` registers all variants:

```rust
widget_factory_plugin_graphic! {
    "weather" => weather_widget => WeatherWidget => html,
    "weather_today" => weather_today_widget => WeatherAtomicWidget,
    "weather_forecast" => weather_forecast_widget => WeatherAtomicWidget,
    "weather_tomorrow" => weather_tomorrow_widget => WeatherAtomicWidget,
    "weather_uv_index" => weather_uv_index_widget => WeatherAtomicWidget,
    "weather_sunrise" => weather_sunrise_widget => WeatherAtomicWidget,
    "weather_sunset" => weather_sunset_widget => WeatherAtomicWidget,
}
```

Each widget struct implements:

- `MessageHandler` — handles `InvokeToolMessage` for Click/Longpress actions.
- `MessageBroadcaster` — broadcasts `WidgetUpdateMessage` on state change.
- `PluginMetaGetter` — returns plugin metadata.
- `AsRef<Option<FfiCoreContext>>` — returns the FFI core context.
- `GraphicRenderer` — renders the view to a pixel buffer (headless).
- `WidgetBuilder` — renders the view as a GTK widget (for GTK sub-areas).

### 9.2 Model Reuse

Atomic Widgets reuse the model crate of their parent Multi-View Widget. Since all variants live in the same crate, they share the dependency directly:

```toml
# plugins/weather/Cargo.toml
[dependencies]
smearor-model-weather = { path = "../../model/weather" }
smearor-plugin-api = { path = "../../plugin-api" }
smearor-render-utils = { path = "../render-utils" }
```

No new model crates are needed for Atomic Widgets — they use the same state topics, message types, and actions as their parent.

### 9.3 Span Group Handling in the Host

The host (`host/mod.rs`) extends the area plugin loading to recognise `span_group` and `span_index` fields:

```rust
/// Plugin entry in area config, extended with span information.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AreaPluginEntry {
    /// Plugin ID.
    pub id: String,
    /// Path to the plugin .so file.
    pub path: String,
    /// Span group identifier (plugins with the same group form a Multi-Span Widget).
    #[serde(default)]
    pub span_group: Option<String>,
    /// Index within the span group (0-based).
    #[serde(default)]
    pub span_index: Option<u32>,
}
```

During rendering, the host:

1. Groups plugins by `span_group`.
2. For each group, sorts by `span_index`.
3. Calls `render_graphic()` on the first plugin with combined dimensions.
4. Splits the result and sends per-button `SetButtonImage` commands.
5. For non-grouped plugins, renders normally (current behaviour).

### 9.4 Compound Action Detection

The host tracks pressed buttons per span group:

```rust
/// Track currently pressed buttons for compound action detection.
struct PressedButtonTracker {
    /// Map: span_group → Vec<(button_index, press_time)>
    pressed: HashMap<String, Vec<(u8, Instant)>>,
}
```

When two or more buttons from the same span group are pressed within 100 ms and held for >= 500 ms, the host dispatches the compound longpress action from the
first plugin's config (`compound_longpress_topic` / `compound_longpress_payload`).

---

## 10. Configuration

### 10.1 Atomic Widget Config (MacroPad)

```toml
areas = ["scroll_band"]

[scroll_band]
area_type = "scroll"
plugins = [
    { id = "weather_today", path = "target/release/libsmearor_weather_widget.so", widget = "weather_today" },
    { id = "weather_tomorrow", path = "target/release/libsmearor_weather_widget.so", widget = "weather_tomorrow" },
    { id = "audio_volume", path = "target/release/libsmearor_audio_widget.so", widget = "audio_volume" },
    { id = "mpris_play_pause", path = "target/release/libsmearor_mpris_widget.so", widget = "mpris_play_pause" },
    { id = "clock_time", path = "target/release/libsmearor_clock_widget.so", widget = "clock_time_digital" },
]

[weather_today]
defaults = "menu_button"
click_topic = "tool.invoke"
click_payload = { tool = "weather_today", action = "open_weather_app" }
longpress_topic = "area.open"
longpress_payload = { area = "weather_detail" }

[audio_volume]
defaults = "menu_button"
click_topic = "tool.invoke"
click_payload = { tool = "audio", action = "toggle_mute" }
longpress_topic = "area.open"
longpress_payload = { area = "audio_detail" }

[mpris_play_pause]
defaults = "menu_button"
click_topic = "tool.invoke"
click_payload = { tool = "mpris", action = "play_pause" }

[clock_time]
defaults = "menu_button"
# No click action — display only
```

### 10.2 Multi-Span Widget Config (MacroPad)

```toml
[scroll_band]
area_type = "scroll"
plugins = [
    { id = "vol_left", path = "target/release/libsmearor_audio_widget.so", widget = "audio_volume_span", span_group = "volume_slider", span_index = 0 },
    { id = "vol_right", path = "target/release/libsmearor_audio_widget.so", widget = "audio_volume_span", span_group = "volume_slider", span_index = 1 },
    { id = "clock_digit_0", path = "target/release/libsmearor_clock_widget.so", widget = "clock_big_digital", span_group = "big_clock", span_index = 0 },
    { id = "clock_digit_1", path = "target/release/libsmearor_clock_widget.so", widget = "clock_big_digital", span_group = "big_clock", span_index = 1 },
    { id = "clock_digit_2", path = "target/release/libsmearor_clock_widget.so", widget = "clock_big_digital", span_group = "big_clock", span_index = 2 },
    { id = "clock_digit_3", path = "target/release/libsmearor_clock_widget.so", widget = "clock_big_digital", span_group = "big_clock", span_index = 3 },
    { id = "clock_digit_4", path = "target/release/libsmearor_clock_widget.so", widget = "clock_big_digital", span_group = "big_clock", span_index = 4 },
]

[vol_left]
defaults = "menu_button"
click_topic = "tool.invoke"
click_payload = { tool = "audio", action = "volume_down" }
longpress_topic = "tool.invoke"
longpress_payload = { tool = "audio", action = "toggle_mute" }
compound_longpress_topic = "area.open"
compound_longpress_payload = { area = "audio_detail" }

[vol_right]
defaults = "menu_button"
click_topic = "tool.invoke"
click_payload = { tool = "audio", action = "volume_up" }
longpress_topic = "tool.invoke"
longpress_payload = { tool = "audio", action = "toggle_mute" }

[clock_digit_0]
defaults = "menu_button"
# Display only — no click action
```

---

## 11. Implementation Phases

### Phase 1: Input Triggers

**Status**: ✅ Implemented.

**Order**: First. All widget phases depend on the new trigger types.

**Changes**:

- Extend `MacroPadInputMessage` handling in `host/mod.rs` to detect Click, Longpress, Hold, and Double Press.
- Add `hold_topic`, `hold_payload`, `double_press_topic`, `double_press_payload` to button/widget config.
- Add `hold_start` and `hold_stop` action dispatching.
- Add double-press detection with 300 ms window.

**Exit Criteria**: All four trigger types are detected and dispatched correctly on a MacroPad device.

### Phase 2: Span Group Support

**Status**: ✅ Implemented.

**Order**: After Phase 1. Multi-Span Widgets depend on this.

**Changes**:

- Add `span_group` and `span_index` fields to `AreaPluginEntry` in `model/area`.
- Extend `render_buttons_to_device()` in `host/mod.rs` to group plugins by `span_group`, render at combined dimensions, and split the result.
- Add compound longpress detection (two+ buttons in same group pressed within 100 ms, held >= 500 ms).
- Add `compound_longpress_topic` / `compound_longpress_payload` to config.

**Exit Criteria**: A 1×2 volume slider renders correctly across two buttons with per-button and compound actions.

### Phase 3: Weather Atomic Widgets

**Status**: ✅ Implemented.

**Order**: After Phase 2. Can be done in parallel with Phase 4–8.

**Changes**:

- Extend `plugins/weather/` crate with `WeatherAtomicWidget` struct in `atomic.rs`.
- Add `atomic_graphic.rs` for `GraphicRenderer` impl.
- Add `WeatherAtomicConfig` to `config.rs`.
- Register all atomic variants in `widget_factory_plugin_graphic!` macro in `lib.rs`.
- Each variant subscribes to `service.weather.state` and renders its specific view.
- Implement `GraphicRenderer` and `WidgetBuilder` for `WeatherAtomicWidget`.

**Exit Criteria**: All Weather Atomic Widget variants render correctly on MacroPad and in GTK sub-areas using a single `.so` file.

### Phase 4: Audio Atomic Widgets

**Status**: ✅ Implemented.

**Order**: After Phase 2. Can be done in parallel with Phase 3, 5–8.

**Changes**:

- Added `Refresh` variant to `AudioCommandAction` in `model/audio` + `refresh()` helper.
- Re-exported `TOPIC_STATUS` and `TOPIC_COMMAND` from `model/audio/src/lib.rs`.
- Handle `Refresh` action in `services/audio/src/service/loaded_service.rs`.
- Extend `plugins/audio/` crate with atomic widget variants: `audio_volume`, `audio_volume_up`, `audio_volume_down`, `audio_mute`, `audio_rotate_device`.
- Add `AudioAtomicWidget` struct in `atomic.rs`, `GraphicRenderer` impl in `atomic_graphic.rs`.
- Add `GraphicRenderer` impl for main `AudioWidget` in `graphic.rs`.
- Register all variants in `widget_factory_plugin_graphic!` macro in `lib.rs`.
- Add `smearor-model-mcp`, `smearor-model-widget`, `smearor-render-utils` dependencies to `Cargo.toml`.
- Each variant subscribes to `service.audio.status` and renders its specific view.
- Atomic widgets request initial status refresh on construction via `AudioCommandMessage::refresh()`.

**Exit Criteria**: All Audio Atomic Widget variants render and respond to Click/Longpress triggers using a single `.so` file.

### Phase 5: MPRIS Atomic Widgets

**Status**: ✅ Implemented.

**Order**: After Phase 2. Can be done in parallel with Phase 3–4, 6–8.

**Changes**:

- Added `Refresh` variant to `MprisCommandAction` in `model/mpris` + `refresh()` helper.
- Re-exported `TOPIC_STATUS` and `TOPIC_COMMAND` from `model/mpris/src/lib.rs`.
- Handle `Refresh` action in `services/mpris/src/service/loaded_service.rs` (maps to internal `MprisCommand::RefreshStatus`).
- Extend `plugins/mpris/` crate with atomic widget variants: `mpris_song`, `mpris_artist`, `mpris_album`, `mpris_next`, `mpris_previous`, `mpris_play_pause`,
  `mpris_stop`, `mpris_switch_player`, `mpris_seek_forward`, `mpris_seek_backward`, `mpris_shuffle`, `mpris_repeat`.
- Add `MprisAtomicWidget` struct in `atomic.rs`, `GraphicRenderer` impl in `atomic_graphic.rs`.
- Add `GraphicRenderer` impl for main `MprisWidget` in `graphic.rs` (added `last_status` field to `MprisWidget`).
- Register all variants in `widget_factory_plugin_graphic!` macro in `lib.rs`.
- Add `smearor-model-mcp`, `smearor-model-widget`, `smearor-render-utils` dependencies to `Cargo.toml`.
- Each variant subscribes to `service.mpris.status` and renders its specific view.
- Atomic widgets request initial status refresh on construction via `MprisCommandMessage::refresh()`.

**Exit Criteria**: All MPRIS Atomic Widget variants render and respond to Click/Longpress triggers using a single `.so` file.

### Phase 6: SysInfo, Clock, Power, and Other Atomic Widgets

**Status**: ✅ Implemented (SysInfo, Clock, Power, Wallpaper, Notifications, Network, Voice Assistant).

**Order**: After Phase 2. Can be done in parallel with Phase 3–5, 7–8.

**Note**: Hyprland Atomic Widgets are **not yet implemented** — they require a `plugins/hyprland/` crate which does not exist yet. The Hyprland Service
(`services/hyprland/`) is fully implemented and operational, providing the necessary `service.hyprland.dispatch` topic. The Hyprland Atomic Widgets can be built
on top of the existing service.

**Changes**:

- Extend existing domain crates with atomic widget variants: SysInfo (CPU, GPU, Memory, Battery, Disk, Network, Uptime, Load), Clock (Time-Digital,
  Date-Digital, Time-Analog, Big-Digital-Clock, Big-Digital-Date, Countdown, Timer), Power (Standby, Hibernate, Lock, Reboot, Shutdown, Logout, Suspend),
  Wallpaper, Hyprland, Notifications, Voice Assistant, Network.
- Add atomic structs and `GraphicRenderer` impls within each existing crate.
- Register all variants in each crate's `widget_factory_plugin_graphic!` macro.
- Clock Multi-Span Widgets (Big-Digital-Clock, Countdown, Timer) use span group rendering.

**Exit Criteria**: All Atomic Widget variants render correctly on MacroPad and in GTK sub-areas, each domain using a single `.so` file.

### Phase 8: Multi-Span Widget Variants

**Status**: ✅ Implemented.

**Order**: After Phase 2 and Phase 4 (Audio Atomic Widgets). Can be done in parallel with Phase 3, 5–7.

**Changes**:

- Add `audio_volume_span` variant to `plugins/audio/` crate for the 1×2 / 2×1 volume slider.
- Add `clock_big_digital`, `clock_big_date`, `clock_countdown`, `clock_timer` variants to `plugins/clock/` crate.
- Each variant renders at combined dimensions and supports per-button and compound actions.
- Register all variants in the respective crate's `widget_factory_plugin_graphic!` macro.

**Exit Criteria**: All Multi-Span Widget variants render correctly across multiple buttons with working per-button and compound actions, each using their
domain's single `.so` file.

### Phase 8b: Workspace Switcher Atomic Widgets

**Status**: ✅ Implemented.

**Order**: After Phase 2. Can be done in parallel with Phase 3–8.

**Changes**:

- Add `WorkspaceAtomicWidget` struct in `plugins/workspace-switcher/src/atomic.rs`.
- Add `WorkspaceAtomicView` enum with variants: `Next`, `Previous`, `Name`, `Select`.
- Add `WorkspaceAtomicConfig` in `config.rs` with `workspace_index`, `icon`, `icon_map`, `default_icon`, `show_label` fields.
- Add `GraphicRenderer` impl in `atomic_graphic.rs`.
- Register all variants in `widget_factory_plugin_graphic!` macro in `lib.rs`: `workspace_next`, `workspace_previous`, `workspace_name`, `workspace_select`.
- Each variant subscribes to `compositor::workspace_snapshot`, `compositor::workspace_changed`, and `compositor::workspace_lifecycle` topics.
- `WorkspaceNext` / `WorkspacePrevious`: Click switches to next/previous workspace, longpress creates new workspace.
- `WorkspaceName`: Displays current workspace name and icon, configurable click/longpress actions.
- `WorkspaceSelect`: Click switches to workspace at configured `workspace_index`.

**Exit Criteria**: All Workspace Switcher Atomic Widget variants render and respond to Click/Longpress triggers using a single `.so` file.

### Phase 9: Integration and Testing

**Status**: ❌ Not implemented.

**Order**: After all previous phases.

**Changes**:

- Integration tests: load headless instance with Atomic Widgets, verify `render_graphic()` output.
- Integration tests: verify Click, Longpress, Hold, and Double Press triggers.
- Integration tests: verify Multi-Span Widget rendering and compound actions.
- Config examples: `config-macropad-atomic-widgets.toml`, `config-macropad-multi-span.toml`.
- GTK integration: verify Atomic Widgets render correctly in GTK sub-areas.

**Exit Criteria**: All Atomic Widgets and Multi-Span Widgets work correctly on MacroPad and in GTK instances. All trigger types are detected and dispatched
correctly.

---

## 12. File Changes Summary

| File                                           | Change                                                                                                   |
|------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| `model/area/src/lib.rs`                        | Add `span_group`, `span_index` to `AreaPluginEntry`                                                      |
| `smearor-swipe-launcher/src/host/mod.rs`       | Extend input handling for Hold, Double Press; add span group rendering; add compound longpress detection |
| `model/audio/src/messages/command.rs`          | Add `Refresh` variant to `AudioCommandAction` + `refresh()` helper                                       |
| `model/audio/src/lib.rs`                       | Re-export `TOPIC_STATUS` and `TOPIC_COMMAND`                                                             |
| `services/audio/src/service/loaded_service.rs` | Handle `Refresh` action                                                                                  |
| `plugins/audio/src/lib.rs`                     | Register Audio Atomic Widget variants in `widget_factory_plugin_graphic!` macro                          |
| `plugins/audio/src/atomic.rs`                  | **New** — `AudioAtomicWidget` struct + trait impls                                                       |
| `plugins/audio/src/atomic_graphic.rs`          | **New** — `GraphicRenderer` impl for `AudioAtomicWidget`                                                 |
| `plugins/audio/src/graphic.rs`                 | **New** — `GraphicRenderer` impl for `AudioWidget`                                                       |
| `plugins/audio/Cargo.toml`                     | Add `smearor-model-mcp`, `smearor-model-widget`, `smearor-render-utils` dependencies                     |
| `model/mpris/src/messages/command.rs`          | Add `Refresh` variant to `MprisCommandAction` + `refresh()` helper                                       |
| `model/mpris/src/lib.rs`                       | Re-export `TOPIC_STATUS` and `TOPIC_COMMAND`                                                             |
| `services/mpris/src/service/loaded_service.rs` | Handle `Refresh` action (maps to `MprisCommand::RefreshStatus`)                                          |
| `plugins/mpris/src/lib.rs`                     | Register MPRIS Atomic Widget variants in `widget_factory_plugin_graphic!` macro                          |
| `plugins/mpris/src/atomic.rs`                  | **New** — `MprisAtomicWidget` struct + trait impls                                                       |
| `plugins/mpris/src/atomic_graphic.rs`          | **New** — `GraphicRenderer` impl for `MprisAtomicWidget`                                                 |
| `plugins/mpris/src/graphic.rs`                 | **New** — `GraphicRenderer` impl for `MprisWidget`                                                       |
| `plugins/mpris/src/widget.rs`                  | Add `last_status` field for `GraphicRenderer` support                                                    |
| `plugins/mpris/Cargo.toml`                     | Add `smearor-model-mcp`, `smearor-model-widget`, `smearor-render-utils` dependencies                     |
| `plugins/weather/src/lib.rs`                   | Register Weather Atomic Widget variants in `widget_factory_plugin_graphic!` macro                        |
| `plugins/weather/src/atomic.rs`                | **New** — `WeatherAtomicWidget` struct + trait impls                                                     |
| `plugins/weather/src/atomic_graphic.rs`        | **New** — `GraphicRenderer` impl for `WeatherAtomicWidget`                                               |
| `plugins/weather/src/config.rs`                | Add `WeatherAtomicConfig` struct                                                                         |
| `plugins/clock/src/lib.rs`                     | Register Clock Atomic Widget variants in `widget_factory_plugin_graphic!` macro                          |
| `plugins/clock/src/atomic.rs`                  | **New** — Clock atomic widget structs + trait impls                                                      |
| `plugins/clock/src/atomic_graphic.rs`          | **New** — `GraphicRenderer` impls for clock atomic widgets                                               |
| `plugins/sysinfo/src/lib.rs`                   | Register SysInfo Atomic Widget variants in `widget_factory_plugin_graphic!` macro                        |
| `plugins/sysinfo/src/atomic.rs`                | **New** — SysInfo atomic widget structs + trait impls                                                    |
| `plugins/sysinfo/src/atomic_graphic.rs`        | **New** — `GraphicRenderer` impls for SysInfo atomic widgets                                             |
| `plugins/power/src/lib.rs`                     | Register Power Atomic Widget variants in `widget_factory_plugin_graphic!` macro                          |
| `plugins/power/src/atomic.rs`                  | **New** — Power atomic widget structs + trait impls                                                      |
| `plugins/power/src/atomic_graphic.rs`          | **New** — `GraphicRenderer` impls for power atomic widgets                                               |
| `plugins/wallpaper/src/lib.rs`                 | Register Wallpaper Atomic Widget variants in `widget_factory_plugin_graphic!` macro                      |
| `plugins/wallpaper/src/atomic.rs`              | **New** — Wallpaper atomic widget structs + trait impls                                                  |
| `plugins/wallpaper/src/atomic_graphic.rs`      | **New** — `GraphicRenderer` impls for wallpaper atomic widgets                                           |
| `plugins/workspace-switcher/src/lib.rs`        | Register Workspace Atomic Widget variants in `widget_factory_plugin_graphic!` macro                      |
| `plugins/workspace-switcher/src/atomic.rs`     | **New** — `WorkspaceAtomicWidget` struct + trait impls                                                   |
| `plugins/workspace-switcher/src/config.rs`     | Add `WorkspaceAtomicConfig` struct                                                                       |
| `plugins/workspace-switcher/Cargo.toml`        | Add `smearor-model-widget` dependency                                                                    |
| `plugin-api/src/graphic/factory.rs`            | Add `@first_name` helper: fallback to first registered widget when `widget` field is empty or missing    |

---

## 13. Risks and Considerations

1. **Crate and library count**: Atomic Widgets are implemented within their parent domain's crate, so no new crates or `.so` files are created for atomic
   variants. Each domain crate grows in source size but produces a single shared library.

2. **Build time**: Since no new crates are added for atomic variants, build time impact is minimal. The existing domain crates are rebuilt with additional
   source files. Parallel compilation is unaffected.

3. **Service connection sharing**: All Atomic Widgets from the same parent subscribe to the same state topic. If 9 Weather Atomic Widgets are loaded, the
   message broker delivers 9 copies of each state update. Mitigation: The message broker could support topic filtering or the widgets could share a common state
   cache via a shared model crate.

4. **Span group complexity**: Multi-Span Widgets add complexity to the host's rendering and input handling. The host must correctly group, sort, render at
   combined dimensions, split, and detect compound actions. Edge cases: what if a span group has gaps (non-contiguous buttons)? What if the grid wraps?
   Mitigation: Span groups must be contiguous in the plugin list. The host validates this on area load.

5. **Double Press vs. Click timing**: The 300 ms double-press window introduces a 300 ms delay before a single Click is dispatched (the host must wait to see if
   a second press arrives). This may feel sluggish. Mitigation: The delay is only applied to buttons that have `double_press_topic` configured. Buttons without
   double-press support dispatch Click immediately on release.

6. **Hold trigger and Click coexistence**: If a button has both Hold and Click configured, the host dispatches `hold_start` on press. On release, if the
   duration was < 500 ms, it also dispatches `click`. This means a short tap on a Hold-configured button fires both `hold_start` and `hold_stop` (quickly) plus
   `click`. Mitigation: If `hold_topic` is configured, the host suppresses `click` for presses < 500 ms and only dispatches `hold_start` + `hold_stop`. Click is
   only dispatched if `hold_topic` is not configured.

7. **Atomic Widget naming**: Atomic Widget variants are identified by the `widget` field in the TOML plugin entry (e.g. `widget = "weather_today"`). The `.so`
   file follows the parent domain's naming pattern (e.g. `libsmearor_weather_widget.so`). Multiple variants share the same `.so` file, differentiated only by
   the `widget` field at load time.

8. **Hyprland Atomic Widgets**: The Hyprland Service is fully implemented and operational, but no `plugins/hyprland/` widget crate exists yet. The Hyprland
   Atomic Widgets listed in Section 5.5 require a new plugin crate that subscribes to `service.hyprland.dispatch` and sends dispatch messages on
   click/longpress. This is a future implementation task.

9. **Default Widget Fallback**: The `widget_factory_plugin_graphic!` macro falls back to the first registered widget variant when the `widget` field is empty or
   missing from the TOML plugin entry. This ensures backward compatibility with existing configurations that predate the factory macro, where a single widget
   type was the only option. Plugin authors should register the "primary" widget (e.g. the Multi-View Widget) as the first entry in the macro invocation.
