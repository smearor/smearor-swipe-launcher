# Concept: GNOME Wallpaper Engine Integration

This document describes the concept for integrating the **GNOME Shell Extension `gnome-wallpaper-engine`** into the *Smearor Swipe Launcher* wallpaper system.
The extension provides GPU-accelerated live video wallpapers on GNOME (Wayland and X11) using `mpv` under the hood. Unlike `mpvpaper` (which renders on the
Layer Shell background layer), the GNOME extension is a native GNOME Shell component controlled entirely via **GSettings** — no process spawning or signal
management is required.

The integration extends the existing wallpaper service with a new `WallpaperType::Gnome` variant that drives the extension through `gsettings` CLI commands
instead of spawning external processes.

---

## 1. Goal & Motivation

The launcher already supports three wallpaper backends (`Video`, `Image`, `Application`) via `mpvpaper` and Layer Shell. On GNOME desktops, the
`gnome-wallpaper-engine` extension offers a superior experience:

- **Native GNOME integration:** Renders inside the GNOME Shell, not on a separate Layer Shell surface.
- **GPU acceleration:** Uses `mpv` with `hwdec=auto` for low CPU usage (~1-3%).
- **Auto-pause:** Pauses during fullscreen apps and on battery power.
- **GSettings control:** No process management needed — all control is via `gsettings set/get` commands.
- **Wayland and X11 support:** Works reliably on both display protocols.

The goal is to add a fourth wallpaper type (`Gnome`) that leverages this extension while reusing the existing service architecture, message flow, and widget UI.

---

## 2. Prerequisites

### 2.1 GNOME Shell Extension

The extension `gnome-wallpaper-engine` must be installed and enabled:

- **Repository:** https://github.com/achu94/gnome-wallpaper-engine
- **Requirements:** `mpv` and `ffmpeg` installed on the system.
- **Installation:** Via GNOME Extensions website, local ZIP import, or manual install into `~/.local/share/gnome-shell/extensions/`.

### 2.2 GSettings Schema

The extension exposes the following GSettings keys under the schema `org.gnome.shell.extensions.gnome-wallpaper-engine` (path:
`/org/gnome/shell/extensions/gnome-wallpaper-engine/`):

| Key                   | Type    | Default | Description                                                         |
|-----------------------|---------|---------|---------------------------------------------------------------------|
| `autostart`           | boolean | `true`  | Autostart the wallpaper on login.                                   |
| `show-indicator`      | boolean | `true`  | Show or hide the tray icon in the top panel.                        |
| `pause-on-fullscreen` | boolean | `true`  | Pause playback when a fullscreen application is active.             |
| `pause-on-battery`    | boolean | `false` | Pause playback when running on battery to save power.               |
| `current-wallpaper`   | string  | `''`    | Absolute path to the current wallpaper video file. Empty = stopped. |

The GSettings schema XML for reference:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<schemalist>
    <schema id="org.gnome.shell.extensions.gnome-wallpaper-engine"
            path="/org/gnome/shell/extensions/gnome-wallpaper-engine/">
        <key name="autostart" type="b">
            <default>true</default>
            <summary>Autostart the wallpaper</summary>
        </key>
        <key name="show-indicator" type="b">
            <default>true</default>
            <summary>Show or hide the tray icon</summary>
        </key>
        <key name="pause-on-fullscreen" type="b">
            <default>true</default>
            <summary>Pause when a fullscreen application is active</summary>
        </key>
        <key name="pause-on-battery" type="b">
            <default>false</default>
            <summary>Pause when running on battery</summary>
        </key>
        <key name="current-wallpaper" type="s">
            <default>''</default>
            <summary>Der Pfad zum aktuellen Hintergrund</summary>
        </key>
    </schema>
</schemalist>
```

### 2.3 GSettings Commands

The wallpaper is controlled primarily through the `current-wallpaper` key. Setting it to a video file path activates the wallpaper; setting it to an empty
string stops it. The `pause-on-fullscreen` and `pause-on-battery` keys are automatic behavior settings, not manual pause/resume controls.

```bash
# Start the wallpaper (set video file path)
gsettings set org.gnome.shell.extensions.gnome-wallpaper-engine current-wallpaper "/absolute/path/to/video.mp4"

