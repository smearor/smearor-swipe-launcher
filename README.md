# Smearor Swipe Launcher

A swipe-driven, touch-optimized application launcher for Wayland desktops. Built in Rust with native GTK 4 widgets, multi-instance support, rotation, and a
plugin-based architecture.

Originally designed for the Smearor touch table, but works on any Wayland desktop — tablets, convertibles, or traditional monitors.

---

## Quickstart

### Prerequisites

- Rust (Edition 2024, `rustc >= 1.95`)
- GTK 4 development headers
- `gtk4-layer-shell`
- Wayland compositor (Hyprland, GNOME on Wayland, etc.)
- `libpulse` (for audio plugin)
- `zbus` (for MPRIS, notifications, GNOME integration)

### From Debian packages

```bash
sudo apt install smearor-swipe-launcher-full
systemctl --user enable --now smearor-swipe-launcher-hyprland
```

On first launch, default configs are copied from `/usr/share/smearor/` to `~/.config/smearor/`.

### From source

```bash
cargo build --release
cargo run --release
```

Plugin `.so` files are placed in `target/release/`. Configuration lives in `configs/launcher/config.toml` (areas and plugins) and
`configs/services/services.toml` (services).

---

## Motivation

Touch-optimized launchers are rare in the Wayland ecosystem. Most existing launchers are keyboard-driven or web-based. The Smearor Swipe Launcher fills this gap
with:

- **Touch-first interaction** — swipe gestures, long-press, double-press as native patterns
- **Multi-instance** — each side of a table or monitor runs its own instance, sharing a single host process
- **Rotation** — 0°, 90°, 180°, 270° with automatic layer-shell positioning
- **Native GTK 4** — full control over rendering and gesture handling, no web view
- **MacroPad support** — Elgato Stream Deck and Loupedeck integration with headless instances rendering widgets as pixel buffers
- **Deep system integration** — Hyprland, GNOME, MPRIS, NetworkManager, PulseAudio, XDG Desktop Portal

---

## Features

- **Swipe and touch navigation** — scroll left/right, swipe up/down for sub-menus
- **Multi-instance** — multiple launcher windows from a single host process
- **Rotation** — visual rotation with automatic layer-shell position adjustment
- **Dynamic area management** — fixed, scroll, and transient areas with transition animations
- **Layout profiles** — per-workspace layout configurations
- **MacroPad integration** — Stream Deck and Loupedeck with LCD key rendering
- **Action bindings** — configurable input-to-message mappings
- **Icon rendering** — freedesktop icon themes with fallback support
- **MCP server** — Model Context Protocol server for AI integration
- **Web interface** — browser-based control
- **Inter-instance events** — message broker routes events between instances

---

## Architecture

The launcher is a single-process **LauncherHost** that manages multiple **LauncherInstance** children. A central **message broker** (tokio unbounded channel)
routes `FfiEnvelope` messages between instances and services.

```
LauncherHost
├── Message Broker
├── Service Manager       (services loaded once, shared across instances)
├── MCP Registry
├── Web Server
└── Launcher Instances
    ├── Instance A (GTK)  — PluginManager + AreaManager + GTK Window
    ├── Instance B (Headless) — PluginManager + AreaManager (MacroPad)
    └── Instance C (Web)  — PluginManager + AreaManager
```

### Crate types

- **Widget plugins** (`plugins/`) — GTK widgets, loaded per-instance as `.so` dynamic libraries
- **Service plugins** (`services/`) — business logic, loaded once and shared
- **Model crates** (`model/`) — shared structs, enums, and message types with `#[stabby::stabby]` FFI support

Plugins communicate via the message broker using typed messages defined in model crates. ABI stability is provided by `stabby`.

> For full architecture details, see the [book](book/src/SUMMARY.md).

---

## Services

| Service              | Description                                                                      |
|----------------------|----------------------------------------------------------------------------------|
| **app-launcher**     | Scans `.desktop` files, provides application search and launch                   |
| **audio**            | PulseAudio volume control, mute toggling, sink management                        |
| **gnome**            | GNOME Shell integration via D-Bus (settings, extensions)                         |
| **http**             | Generic HTTP client for outbound requests from plugins                           |
| **hyprland**         | Hyprland IPC: workspace tracking, window management, dispatch                    |
| **loupedeck**        | Loupedeck MacroPad USB HID driver (CT, Live, Live S, Razer Stream Controller)    |
| **mpris**            | Media player control via D-Bus MPRIS interface                                   |
| **network**          | NetworkManager integration: WiFi, Ethernet, VPN, airplane mode                   |
| **notifications**    | D-Bus notification daemon listener                                               |
| **personalization**  | Reads desktop settings for adaptive theming (accent color, font)                 |
| **power**            | Power management via systemd-logind (shutdown, reboot, suspend, etc.)            |
| **streamdeck**       | Elgato Stream Deck USB HID driver (all models)                                   |
| **sysinfo**          | System metrics: CPU, memory, disk, network, temperature, uptime, load            |
| **terminal_command** | Launches and manages terminal commands from widgets                              |
| **voice_assistant**  | Local LLM voice assistant with ReAct tool selection, STT (whisper-rs), TTS       |
| **wallpaper**        | Wallpaper theme scanning and application                                         |
| **wayland**          | Wayland compositor integration: layer-shell, monitor events, workspace lifecycle |
| **weather**          | Weather data from Open-Meteo API with geocoding                                  |

---

## Plugins

| Plugin                 | Description                                                                             |
|------------------------|-----------------------------------------------------------------------------------------|
| **app-launcher**       | Displays and launches applications from `.desktop` files                                |
| **audio**              | Volume control widget with dynamic icon, click-to-mute, scroll-to-adjust                |
| **button**             | Generic configurable button — icon, text, colors, and action bindings from config       |
| **clock**              | Clock widget with configurable formats and timezones                                    |
| **mpris**              | Media player control: album art, track info, play/pause/next/previous                   |
| **network**            | Network status with 7 views: WiFi, Ethernet, throughput, scan, VPN, airplane mode, QR   |
| **notifications**      | Notification badge counter and slide-in banners                                         |
| **power**              | Power actions: shutdown, reboot, suspend, hibernate, lock, logout, firmware reboot      |
| **sysinfo**            | Real-time system monitoring sub-widgets (CPU, memory, disk, network, temperature, etc.) |
| **voice_assistant**    | Voice assistant UI with microphone icon and state feedback                              |
| **wallpaper**          | Wallpaper theme browser with preview images                                             |
| **weather**            | Weather forecast with 15 views (current, forecast, wind, UV, sunrise/sunset, etc.)      |
| **workspace-switcher** | Visual workspace switching for Hyprland                                                 |

---

## License

MIT License — see [LICENSE.md](LICENSE.md).

Copyright (c) 2026 Andreas Schaeffer, the Reactive Graph Contributors and the Smearor Contributors.
