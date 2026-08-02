# Debian Packaging Concept for Smearor Swipe Launcher

## Overview

This concept describes how to build Debian packages for the Smearor Swipe Launcher using `cargo-deb`. The packaging strategy uses **one package per
plugin/service**
to avoid glob-pattern limitations in `cargo-deb` and to allow fine-grained installation control.

## Package Architecture

### Package: `smearor-swipe-launcher` (main package)

Contains the main binary, systemd user service files, default user configs, example configs, and udev rules.

**Contents:**

- Binary: `target/release/smearor-swipe-launcher` → `/usr/bin/`
- Systemd user service: `smearor-swipe-launcher.service` → `/usr/lib/systemd/user/`
- Systemd user service (Hyprland variant): `smearor-swipe-launcher-hyprland.service` → `/usr/lib/systemd/user/`
- Systemd user service (GNOME variant): `smearor-swipe-launcher-gnome.service` → `/usr/lib/systemd/user/`
- Default launcher config: `configs/launcher/minimal.toml` → `/usr/share/smearor/launcher/config.toml`
- Default services config: `configs/services/services.toml` → `/usr/share/smearor/services/services.toml`
- Default wallpaper config: `configs/services/wallpaper.toml` → `/usr/share/smearor/services/wallpaper.toml`
- Example launcher configs: `configs/launcher/*.toml` (except minimal.toml) → `/usr/share/smearor/examples/launcher/`
- Example area configs: `configs/areas/*.toml` → `/usr/share/smearor/examples/areas/`
- Udev rules: `resources/udev/*.rules` → `/usr/lib/udev/rules.d/`
- Postinst script: creates `/usr/lib/smearor/` plugin directory (user config bootstrap is handled by the launcher at runtime — see Config Installation Strategy)

**Note on resources:** The following resources are compiled into the binary at build time via `include_str!`/`include_bytes!`/
`glib_build_tools::compile_resources`:

- `resources/style.css` (CSS theme)
- `resources/web/*` (web server templates, CSS, JS, fonts)
- `resources/NerdFontsSymbolsOnly/*` (GTK gresource)
- `resources/voice-assistant-system-prompt.txt` (voice assistant default prompt)

No separate resource installation is needed.

### Plugin Packages (Widgets)

Each widget plugin is a separate Debian package containing a single `.so` file. All packages depend on `smearor-swipe-launcher`.

| Crate Name                       | Package Name                        | .so File                               | Install Path        |
|----------------------------------|-------------------------------------|----------------------------------------|---------------------|
| `smearor-app-launcher-widget`    | `smearor-plugin-app-launcher`       | `libsmearor_app_launcher_widget.so`    | `/usr/lib/smearor/` |
| `smearor-audio-widget`           | `smearor-plugin-audio`              | `libsmearor_audio_widget.so`           | `/usr/lib/smearor/` |
| `smearor-button-widget`          | `smearor-plugin-button`             | `libsmearor_button_widget.so`          | `/usr/lib/smearor/` |
| `smearor-clock-widget`           | `smearor-plugin-clock`              | `libsmearor_clock_widget.so`           | `/usr/lib/smearor/` |
| `smearor-mpris-widget`           | `smearor-plugin-mpris`              | `libsmearor_mpris_widget.so`           | `/usr/lib/smearor/` |
| `smearor-network-widget`         | `smearor-plugin-network`            | `libsmearor_network_widget.so`         | `/usr/lib/smearor/` |
| `smearor-notifications-widget`   | `smearor-plugin-notifications`      | `libsmearor_notifications_widget.so`   | `/usr/lib/smearor/` |
| `smearor-power-widget`           | `smearor-plugin-power`              | `libsmearor_power_widget.so`           | `/usr/lib/smearor/` |
| `smearor-sysinfo-widget`         | `smearor-plugin-sysinfo`            | `libsmearor_sysinfo_widget.so`         | `/usr/lib/smearor/` |
| `smearor-voice-assistant-widget` | `smearor-plugin-voice-assistant`    | `libsmearor_voice_assistant_widget.so` | `/usr/lib/smearor/` |
| `smearor-wallpaper-widget`       | `smearor-plugin-wallpaper`          | `libsmearor_wallpaper_widget.so`       | `/usr/lib/smearor/` |
| `smearor-weather-widget`         | `smearor-plugin-weather`            | `libsmearor_weather_widget.so`         | `/usr/lib/smearor/` |
| `smearor-workspace-switcher`     | `smearor-plugin-workspace-switcher` | `libsmearor_workspace_switcher.so`     | `/usr/lib/smearor/` |