# Stop the wallpaper (clear the path)
gsettings set org.gnome.shell.extensions.gnome-wallpaper-engine current-wallpaper ""

# Read current wallpaper state
gsettings get org.gnome.shell.extensions.gnome-wallpaper-engine current-wallpaper

# Configure automatic pause behavior (optional settings)
gsettings set org.gnome.shell.extensions.gnome-wallpaper-engine pause-on-fullscreen true
gsettings set org.gnome.shell.extensions.gnome-wallpaper-engine pause-on-battery false

# Configure autostart on login
gsettings set org.gnome.shell.extensions.gnome-wallpaper-engine autostart true

# Configure tray icon visibility
gsettings set org.gnome.shell.extensions.gnome-wallpaper-engine show-indicator false
```

---

## 3. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Wallpaper Widget         |                 | Wallpaper Service          |
| (subscribed to           |                 | (Singleton)                |
|  service.wallpaper.status)|                |                            |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Command Message                         |
             |  (select theme, start, stop)                |
             |===========================================> |
             |  Topic: "service.wallpaper.command"         |
             |                                             |
             |                                             |  2. GSettings CLI commands
             |                                             |     gsettings set ... current-wallpaper
             |                                             |     gsettings set ... pause-on-fullscreen
             |                                             |     gsettings set ... pause-on-battery
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.wallpaper.status"
             |                                             |     Payload: WallpaperStatusMessage { ... }
+--------------------------+                 +----------------------------+
```

The key difference from `Video`/`Image`/`Application` types: **no process spawning, no PID tracking, no SIGTERM/SIGKILL**. The GNOME extension manages its own
`mpv` process internally. The service only issues `gsettings` commands.

---

## 4. Changes to Model Crate (`model/wallpaper`)

### 4.1 New Wallpaper Type Variant

Add `Gnome` to the existing `WallpaperType` enum:

```rust
/// The type of wallpaper engine used by a theme.
/// Each variant determines which process the service spawns and how the config is interpreted.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum WallpaperType {
    /// Video slideshow using mpvpaper.
    #[default]
    Video,
    /// Image slideshow using mpvpaper.
    Image,
    /// Application-based wallpaper using a custom command or wrapper (e.g., smearor-wrot).
    Application,
    /// GNOME Shell extension gnome-wallpaper-engine (controlled via GSettings).
    Gnome,
}
```

### 4.2 New Gnome Config Struct

A new file `src/messages/gnome_config.rs` defines the GNOME-specific configuration:

```rust
use serde::Deserialize;
use serde::Serialize;

/// Configuration for a GNOME wallpaper engine theme.
/// The service controls the gnome-wallpaper-engine extension via GSettings CLI commands.
/// No process spawning or signal management is required.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct GnomeConfig {
    /// Absolute path to the video file to set as wallpaper.
    /// Setting this via `gsettings set current-wallpaper` activates the wallpaper.
    pub wallpaper_path: stabby::string::String,
    /// Whether to pause the wallpaper when a fullscreen application is active.
    /// Maps to the GSettings key `pause-on-fullscreen`.
    pub pause_on_fullscreen: bool,
    /// Whether to pause the wallpaper when running on battery power.
    /// Maps to the GSettings key `pause-on-battery`.
    pub pause_on_battery: bool,
    /// Whether to show the tray icon in the GNOME top panel.
    /// Maps to the GSettings key `show-indicator`.
    pub show_indicator: bool,
}
```

### 4.3 Extended Theme Config Enum

Add the `Gnome` variant to `WallpaperThemeConfig`:

```rust
/// Type-specific configuration for a wallpaper theme.
/// The variant must match the `wallpaper_type` field of the parent `WallpaperTheme`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[stabby::stabby]
pub enum WallpaperThemeConfig {
    /// Configuration for a video wallpaper theme.
    Video(VideoConfig),
    /// Configuration for an image wallpaper theme.
    Image(ImageConfig),
    /// Configuration for an application-based wallpaper theme.
    Application(AppConfig),
    /// Configuration for a GNOME wallpaper engine theme.
    Gnome(GnomeConfig),
}
```

### 4.4 Updated Nerd Font Icon Mapping

```rust
/// Returns the Nerd Font icon name for a given wallpaper type.
pub fn wallpaper_type_icon(wallpaper_type: &WallpaperType) -> &'static str {
    match wallpaper_type {
        WallpaperType::Video => "\u{f03d}",
        WallpaperType::Image => "\u{f03e}",
        WallpaperType::Application => "\u{f2d0}",
        WallpaperType::Gnome => "\u{f03d}",
    }
}
```

### 4.5 Updated `lib.rs` Exports

```rust
pub use messages::gnome_config::GnomeConfig;
```

---

## 5. Changes to Service Crate (`services/wallpaper`)

### 5.1 New GSettings Helper Module

A new file `src/gsettings.rs` provides helper functions for issuing GSettings commands:

```rust
use tokio::process::Command;
use tracing::debug;
use tracing::error;

/// The GSettings schema for the gnome-wallpaper-engine extension.
pub const GSETTINGS_SCHEMA: &str = "org.gnome.shell.extensions.gnome-wallpaper-engine";

/// Sets a boolean GSettings key for the GNOME wallpaper engine.
pub async fn set_boolean(key: &str, value: bool) {
    let value_str = if value { "true" } else { "false" };
    debug!("GSettings: set {} {} = {}", GSETTINGS_SCHEMA, key, value_str);
    let result = Command::new("gsettings")
        .args(["set", GSETTINGS_SCHEMA, key, value_str])
        .output()
        .await;
    if let Err(e) = result {
        error!("GSettings: failed to set {} {}: {}", key, value_str, e);
    }
}

/// Sets a string GSettings key for the GNOME wallpaper engine.
pub async fn set_string(key: &str, value: &str) {
    debug!("GSettings: set {} {} = \"{}\"", GSETTINGS_SCHEMA, key, value);
    let result = Command::new("gsettings")
        .args(["set", GSETTINGS_SCHEMA, key, value])
        .output()
        .await;
    if let Err(e) = result {
        error!("GSettings: failed to set {} \"{}\": {}", key, value, e);
    }
}

/// Gets a boolean GSettings key for the GNOME wallpaper engine.
/// Returns `None` if the command fails or the output cannot be parsed.
pub async fn get_boolean(key: &str) -> Option<bool> {
    let output = Command::new("gsettings")
        .args(["get", GSETTINGS_SCHEMA, key])
        .output()
        .await
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match stdout.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Gets a string GSettings key for the GNOME wallpaper engine.
/// Returns `None` if the command fails or the value is empty.
/// The output is wrapped in single quotes by gsettings (e.g., "'/path/to/video.mp4'").
/// An unset string key returns `''` (two single quotes) — this is treated as `None`.
/// Double-quoted empty strings (`""`) and whitespace-only values are also treated as `None`.
pub async fn get_string(key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", GSETTINGS_SCHEMA, key])
        .output()
        .await
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Handle edge cases: empty output, bare quotes, or whitespace-only strings
    if stdout.is_empty() || stdout == "''" || stdout == "\"\"" {
        return None;
    }
    // gsettings wraps string values in single quotes: '/path/to/video.mp4'
    // Strip surrounding single or double quotes
    let unquoted = stdout
        .trim_start_matches('\'')
        .trim_end_matches('\'')
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim();
    if unquoted.is_empty() { None } else { Some(unquoted.to_string()) }
}

/// Starts the wallpaper by setting the video file path.
/// The GNOME extension picks up the new path automatically and starts playback.
pub async fn start_wallpaper(path: &str) {
    set_string("current-wallpaper", path).await;
}

/// Stops the wallpaper by clearing the current wallpaper path.
/// Setting `current-wallpaper` to an empty string disables the wallpaper.
pub async fn stop_wallpaper() {
    set_string("current-wallpaper", "").await;
}

/// Sets whether to pause the wallpaper when a fullscreen application is active.
pub async fn set_pause_on_fullscreen(value: bool) {
    set_boolean("pause-on-fullscreen", value).await;
}

/// Sets whether to pause the wallpaper when running on battery power.
pub async fn set_pause_on_battery(value: bool) {
    set_boolean("pause-on-battery", value).await;
}

/// Sets whether to show the tray icon in the GNOME top panel.
pub async fn set_show_indicator(value: bool) {
    set_boolean("show-indicator", value).await;
}

/// Gets the current wallpaper video file path.
/// Returns `None` if no wallpaper is set or the command fails.
pub async fn get_current_wallpaper() -> Option<String> {
    get_string("current-wallpaper").await
}

/// Checks whether a wallpaper is currently active (non-empty path).
pub async fn is_wallpaper_active() -> bool {
    get_current_wallpaper()
        .await
        .map(|path| !path.is_empty())
        .unwrap_or(false)
}
```

