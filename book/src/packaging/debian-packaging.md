# Debian Packaging

The Smearor Swipe Launcher can be packaged as Debian (`.deb`) packages using [`cargo-deb`](https://github.com/kornelski/cargo-deb). The packaging strategy uses
**one package per plugin/service** for fine-grained installation control, plus a metapackage for convenient full installation.

## Package Architecture

### Main Package: `smearor-swipe-launcher`

Contains the main binary, systemd user service files, default configs, example configs, and udev rules.

**Contents:**

- Binary: `/usr/bin/smearor-swipe-launcher`
- Systemd user services: `/usr/lib/systemd/user/smearor-swipe-launcher*.service`
- Default configs: `/usr/share/smearor/launcher/config.toml`, `/usr/share/smearor/services/services.toml`, `/usr/share/smearor/services/wallpaper.toml`
- Example configs: `/usr/share/smearor/examples/launcher/*.toml`, `/usr/share/smearor/examples/areas/*.toml`
- Udev rules: `/usr/lib/udev/rules.d/52-loupedeck.rules`, `/usr/lib/udev/rules.d/52-streamdeck.rules`
- Postinst script: creates `/usr/lib/smearor/` plugin directory

### Plugin Packages (Widgets)

Each widget plugin is a separate Debian package containing a single `.so` file installed to `/usr/lib/smearor/`. All widget packages depend on
`smearor-swipe-launcher` and their corresponding service packages.

| Package Name                        | .so File                               |
|-------------------------------------|----------------------------------------|
| `smearor-plugin-app-launcher`       | `libsmearor_app_launcher_widget.so`    |
| `smearor-plugin-audio`              | `libsmearor_audio_widget.so`           |
| `smearor-plugin-button`             | `libsmearor_button_widget.so`          |
| `smearor-plugin-clock`              | `libsmearor_clock_widget.so`           |
| `smearor-plugin-mpris`              | `libsmearor_mpris_widget.so`           |
| `smearor-plugin-network`            | `libsmearor_network_widget.so`         |
| `smearor-plugin-notifications`      | `libsmearor_notifications_widget.so`   |
| `smearor-plugin-power`              | `libsmearor_power_widget.so`           |
| `smearor-plugin-sysinfo`            | `libsmearor_sysinfo_widget.so`         |
| `smearor-plugin-voice-assistant`    | `libsmearor_voice_assistant_widget.so` |
| `smearor-plugin-wallpaper`          | `libsmearor_wallpaper_widget.so`       |
| `smearor-plugin-weather`            | `libsmearor_weather_widget.so`         |
| `smearor-plugin-workspace-switcher` | `libsmearor_workspace_switcher.so`     |

### Service Packages

Each service plugin is a separate Debian package containing a single `.so` file installed to `/usr/lib/smearor/`. All service packages depend on
`smearor-swipe-launcher`.

| Package Name                       | .so File                                 |
|------------------------------------|------------------------------------------|
| `smearor-service-app-launcher`     | `libsmearor_app_launcher_service.so`     |
| `smearor-service-audio`            | `libsmearor_audio_service.so`            |
| `smearor-service-gnome`            | `libsmearor_gnome_service.so`            |
| `smearor-service-http`             | `libsmearor_http_service.so`             |
| `smearor-service-hyprland`         | `libsmearor_hyprland_service.so`         |
| `smearor-service-loupedeck`        | `libsmearor_loupedeck_service.so`        |
| `smearor-service-mpris`            | `libsmearor_mpris_service.so`            |
| `smearor-service-network`          | `libsmearor_network_service.so`          |
| `smearor-service-notifications`    | `libsmearor_notifications_service.so`    |
| `smearor-service-personalization`  | `libsmearor_personalization_service.so`  |
| `smearor-service-power`            | `libsmearor_power_service.so`            |
| `smearor-service-streamdeck`       | `libsmearor_streamdeck_service.so`       |
| `smearor-service-sysinfo`          | `libsmearor_sysinfo_service.so`          |
| `smearor-service-terminal-command` | `libsmearor_terminal_command_service.so` |
| `smearor-service-voice-assistant`  | `libsmearor_voice_assistant_service.so`  |
| `smearor-service-wallpaper`        | `libsmearor_wallpaper_service.so`        |
| `smearor-service-wayland`          | `libsmearor_wayland_service.so`          |
| `smearor-service-weather`          | `libsmearor_weather_service.so`          |

### Metapackage: `smearor-swipe-launcher-full`

A metapackage that pulls in the main launcher plus all standard widgets and services with a single command:

```bash
sudo apt install smearor-swipe-launcher-full
```

It `Depends` on the main package, all 13 widgets, and 16 standard services. It `Recommends` `smearor-service-loupedeck` and `smearor-service-streamdeck`
(hardware-specific, not useful for most users).

## Inter-Package Dependencies

### Widget → Service Dependencies

Widgets that depend on a service at runtime declare a `Depends` on the corresponding service package:

| Widget Package                      | Depends on (service packages)                                                                                         |
|-------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `smearor-plugin-app-launcher`       | `smearor-service-app-launcher`, `smearor-service-personalization`                                                     |
| `smearor-plugin-audio`              | `smearor-service-audio`, `smearor-service-personalization`                                                            |
| `smearor-plugin-button`             | `smearor-service-personalization`                                                                                     |
| `smearor-plugin-clock`              | `smearor-service-personalization`                                                                                     |
| `smearor-plugin-mpris`              | `smearor-service-mpris`, `smearor-service-personalization`                                                            |
| `smearor-plugin-network`            | `smearor-service-network`, `smearor-service-personalization`                                                          |
| `smearor-plugin-notifications`      | `smearor-service-notifications`, `smearor-service-personalization`                                                    |
| `smearor-plugin-power`              | `smearor-service-power`, `smearor-service-personalization`                                                            |
| `smearor-plugin-sysinfo`            | `smearor-service-sysinfo`, `smearor-service-personalization`                                                          |
| `smearor-plugin-voice-assistant`    | `smearor-service-voice-assistant`, `smearor-service-personalization`                                                  |
| `smearor-plugin-wallpaper`          | `smearor-service-wallpaper`, `smearor-service-personalization`                                                        |
| `smearor-plugin-weather`            | `smearor-service-weather`, `smearor-service-personalization`                                                          |
| `smearor-plugin-workspace-switcher` | `smearor-service-hyprland` \| `smearor-service-gnome` \| `smearor-service-wayland`, `smearor-service-personalization` |

### Service → Service Dependencies

| Service Package                   | Depends on                        |
|-----------------------------------|-----------------------------------|
| `smearor-service-voice-assistant` | `smearor-service-personalization` |
| `smearor-service-weather`         | `smearor-service-personalization` |

## Dynamic Dependency Resolution

System library dependencies are resolved dynamically at build time using `cargo-deb`'s `$auto` mechanism (via `dpkg-shlibdeps`). This avoids hardcoded package
names with ABI suffixes that break across Debian/Ubuntu releases:

```toml
[package.metadata.deb]
depends = "$auto, gtk4-layer-shell"
```

The only manually added dependency is `gtk4-layer-shell` (not auto-detected because it's linked via `pkg-config`).

For `voice-assistant-service`, optional ML libraries (llama.cpp, ROCm) are declared via `recommends` instead of `depends`, since the service has a CPU fallback.

## Config Installation Strategy

### System-wide defaults

The main package installs default configs to `/usr/share/smearor/` as templates. These use `name=` entries instead of `path=` for plugin resolution, allowing
the launcher to find plugins in `/usr/lib/smearor/` automatically.

### First-run bootstrap

On first launch, the launcher copies default configs from `/usr/share/smearor/` to `~/.config/smearor/` if they don't already exist. This works for all users
without requiring root privileges.

### Config discovery fallback order

The launcher discovers configs in this order:

1. CLI arguments (`--config`, `--services-config`)
2. Working directory (`*.toml`, `services.toml`, `wallpaper.toml`)
3. User config (`~/.config/smearor/launcher/*.toml`, `~/.config/smearor/services/services.toml`)
4. System default (`/usr/share/smearor/launcher/*.toml`, `/usr/share/smearor/services/services.toml`)

## Systemd User Service

Three systemd user service files are shipped:

- `smearor-swipe-launcher.service` — generic
- `smearor-swipe-launcher-hyprland.service` — Hyprland variant
- `smearor-swipe-launcher-gnome.service` — GNOME variant

Enable the appropriate service for your compositor:

```bash
systemctl --user enable --now smearor-swipe-launcher-hyprland
# or
systemctl --user enable --now smearor-swipe-launcher-gnome
```

## Building Packages

### Prerequisites

```bash
cargo install cargo-deb
```

### Build all packages

```bash
./scripts/build-deb.sh
```

This produces 33 `.deb` files in `target/debian/`:

- 1 main package
- 13 widget plugin packages
- 18 service packages
- 1 metapackage

### Build individual packages

```bash
cargo deb -p smearor-swipe-launcher
cargo deb -p smearor-clock-widget
cargo deb -p smearor-audio-service
cargo deb -p smearor-swipe-launcher-full
```

## File Tree After Installation

```
/usr/bin/
    smearor-swipe-launcher

/usr/lib/smearor/
    libsmearor_*.so                        (all widget and service plugins)

/usr/share/smearor/
    launcher/config.toml                   (default)
    services/services.toml                 (default)
    services/wallpaper.toml                (default)
    examples/launcher/*.toml               (example configs)
    examples/areas/*.toml                  (area config examples)

/usr/lib/systemd/user/
    smearor-swipe-launcher.service
    smearor-swipe-launcher-hyprland.service
    smearor-swipe-launcher-gnome.service

/usr/lib/udev/rules.d/
    52-loupedeck.rules
    52-streamdeck.rules
```

After first launch, user configs are created at `~/.config/smearor/`.