**Excluded:** `smearor-render-utils` (regular library dependency, not a cdylib plugin).

### Service Packages

Each service plugin is a separate Debian package containing a single `.so` file. All packages depend on `smearor-swipe-launcher`.

| Crate Name                         | Package Name                       | .so File                                 | Install Path        |
|------------------------------------|------------------------------------|------------------------------------------|---------------------|
| `smearor-app-launcher-service`     | `smearor-service-app-launcher`     | `libsmearor_app_launcher_service.so`     | `/usr/lib/smearor/` |
| `smearor-audio-service`            | `smearor-service-audio`            | `libsmearor_audio_service.so`            | `/usr/lib/smearor/` |
| `smearor-gnome-service`            | `smearor-service-gnome`            | `libsmearor_gnome_service.so`            | `/usr/lib/smearor/` |
| `smearor-http-service`             | `smearor-service-http`             | `libsmearor_http_service.so`             | `/usr/lib/smearor/` |
| `smearor-hyprland-service`         | `smearor-service-hyprland`         | `libsmearor_hyprland_service.so`         | `/usr/lib/smearor/` |
| `smearor-loupedeck-service`        | `smearor-service-loupedeck`        | `libsmearor_loupedeck_service.so`        | `/usr/lib/smearor/` |
| `smearor-mpris-service`            | `smearor-service-mpris`            | `libsmearor_mpris_service.so`            | `/usr/lib/smearor/` |
| `smearor-network-service`          | `smearor-service-network`          | `libsmearor_network_service.so`          | `/usr/lib/smearor/` |
| `smearor-notifications-service`    | `smearor-service-notifications`    | `libsmearor_notifications_service.so`    | `/usr/lib/smearor/` |
| `smearor-personalization-service`  | `smearor-service-personalization`  | `libsmearor_personalization_service.so`  | `/usr/lib/smearor/` |
| `smearor-power-service`            | `smearor-service-power`            | `libsmearor_power_service.so`            | `/usr/lib/smearor/` |
| `smearor-streamdeck-service`       | `smearor-service-streamdeck`       | `libsmearor_streamdeck_service.so`       | `/usr/lib/smearor/` |
| `smearor-sysinfo-service`          | `smearor-service-sysinfo`          | `libsmearor_sysinfo_service.so`          | `/usr/lib/smearor/` |
| `smearor-terminal-command-service` | `smearor-service-terminal-command` | `libsmearor_terminal_command_service.so` | `/usr/lib/smearor/` |
| `smearor-voice-assistant-service`  | `smearor-service-voice-assistant`  | `libsmearor_voice_assistant_service.so`  | `/usr/lib/smearor/` |
| `smearor-wallpaper-service`        | `smearor-service-wallpaper`        | `libsmearor_wallpaper_service.so`        | `/usr/lib/smearor/` |
| `smearor-wayland-service`          | `smearor-service-wayland`          | `libsmearor_wayland_service.so`          | `/usr/lib/smearor/` |
| `smearor-weather-service`          | `smearor-service-weather`          | `libsmearor_weather_service.so`          | `/usr/lib/smearor/` |

### Metapackage: `smearor-swipe-launcher-full`

A metapackage that pulls in the main launcher plus all standard widgets and services with a single `apt install smearor-swipe-launcher-full`.

**Contents:** No files — `Depends` only.

**Depends:**

```
smearor-swipe-launcher (>= 0.1.0),
smearor-plugin-app-launcher (>= 0.1.0),
smearor-plugin-audio (>= 0.1.0),
smearor-plugin-button (>= 0.1.0),
smearor-plugin-clock (>= 0.1.0),
smearor-plugin-mpris (>= 0.1.0),
smearor-plugin-network (>= 0.1.0),
smearor-plugin-notifications (>= 0.1.0),
smearor-plugin-power (>= 0.1.0),
smearor-plugin-sysinfo (>= 0.1.0),
smearor-plugin-voice-assistant (>= 0.1.0),
smearor-plugin-wallpaper (>= 0.1.0),
smearor-plugin-weather (>= 0.1.0),
smearor-plugin-workspace-switcher (>= 0.1.0),
smearor-service-app-launcher (>= 0.1.0),
smearor-service-audio (>= 0.1.0),
smearor-service-gnome (>= 0.1.0),
smearor-service-http (>= 0.1.0),
smearor-service-hyprland (>= 0.1.0),
smearor-service-mpris (>= 0.1.0),
smearor-service-network (>= 0.1.0),
smearor-service-notifications (>= 0.1.0),
smearor-service-personalization (>= 0.1.0),
smearor-service-power (>= 0.1.0),
smearor-service-sysinfo (>= 0.1.0),
smearor-service-terminal-command (>= 0.1.0),
smearor-service-voice-assistant (>= 0.1.0),
smearor-service-wallpaper (>= 0.1.0),
smearor-service-wayland (>= 0.1.0),
smearor-service-weather (>= 0.1.0)
```