### 5.2 Extended Start Logic

In `start_selected_wallpaper_theme`, add a branch for `WallpaperType::Gnome`:

```rust
// 3. Spawn the respective engine driver process
let monitor_pids: Vec<(u32, String) > = match theme.wallpaper_type {
WallpaperType::Video => spawn_mpvpaper_video( & theme)
.await
.map( | (pid, outputs) | outputs.into_iter().map( | o| (pid, o)).collect())
.unwrap_or_default(),
WallpaperType::Image => spawn_mpvpaper_image( & theme)
.await
.map( | (pid, outputs) | outputs.into_iter().map( | o| (pid, o)).collect())
.unwrap_or_default(),
WallpaperType::Application => spawn_application( & theme).await,
WallpaperType::Gnome => start_gnome_wallpaper( & theme).await,
};
```

### 5.3 New `start_gnome_wallpaper` Function

```rust
/// Starts a GNOME wallpaper engine theme by issuing GSettings commands.
/// Returns an empty vector since no process PIDs are tracked — the GNOME
/// extension manages its own mpv process internally.
async fn start_gnome_wallpaper(theme: &WallpaperTheme) -> Vec<(u32, String)> {
    let WallpaperThemeConfig::Gnome(config) = &theme.config else {
        return Vec::new();
    };

    // 1. Configure automatic pause behavior
    gsettings::set_pause_on_fullscreen(config.pause_on_fullscreen).await;
    gsettings::set_pause_on_battery(config.pause_on_battery).await;

    // 2. Configure tray icon visibility
    gsettings::set_show_indicator(config.show_indicator).await;

    // 3. Set the wallpaper video path — this activates the wallpaper automatically
    gsettings::start_wallpaper(&config.wallpaper_path).await;

    // No PIDs to track — the GNOME extension manages its own process.
    Vec::new()
}
```

### 5.4 Extended Stop Logic

In `stop_current_wallpaper_theme`, add a branch for GNOME themes. Since there are no PIDs to signal, the service disables the wallpaper via GSettings:

```rust
async fn stop_current_wallpaper_theme(
    state: &Arc<RwLock<WallpaperStatusMessage>>,
    kill_grace_period_ms: u64,
) {
    // Check if the current theme is a GNOME theme
    let is_gnome_theme = {
        let current = state.read().await;
        current.current_theme.as_ref().and_then(|name| {
            current.themes.iter().find(|t| t.name == *name)
        }).map(|t| t.wallpaper_type == WallpaperType::Gnome).unwrap_or(false)
    };

    if is_gnome_theme {
        // GNOME theme: stop by clearing current-wallpaper GSettings key
        gsettings::stop_wallpaper().await;

        let mut current = state.write().await;
        current.current_processes.clear();
        current.current_theme = None;
        return;
    }

    // ... existing SIGTERM/SIGKILL logic for Video/Image/Application ...
}
```

### 5.5 Extended Drop Implementation

The `Drop` implementation for `WallpaperService` should also stop the GNOME wallpaper engine if a GNOME theme is currently running. Since `Drop` is synchronous
and cannot use `.await`, a blocking `std::process::Command` call is used directly here. This is acceptable because Drop is the last operation before shutdown —
blocking briefly is not a concern:

```rust
impl Drop for WallpaperService {
    fn drop(&mut self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Check if a GNOME theme is currently running
            let is_gnome = self.state.read()
                .map(|s| {
                    s.current_theme.as_ref().and_then(|name| {
                        s.themes.iter().find(|t| t.name == *name)
                    }).map(|t| t.wallpaper_type == WallpaperType::Gnome).unwrap_or(false)
                })
                .unwrap_or(false);

            if is_gnome {
                // Drop is synchronous — use std::process::Command directly
                // instead of the async gsettings helpers.
                // GSETTINGS_SCHEMA is imported from the gsettings module.
                let _ = std::process::Command::new("gsettings")
                    .args(["set", gsettings::GSETTINGS_SCHEMA, "current-wallpaper", ""])
                    .output();
                debug!("Wallpaper service: stopped GNOME wallpaper engine on drop");
            }

            // ... existing SIGTERM logic for mpvpaper/Application processes ...
        }));

        if let Err(_) = result {
            error!("Wallpaper service: panic during drop, processes may not have been terminated");
        }
    }
}
```

### 5.6 Status Query (Optional Enhancement)

The service can query GSettings on startup or refresh to determine the actual GNOME wallpaper state:

```rust
/// Queries the current GNOME wallpaper engine state via GSettings.
/// Returns `Some((active, path))` if the extension is installed.
pub async fn query_gnome_state() -> Option<(bool, String)> {
    let path = gsettings::get_current_wallpaper().await?;
    let active = !path.is_empty();
    Some((active, path))
}
```

---

## 6. Configuration Example

### 6.1 GNOME Theme in `wallpaper.toml`

```toml
[[themes]]
name = "GNOME Live Wallpaper"
description = "Live video wallpaper via GNOME Shell extension"
preview_image_path = ""
wallpaper_type = "Gnome"
config.Gnome.wallpaper_path = "/home/aschaeffer/Videos/Backgrounds/nature.mp4"
config.Gnome.pause_on_fullscreen = true
config.Gnome.pause_on_battery = false
config.Gnome.show_indicator = true
```

### 6.2 Mixed Configuration

The service supports mixing GNOME themes with mpvpaper-based themes. The user can swipe between them and the service will use the appropriate backend:

```toml
[[themes]]
name = "Smearor"
description = "Die schoensten Hintergruende fuer den Tisch"
preview_image_path = ""
wallpaper_type = "Video"
config.Video.directory = "/home/aschaeffer/Videos/Backgrounds"
config.Video.outputs = ["ALL"]
config.Video.loop_playlist = true
config.Video.shuffle = false
config.Video.muted = true
config.Video.volume = 50
config.Video.speed_percentage = 100
config.Video.extra_arguments = []

[[themes]]
name = "GNOME Nature"
description = "Live wallpaper via GNOME extension"
preview_image_path = ""
wallpaper_type = "Gnome"
config.Gnome.wallpaper_path = "/home/aschaeffer/Videos/nature.mp4"
config.Gnome.pause_on_fullscreen = true
config.Gnome.pause_on_battery = false
config.Gnome.show_indicator = true
```

---

## 7. Key Differences from Existing Wallpaper Types