**Recommends:**

```
smearor-service-loupedeck (>= 0.1.0),
smearor-service-streamdeck (>= 0.1.0)
```

**Note:** `loupedeck-service` and `streamdeck-service` are `Recommends`
instead of `Depends` because they require specific hardware (Loupedeck / Stream Deck devices) and are not useful for most users.

**Implementation:** Create a minimal crate `packages/full/Cargo.toml` with no source files, only `[package.metadata.deb]`:

```toml
[package]
name = "smearor-swipe-launcher-full"
version.workspace = true
edition.workspace = true

[package.metadata.deb]
name = "smearor-swipe-launcher-full"
depends = "smearor-swipe-launcher (>= 0.1.0), ..."
recommends = "smearor-service-loupedeck (>= 0.1.0), smearor-service-streamdeck (>= 0.1.0)"
assets = []
```

## Config Installation Strategy

### System-wide defaults (shipped in main package)

These files are installed to `/usr/share/smearor/` as **templates**:

```
/usr/share/smearor/launcher/config.toml          (from configs/launcher/minimal.toml)
/usr/share/smearor/services/services.toml         (from configs/services/services.toml)
/usr/share/smearor/services/wallpaper.toml        (from configs/services/wallpaper.toml)
/usr/share/smearor/examples/launcher/*.toml       (all other launcher configs)
/usr/share/smearor/examples/areas/*.toml          (area configs)
```

### User config installation (postinst script)

The `postinst` script copies default configs to the user's XDG config directory on first installation. It runs as the invoking user (dpkg runs postinst as root
for system packages, so we use a different approach — see below).

**Problem:** `postinst` runs as root, but configs must go into `~/.config/smearor/`
for the actual desktop user. Solutions:

1. **Postinst creates system-wide skeleton** at `/etc/skel/.config/smearor/`
   → New users get configs automatically via `useradd -m` skeleton.
2. **Systemd user service ExecStartPre** copies configs if missing.
3. **Launcher itself** copies defaults on first run (preferred — see Phase 4).

**Recommended approach:** The launcher itself checks on startup whether
`~/.config/smearor/launcher/config.toml` exists. If not, it copies from
`/usr/share/smearor/launcher/config.toml`. Same for `services.toml` and
`wallpaper.toml`. This is the most robust approach and works for all users.

### Config-Discovery extension (IMPLEMENTED)

The `ConfigDiscoveryService` has been extended with system-wide fallback locations. All three discovery methods now follow a 4-priority fallback order:

**Launcher configs:**

1. CLI `--config`
2. `*.toml` in working directory (excluding `services.toml`, `wallpaper.toml`)
3. `~/.config/smearor/launcher/*.toml` (user)
4. `/usr/share/smearor/launcher/*.toml` (system default)

**Services config:**

1. CLI `--services-config`
2. `services.toml` in working directory
3. `~/.config/smearor/services/services.toml` (user)
4. `/usr/share/smearor/services/services.toml` (system default)

**Wallpaper config:**

1. `wallpaper.toml` in working directory
2. `~/.config/smearor/services/wallpaper.toml` (user)
3. `/usr/share/smearor/services/wallpaper.toml` (system default)

**Implementation:** `config/discovery.rs` — `discover_launcher_configs()`,
`discover_services_config()`, and new `discover_wallpaper_config()` all include the system-wide fallback as last priority.

### Wallpaper config loading (IMPLEMENTED)

The wallpaper config (`wallpaper.toml`) is loaded by the wallpaper service plugin, not by the launcher's `ConfigDiscoveryService` directly. The flow:

1. `services.toml` has a `[wallpaper]` section with optional `config_path`
2. If `config_path` is empty or omitted, the launcher resolves the path via
   `ConfigDiscoveryService::discover_wallpaper_config()` and **injects** it into the wallpaper service's config JSON before loading the service
3. The wallpaper service receives `config_path` in its config and loads themes from that file
4. As a fallback (if the launcher does not inject a path, e.g. standalone service), the wallpaper service has its own `discover_wallpaper_config()`
   with the same 3-priority fallback order

**Implementation:**

- `services/wallpaper/src/config.rs` — `config_path` defaults to empty string,
  `load_or_discover_themes()` handles empty path, own `discover_wallpaper_config()`
  with system-wide fallback
- `application.rs` — `load_services()` injects resolved wallpaper config path into `[wallpaper]` config JSON when `config_path` is not explicitly set
- `configs/services/services.toml` — `config_path` commented out with explanation

## Systemd User Service

### `smearor-swipe-launcher.service` (generic)

```ini
[Unit]
Description=Smearor Swipe Launcher
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/bin/smearor-swipe-launcher
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

### `smearor-swipe-launcher-hyprland.service` (Hyprland variant)

```ini
[Unit]
Description=Smearor Swipe Launcher (Hyprland)
After=hyprland-session.target
Requires=hyprland-session.target

[Service]
Type=simple
ExecStart=/usr/bin/smearor-swipe-launcher
Restart=on-failure
RestartSec=5

[Install]
WantedBy=hyprland-session.target
```

### `smearor-swipe-launcher-gnome.service` (GNOME variant)

```ini
[Unit]
Description=Smearor Swipe Launcher (GNOME)
After=gnome-session.target
Requires=gnome-session.target

[Service]
Type=simple
ExecStart=/usr/bin/smearor-swipe-launcher
Restart=on-failure
RestartSec=5