| Aspect                 | Video / Image / Application             | Gnome                                                  |
|------------------------|-----------------------------------------|--------------------------------------------------------|
| **Process management** | Service spawns and terminates processes | GNOME extension manages its own process                |
| **PID tracking**       | Per-monitor PIDs in `current_processes` | No PIDs — `current_processes` is empty                 |
| **Start mechanism**    | `tokio::process::Command::spawn`        | `gsettings set ... current-wallpaper <path>`           |
| **Stop mechanism**     | `SIGTERM` → poll → `SIGKILL`            | `gsettings set ... current-wallpaper ""`               |
| **Path setting**       | Passed as CLI argument to `mpvpaper`    | `gsettings set ... current-wallpaper`                  |
| **Pause behavior**     | Not supported (process kill only)       | `pause-on-fullscreen` / `pause-on-battery` (automatic) |
| **Output targeting**   | Per-monitor via `outputs` config        | GNOME extension handles output internally              |
| **Compositor**         | Wayland (Layer Shell)                   | GNOME (Wayland and X11)                                |
| **Drop cleanup**       | `SIGTERM` to tracked PIDs               | `gsettings set ... current-wallpaper ""`               |

---

## 8. Roadmap

### Phase 1: Model Extension — `model/wallpaper`

**Goal:** Add `Gnome` variant and `GnomeConfig` struct to the existing model crate.

**Dependencies:** Existing `model/wallpaper` crate.

**Order:**

1. Create `src/messages/gnome_config.rs` with `GnomeConfig` struct.
2. Add `Gnome` variant to `WallpaperType` enum in `src/messages/wallpaper_type.rs`.
3. Add `Gnome(GnomeConfig)` variant to `WallpaperThemeConfig` in `src/messages/theme_config.rs`.
4. Update `wallpaper_type_icon` to handle the `Gnome` variant.
5. Add `pub use messages::gnome_config::GnomeConfig;` to `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- `GnomeConfig` serializes and deserializes correctly.
- `WallpaperType::Gnome` round-trips through serde.

---

### Phase 2: Service Extension — `services/wallpaper`

**Goal:** Add GSettings-based control to the existing wallpaper service.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create `src/gsettings.rs` with helper functions (`set_boolean`, `set_string`, `get_boolean`, `get_string`, `start_wallpaper`, `stop_wallpaper`,
   `set_pause_on_fullscreen`, `set_pause_on_battery`, `set_show_indicator`, `get_current_wallpaper`, `is_wallpaper_active`).
2. Add `start_gnome_wallpaper` function in `src/process.rs` (or inline in `service.rs`).
3. Extend `start_selected_wallpaper_theme` with a `WallpaperType::Gnome` branch.
4. Extend `stop_current_wallpaper_theme` with GNOME-specific stop logic (clear `current-wallpaper`).
5. Extend `Drop` implementation to stop GNOME wallpaper on service shutdown.
6. Add unit tests for GSettings argument construction.
7. Run `cargo check` and `cargo test`.

**Exit criteria:**

- `gsettings` commands are issued correctly for start (set `current-wallpaper`), stop (clear `current-wallpaper`), and pause settings (`pause-on-fullscreen`,
  `pause-on-battery`).
- No process PIDs are tracked for GNOME themes.
- `Drop` stops the GNOME wallpaper engine by clearing `current-wallpaper`.
- Unit tests verify GSettings command construction.

---

### Phase 3: Widget Update — `plugins/wallpaper`

**Goal:** Ensure the existing widget handles `Gnome` wallpaper type gracefully.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Verify that `wallpaper_type_icon` returns a valid icon for `Gnome`.
2. Verify that the widget displays GNOME themes in the theme list.
3. Verify that swipe/press/long-press gestures work with GNOME themes.
4. No new widget code should be needed — the widget is type-agnostic.

**Exit criteria:**

- GNOME themes appear in the widget theme list.
- Selecting a GNOME theme shows the correct icon and name.
- Press starts the GNOME wallpaper via GSettings.
- Long press stops the GNOME wallpaper via GSettings.

---

### Phase 4: Configuration & Testing

**Goal:** Add sample GNOME themes and verify end-to-end behavior.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add a sample GNOME theme entry to `wallpaper.toml`:
   ```toml
   [[themes]]
   name = "GNOME Live Wallpaper"
   description = "Live video wallpaper via GNOME Shell extension"
   preview_image_path = ""
   wallpaper_type = "Gnome"
   config.Gnome.wallpaper_path = "/home/aschaeffer/Videos/Backgrounds/18246-290359913.mp4"
   config.Gnome.pause_on_fullscreen = true
   config.Gnome.pause_on_battery = false
   config.Gnome.show_indicator = true
   ```
2. Test start/stop via the widget (press starts, long press stops).
3. Test start/stop via MCP tools.
4. Test service shutdown (Drop) stops the GNOME wallpaper.
5. Test mixed configurations (GNOME + mpvpaper themes).
6. Run `cargo clippy` and `cargo fmt`.

**Exit criteria:**

- GNOME wallpaper starts and stops correctly via GSettings.
- Service shutdown stops the GNOME wallpaper by clearing `current-wallpaper`.
- Mixed configurations work (switching between GNOME and mpvpaper themes).
- No `unwrap`, `expect`, or `panic` in new code.
- `rustfmt` and `clippy` are clean.

---

### Summary of Order

```
Phase 1: model/wallpaper (add Gnome variant + GnomeConfig)
    |
    v
Phase 2: services/wallpaper (add gsettings.rs + start/stop logic)
    |
    v
Phase 3: plugins/wallpaper (verify type-agnostic widget)
    |
    v
Phase 4: configuration and end-to-end testing
```

---

## 9. Technical Notes

- **GSettings CLI:** The `gsettings` command is part of GNOME's dconf system and is available on any GNOME desktop. It communicates with the `dconf-service` via
  D-Bus. Commands execute instantaneously with no perceptible delay.
- **No process management:** Unlike `mpvpaper` themes, the GNOME backend does not spawn or terminate processes. The `gnome-wallpaper-engine` extension manages
  its own `mpv` instance internally. The service only issues GSettings commands.
- **No PID tracking:** `current_processes` is empty for GNOME themes. The widget status indicator shows "running" based on `current_theme` being set, not on PID
  presence.
- **No output targeting:** The GNOME extension handles monitor output internally. The `outputs` field from `VideoConfig`/`ImageConfig` does not apply to GNOME
  themes.
- **Automatic pause:** The GNOME extension supports automatic pause when fullscreen apps are active (`pause-on-fullscreen`) or when running on battery (
  `pause-on-battery`). These are behavioral settings, not manual pause/resume controls. There is no manual pause key in the GSettings schema.
- **GSettings string quoting:** `gsettings get` wraps string values in single quotes (e.g., `'/path/to/video.mp4'`). The helper function strips these quotes
  when parsing.
- **Error handling:** GSettings command failures are logged via `tracing::error` but do not panic. The service continues operating even if GSettings is
  unavailable (e.g., on non-GNOME desktops).
- **Cross-desktop compatibility:** If the GNOME extension is not installed, `gsettings` commands will fail silently (logged as errors). The service remains
  functional for other wallpaper types.
- **Drop safety:** The `Drop` implementation uses `catch_unwind` (already in place) and checks whether the current theme is a GNOME theme before clearing
  `current-wallpaper` via GSettings.

---

## 10. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** Changes are confined to the existing `model/wallpaper`, `services/wallpaper`, and `plugins/wallpaper` crates. No new crates are needed.
- **One struct per file:** `GnomeConfig` is defined in its own file `src/messages/gnome_config.rs`.
- **Service traits:** The service continues to implement `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Async runtime:** GSettings commands use `tokio::process::Command` with `.await`, integrating non-blocking into the async command loop. This prevents a
  hanging `dconf-service` from blocking the Tokio executor. The `Drop` implementation is the sole exception — it uses `std::process::Command` directly
  since `Drop` is synchronous and runs during shutdown where blocking is acceptable.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops.
- **FFI stability:** `GnomeConfig` carries `#[stabby::stabby]`. String fields use `stabby::string::String`.
- **No panic:** GSettings command failures are handled with `Result` and `Option`. No `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Dependencies:** No new external dependencies. `tokio::process::Command` (already in the dependency tree) is used for async GSettings calls;
  `std::process::Command` is used only in the synchronous `Drop` path.

---

*End of document.*