[Install]
WantedBy=gnome-session.target
```

Users enable the appropriate service:

```bash
systemctl --user enable --now smearor-swipe-launcher-hyprland
# or
systemctl --user enable --now smearor-swipe-launcher-gnome
```

## Debian Dependencies

### Main package (`smearor-swipe-launcher`)

Uses `cargo-deb`'s `$auto` mechanism to dynamically resolve system library dependencies via `dpkg-shlibdeps` at build time. This avoids hardcoded package names
with ABI suffixes (e.g. `libglib2.0-0t64`) that break across Debian/Ubuntu releases:

```toml
[package.metadata.deb]
depends = "$auto, gtk4-layer-shell"
```

`$auto` runs `dpkg-shlibdeps` against the compiled binary, which automatically determines the correct `libgtk-4-1`, `libglib2.0-0t64`, `libssl3`, etc. for the
build system's distribution. The only manually added dependency is
`gtk4-layer-shell` (not auto-detected because it's linked via `pkg-config`
and may not appear in `ldd` output on all systems).

**Reference:** The `ldd` analysis (see Open Question #2) confirmed the full list of linked system libraries for documentation purposes, but the actual Debian
dependencies are resolved dynamically at build time.

### Plugin/Service packages

Each plugin/service package depends on `smearor-swipe-launcher (>= 0.1.0)`. Additional per-plugin dependencies may be needed (e.g. `libpulse0` for audio).

Widgets that depend on a service at runtime (via message broker topics) must declare a `Depends` on the corresponding service package. Services that depend on
other services must also declare `Depends` accordingly.

#### Widget → Service dependencies

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

**Note:** `smearor-plugin-workspace-switcher` uses an alternative dependency (`|`) because it works with any compositor service (Hyprland, GNOME, or Wayland).

**Note:** `smearor-plugin-clock` depends on `smearor-service-personalization`
for locale/theme awareness, but has no dedicated clock service (the clock model is used directly by the widget).

#### Service → Service dependencies

| Service Package                   | Depends on (service packages)     |
|-----------------------------------|-----------------------------------|
| `smearor-service-voice-assistant` | `smearor-service-personalization` |
| `smearor-service-weather`         | `smearor-service-personalization` |

All other services have no inter-service runtime dependencies.

**How dependencies were determined:** By analyzing which `smearor-*-model`
crates each widget/service imports and which topics it subscribes to or broadcasts. The `smearor-model-mcp` crate is excluded as it is the MCP protocol layer
built into the launcher core, not a service dependency.

## Implementation Phases

### Phase 1: System Config Templates

**Goal:** Create system-wide config files that use `name=` instead of `path=`.

**Tasks:**

1. Create `configs/system/launcher/config.toml` — copy of `minimal.toml` with
   `name=` entries instead of `path=`
2. Create `configs/system/services/services.toml` — copy of `services.toml` with
   `name=` entries instead of `path=`
3. Create `configs/system/services/wallpaper.toml` — copy of `wallpaper.toml`
   (no plugin paths, only theme definitions — may need no changes)

**Exit criteria:** System configs use only `name=` and are valid TOML.

### Phase 2: Config-Discovery Extension (DONE)

**Goal:** Launcher finds configs in `/usr/share/smearor/` as last fallback.

**Status:** Implemented in `config/discovery.rs`.

**What was done:**

1. Extended `discover_launcher_configs()` with Priority 4: `/usr/share/smearor/launcher/*.toml`
2. Extended `discover_services_config()` with Priority 4: `/usr/share/smearor/services/services.toml`
3. Added `discover_wallpaper_config()` method with 3-priority fallback:
   working dir → `~/.config/smearor/services/wallpaper.toml` → `/usr/share/smearor/services/wallpaper.toml`
4. Made `config_path` optional in `WallpaperServiceConfig` (defaults to empty string)
5. Launcher injects resolved wallpaper config path into service config JSON in `load_services()`
6. Wallpaper service has own fallback `discover_wallpaper_config()` for standalone operation

**Exit criteria:** Met — launcher starts without CLI args and finds configs in
`/usr/share/smearor/` when no user configs exist.

### Phase 3: First-Run Config Bootstrap

**Goal:** Launcher copies default configs to `~/.config/smearor/` on first run.

**Tasks:**

1. Add `bootstrap_user_configs()` function in launcher startup
2. Check for `~/.config/smearor/launcher/config.toml` — if missing, copy from
   `/usr/share/smearor/launcher/config.toml`
3. Check for `~/.config/smearor/services/services.toml` — if missing, copy from
   `/usr/share/smearor/services/services.toml`
4. Check for `~/.config/smearor/services/wallpaper.toml` — if missing, copy from
   `/usr/share/smearor/services/wallpaper.toml`
5. Create directories with `std::fs::create_dir_all()` as needed
6. Log bootstrap actions at `info` level

**Exit criteria:** Fresh user account gets working configs after first launch.

### Phase 4: Systemd User Service Files

**Goal:** Ship systemd user service files for Hyprland and GNOME.

**Tasks:**

1. Create `resources/systemd/smearor-swipe-launcher.service` (generic)
2. Create `resources/systemd/smearor-swipe-launcher-hyprland.service`
3. Create `resources/systemd/smearor-swipe-launcher-gnome.service`
4. Install to `/usr/lib/systemd/user/` in main package

**Exit criteria:** `systemctl --user enable --now smearor-swipe-launcher-hyprland`
starts the launcher.

### Phase 5: cargo-deb Configuration (Main Package)

**Goal:** `[package.metadata.deb]` section in `smearor-swipe-launcher/Cargo.toml`.

**Tasks:**

1. Add `cargo-deb` to `[build-dependencies]` or install via `cargo install cargo-deb`
2. Add `[package.metadata.deb]` section with:
    - `maintainer`, `license`, `extended-description`
    - `depends = "$auto, gtk4-layer-shell"` (auto-resolved via `dpkg-shlibdeps`)
    - `assets` list (binary, configs, examples, systemd units, udev rules)
    - `maintainer-scripts` directory for postinst/prerm
3. Create `debian/postinst` — create `/usr/lib/smearor/` directory
4. Create `debian/prerm` — optional cleanup

**Exit criteria:** `cargo deb -p smearor-swipe-launcher` produces a valid `.deb`.

### Phase 6: cargo-deb Configuration (Plugin/Service Packages)

**Goal:** Each plugin/service crate gets its own `[package.metadata.deb]` section.

**Tasks:**

1. For each plugin crate (13 widgets), add `[package.metadata.deb]` to its
   `Cargo.toml`:
   ```toml
   [package.metadata.deb]
   name = "smearor-plugin-clock"
   depends = "$auto, smearor-swipe-launcher (>= 0.1.0), smearor-service-personalization (>= 0.1.0)"
   assets = [
       ["target/release/libsmearor_clock_widget.so", "/usr/lib/smearor/", "644"]
   ]
   ```
   Widgets with a dedicated service also depend on that service, e.g.:
   ```toml
   # smearor-plugin-audio
   depends = "$auto, smearor-swipe-launcher (>= 0.1.0), smearor-service-audio (>= 0.1.0), smearor-service-personalization (>= 0.1.0)"
   ```
   The workspace-switcher widget uses an alternative dependency:
   ```toml
   # smearor-plugin-workspace-switcher
   depends = "$auto, smearor-swipe-launcher (>= 0.1.0), smearor-service-hyprland (>= 0.1.0) | smearor-service-gnome (>= 0.1.0) | smearor-service-wayland (>= 0.1.0), smearor-service-personalization (>= 0.1.0)"
   ```
2. For each service crate (18 services), add `[package.metadata.deb]` to its
   `Cargo.toml`:
   ```toml
   [package.metadata.deb]
   name = "smearor-service-audio"
   depends = "$auto, smearor-swipe-launcher (>= 0.1.0)"
   assets = [
       ["target/release/libsmearor_audio_service.so", "/usr/lib/smearor/", "644"]
   ]
   ```
   Services that depend on other services, e.g.:
   ```toml
   # smearor-service-voice-assistant
   depends = "$auto, smearor-swipe-launcher (>= 0.1.0), smearor-service-personalization (>= 0.1.0)"
   ```
3. Per-plugin system library dependencies are auto-resolved by `$auto`
   (via `dpkg-shlibdeps`). Plugins with libraries not detectable by
   `dpkg-shlibdeps` (e.g. `libllama` for voice-assistant-service) must add them manually. Use `recommends` instead of `depends` for optional libs (e.g. ROCm/HIP
   for voice-assistant-service).

**Exit criteria:** `cargo deb -p smearor-clock-widget` produces a valid `.deb`
that installs `libsmearor_clock_widget.so` to `/usr/lib/smearor/`.

### Phase 7: Build Script / Makefile

**Goal:** Automate the full release + packaging workflow.

**Tasks:**

1. Create `scripts/build-deb.sh`:
   ```bash
   #!/bin/bash
   set -euo pipefail
   
   # 1. Release build for entire workspace
   cargo build --release --workspace
   
   # 2. Build main package
   cargo deb -p smearor-swipe-launcher
   
   # 3. Build all plugin packages
   for pkg in smearor-app-launcher-widget smearor-audio-widget ...; do
       cargo deb -p "$pkg"
   done
   
   # 4. Build all service packages
   for pkg in smearor-app-launcher-service smearor-audio-service ...; do
       cargo deb -p "$pkg"
   done
   
   # 5. Build metapackage
   cargo deb -p smearor-swipe-launcher-full
   
   echo "All .deb files in target/debian/"
   ```

**Exit criteria:** Single script builds all 33 `.deb` packages (32 + 1 metapackage).

## Open Questions

1. ~~**Wallpaper config loading:** How is `wallpaper.toml` currently loaded?~~
   **Resolved.** The wallpaper service loads `wallpaper.toml` itself. The
   `config_path` in `services.toml` `[wallpaper]` section is now optional. When omitted, the launcher resolves the path via `ConfigDiscoveryService::
   discover_wallpaper_config()` and injects it into the service config. The wallpaper service has its own fallback discovery for standalone operation.
2. **Per-plugin system dependencies:** System library dependencies are resolved dynamically at build time via `cargo-deb`'s `$auto` mechanism
   (`dpkg-shlibdeps`). The `ldd` analysis below serves as reference only.

   **Group 1: Minimal service plugins** (no GUI, no unique system deps beyond what the launcher already provides):
   `app-launcher-service`, `http-service`, `hyprland-service`, `loupedeck-service`,
   `mpris-service`, `network-service`, `notifications-service`, `personalization-service`,
   `power-service`, `streamdeck-service`, `sysinfo-service`, `terminal-command-service`,
   `wallpaper-service`, `wayland-service`, `weather-service`
   → `$auto` resolves no additional system libs beyond the launcher package.

   **Group 2: GTK widget plugins** (all 13 widgets link against `libgtk-4` and related libraries, but these are already dependencies of the main package):
   All widgets → `$auto` resolves system libs transitively via the launcher package.

   **Group 3: Plugins with additional system deps not auto-detected:**

   | Plugin | Additional deps to declare manually |
         |---|---|
   | `audio-service` | (none — `libpulse0` is auto-detected by `dpkg-shlibdeps`) |
   | `voice-assistant-service` | `recommends = "libllama0"` (llama.cpp, not in standard repos); ROCm libs via `recommends` (optional, CPU fallback) |

   **Note:** `voice-assistant-service` has heavy ML dependencies (llama.cpp, ROCm/HIP). These are not available as standard Debian packages and must be
   installed separately or bundled. Use `recommends` instead of `depends` for optional libs, since the service has a CPU fallback.
3. ~~**Version numbering:** All crates share `version.workspace = true` (0.1.0)
   except some plugins that have `version = "0.1.0"` hardcoded. Should be unified.~~
   **Resolved.** Every service and widget plugin now has an explicit version number.
4. ~~**APT repository:** Should we set up a Debian repository for `apt install`
   or just distribute `.deb` files directly?~~
   **Out of scope.**
5. ~~**Desktop entry:** Should we ship a `.desktop` file as an alternative to systemd user service for autostart?~~
   **No.**

## File Tree After Installation

```
/usr/bin/
    smearor-swipe-launcher

/usr/lib/smearor/
    libsmearor_app_launcher_widget.so      (from smearor-plugin-app-launcher)
    libsmearor_audio_widget.so             (from smearor-plugin-audio)
    libsmearor_button_widget.so            (from smearor-plugin-button)
    libsmearor_clock_widget.so             (from smearor-plugin-clock)
    libsmearor_mpris_widget.so             (from smearor-plugin-mpris)
    libsmearor_network_widget.so           (from smearor-plugin-network)
    libsmearor_notifications_widget.so     (from smearor-plugin-notifications)
    libsmearor_power_widget.so             (from smearor-plugin-power)
    libsmearor_sysinfo_widget.so           (from smearor-plugin-sysinfo)
    libsmearor_voice_assistant_widget.so   (from smearor-plugin-voice-assistant)
    libsmearor_wallpaper_widget.so         (from smearor-plugin-wallpaper)
    libsmearor_weather_widget.so           (from smearor-plugin-weather)
    libsmearor_workspace_switcher.so       (from smearor-plugin-workspace-switcher)
    libsmearor_app_launcher_service.so     (from smearor-service-app-launcher)
    libsmearor_audio_service.so            (from smearor-service-audio)
    libsmearor_gnome_service.so            (from smearor-service-gnome)
    libsmearor_http_service.so             (from smearor-service-http)
    libsmearor_hyprland_service.so         (from smearor-service-hyprland)
    libsmearor_loupedeck_service.so        (from smearor-service-loupedeck)
    libsmearor_mpris_service.so            (from smearor-service-mpris)
    libsmearor_network_service.so          (from smearor-service-network)
    libsmearor_notifications_service.so    (from smearor-service-notifications)
    libsmearor_personalization_service.so  (from smearor-service-personalization)
    libsmearor_power_service.so            (from smearor-service-power)
    libsmearor_streamdeck_service.so       (from smearor-service-streamdeck)
    libsmearor_sysinfo_service.so          (from smearor-service-sysinfo)
    libsmearor_terminal_command_service.so (from smearor-service-terminal-command)
    libsmearor_voice_assistant_service.so  (from smearor-service-voice-assistant)
    libsmearor_wallpaper_service.so        (from smearor-service-wallpaper)
    libsmearor_wayland_service.so          (from smearor-service-wayland)
    libsmearor_weather_service.so          (from smearor-service-weather)

/usr/share/smearor/
    launcher/
        config.toml                        (default, from minimal.toml)
    services/
        services.toml                      (default)
        wallpaper.toml                     (default)
    examples/
        launcher/
            config.toml                    (full example)
            example-bottom.toml
            example-left.toml
            example-right.toml
            example-top.toml
            layout_profiles.toml
            minimal.toml
            rotated-90.toml
            streamdeck.toml
            streamcontrollerx.toml
            web.toml
        areas/
            scroll_menu.toml

/usr/lib/systemd/user/
    smearor-swipe-launcher.service
    smearor-swipe-launcher-hyprland.service
    smearor-swipe-launcher-gnome.service

/usr/lib/udev/rules.d/
    52-loupedeck.rules
    52-streamdeck.rules
```

## User Config Tree (after first launch)

```
~/.config/smearor/
    launcher/
        config.toml                        (copied from /usr/share/smearor/launcher/)
    services/
        services.toml                      (copied from /usr/share/smearor/services/)
        wallpaper.toml                     (copied from /usr/share/smearor/services/)
```
