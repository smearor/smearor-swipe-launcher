# Concept: Wallpaper Service & Widget

This document describes the concept for a **Wallpaper Service** and a **Wallpaper Widget** in the *Smearor Swipe Launcher*. The service manages wallpaper
processes (video slideshows, image slideshows, and application-based wallpapers) using `mpvpaper` and Layer Shell background rendering. The widget provides a
GTK4 tile for theme selection via swipe gestures and press interactions.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/wallpaper`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/wallpaper`):** Singleton background service that spawns and terminates wallpaper processes, manages theme state, and broadcasts
   status updates.
3. **Widget Crate (`plugins/wallpaper`):** Pure GTK4 UI that displays theme previews and allows selection via swipe/press gestures.

---

## 1. Goal & Motivation

The launcher should be able to control the desktop wallpaper in a flexible and configurable way. Three types of wallpapers are supported:

- **Video Wallpaper:** Uses `mpvpaper` to play video files as a slideshow on the Layer Shell background layer.
- **Image Wallpaper:** Uses `mpvpaper` to display image files as a slideshow on the Layer Shell background layer.
- **Application Wallpaper:** Uses a custom application or a wrapper like `smearor-wrot` to render an application (e.g., a weather app) on the Layer Shell
  background layer.

All wallpaper types render behind all other windows on the Wayland background layer. The service manages process lifecycle (start, stop, restart) and the widget
provides a user-friendly interface for theme selection and activation.

---

## 2. System Architecture & Data Flow

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
             |                                             |  2. Spawn / terminate process
             |                                             |     mpvpaper / smearor-wrot / ...
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.wallpaper.status"
             |                                             |     Payload: WallpaperStatusMessage { ... }
+--------------------------+                 +----------------------------+
             |                                             |
             |                                             |  4. MCP Tools
             |                                             |     add_wallpaper_theme
             |                                             |     remove_wallpaper_theme
             |                                             |     select_wallpaper_theme
             |                                             |     stop_current_wallpaper_process
             |                                             |     start_selected_wallpaper_process
+--------------------------+                 +----------------------------+
```

The service also registers **MCP resources** and **MCP tools** so that AI clients can query wallpaper state and control themes programmatically (e.g., "Set the
Halloween wallpaper theme").

---

## 3. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path                  | Responsibility                                                |
|-------------|-----------------------|---------------------------------------------------------------|
| **Model**   | `model/wallpaper/`    | Shared structs, enums, topics, and message formats            |
| **Service** | `services/wallpaper/` | Process lifecycle, theme management, MCP resources, MCP tools |
| **Widget**  | `plugins/wallpaper/`  | GTK4 tile UI, preview image, swipe/press gesture handling     |

---

## 4. Model Crate (`model/wallpaper`)

### 4.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.wallpaper.command";
pub const TOPIC_STATUS: &str = "service.wallpaper.status";
```

### 4.2 Wallpaper Type Enum

```rust
/// The type of wallpaper engine used by a theme.
/// Each variant determines which process the service spawns and how the config is interpreted.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum WallpaperType {
    /// Video slideshow using mpvpaper.
    Video,
    /// Image slideshow using mpvpaper.
    Image,
    /// Application-based wallpaper using a custom command or wrapper (e.g., smearor-wrot).
    #[default]
    Application,
}
```

### 4.3 Video Config

```rust
/// Configuration for a video wallpaper theme.
/// All fields are passed to mpvpaper when spawning the video slideshow.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct VideoConfig {
    /// Directory containing the video files to play.
    pub directory: stabby::string::String,
    /// Target display outputs. `["ALL"]` targets all monitors; otherwise specific names like `["DP-1", "HDMI-A-1"]`.
    pub outputs: stabby::vec::Vec<stabby::string::String>,
    /// Whether to loop the playlist infinitely.
    pub loop_playlist: bool,
    /// Whether to shuffle the video order.
    pub shuffle: bool,
    /// Whether to mute audio.
    pub muted: bool,
    /// Playback volume (0-100). Only relevant if audio is not muted.
    pub volume: u32,
    /// Playback speed as percentage (100 = normal, 50 = half speed, 200 = double speed).
    pub speed_percentage: u32,
    /// Custom array of extra arguments passed directly to mpvpaper.
    pub extra_arguments: stabby::vec::Vec<stabby::string::String>,
}
```

### 4.4 Image Config

```rust
/// Configuration for an image wallpaper theme.
/// All fields are passed to mpvpaper when spawning the image slideshow.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ImageConfig {
    /// Directory containing the image files to display.
    pub directory: stabby::string::String,
    /// Target display outputs. `["ALL"]` targets all monitors; otherwise specific names like `["DP-1", "HDMI-A-1"]`.
    pub outputs: stabby::vec::Vec<stabby::string::String>,
    /// Time in milliseconds each image is displayed before advancing (e.g., 30000 = 30 seconds).
    pub display_duration_ms: u32,
    /// Whether to shuffle the image order.
    pub shuffle: bool,
    /// Whether to enable transition effects between images.
    pub transitions: bool,
    /// Transition effect name (e.g., "fade", "slide"). Only used if `transitions` is true.
    pub transition_effect: stabby::string::String,
    /// Transition duration in milliseconds (e.g., 1500 = 1.5 seconds).
    pub transition_duration_ms: u32,
    /// Custom array of extra arguments passed directly to mpvpaper.
    pub extra_arguments: stabby::vec::Vec<stabby::string::String>,
}
```

### 4.5 App Config

```rust
/// Configuration for an application-based wallpaper theme.
/// The service spawns the configured command with its arguments on the Layer Shell background layer.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct AppConfig {
    /// Base executable command (e.g., "smearor-wrot").
    pub command: stabby::string::String,
    /// Target display outputs. `["ALL"]` targets all monitors; otherwise specific names like `["DP-1", "HDMI-A-1"]`.
    pub outputs: stabby::vec::Vec<stabby::string::String>,
    /// Array of arguments passed to the command (e.g., ["--layer", "background", "--output", "{monitor}", "/path/to/app"]).
    /// The placeholder `{monitor}` is replaced at runtime with each target monitor name (e.g., "DP-1"), spawning one process per output.
    pub arguments: stabby::vec::Vec<stabby::string::String>,
}
```

### 4.6 Theme Config Enum

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
}
```

### 4.7 Wallpaper Theme

```rust
/// A complete wallpaper theme definition.
/// Each theme specifies its type, display metadata, preview image, and type-specific configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct WallpaperTheme {
    /// Unique name of the theme.
    pub name: stabby::string::String,
    /// Human-readable description of the theme.
    pub description: stabby::string::String,
    /// File path to the preview image shown in the widget.
    pub preview_image_path: stabby::string::String,
    /// The wallpaper engine type.
    pub wallpaper_type: WallpaperType,
    /// Type-specific configuration for this theme.
    pub config: WallpaperThemeConfig,
}
```

### 4.8 Command Action Enum

```rust
/// Actions the wallpaper service can perform on request.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum WallpaperCommandAction {
    /// Select a theme without starting it (updates `selected_theme` only).
    #[default]
    SelectTheme,
    /// Start the currently selected theme (stops any running theme first).
    StartSelected,
    /// Stop the currently running wallpaper process.
    StopCurrent,
    /// Refresh the status broadcast.
    Refresh,
}
```

### 4.9 Command Message (Widget -> Service)

```rust
/// Command message sent by widgets or MCP clients to the wallpaper service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct WallpaperCommandMessage {
    /// The action to perform.
    pub action: WallpaperCommandAction,
    /// Name of the theme to select (only used with `SelectTheme`).
    pub theme_name: stabby::string::String,
}
```

### 4.10 Monitor Process & Status Message (Service -> Widget)

When a wallpaper theme spans multiple monitors, the service tracks each monitor's process independently. This enables per-monitor process management and future
support for running different themes on different monitors simultaneously (e.g., a video slideshow on DP-1 and an image slideshow on HDMI-A-1).

```rust
/// Tracks the PID of a wallpaper process running on a specific monitor.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct MonitorProcess {
    /// Monitor name (e.g., "DP-1", "HDMI-A-1").
    pub monitor: stabby::string::String,
    /// PID of the wallpaper process running on this monitor (0 if none).
    pub process_id: u32,
}

/// Complete wallpaper status message broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct WallpaperStatusMessage {
    /// Name of the currently running theme (`None` if stopped).
    pub current_theme: stabby::option::Option<stabby::string::String>,
    /// PIDs of active wallpaper processes per monitor (empty if none).
    pub current_processes: stabby::vec::Vec<MonitorProcess>,
    /// Name of the theme currently staged/focused in the UI.
    pub selected_theme: stabby::option::Option<stabby::string::String>,
    /// List of all configured themes.
    pub themes: stabby::vec::Vec<WallpaperTheme>,
    /// Index of the selected theme in the `themes` list.
    pub selected_theme_index: usize,
}
```

### 4.11 Nerd Font Icon Mapping

| Type        | Icon | Nerd Font Name         |
|-------------|------|------------------------|
| Video       | 󰕧   | `nf-md-movie_play`     |
| Image       | 󰋹   | `nf-md-image_multiple` |
| Application | 󰋜   | `nf-md-application`    |
| Stopped     | 󰅖   | `nf-md-close`          |
| Running     | 󰐦   | `nf-md-play_circle`    |

```rust
/// Returns the Nerd Font icon name for a given wallpaper type.
pub fn wallpaper_type_icon(wallpaper_type: &WallpaperType) -> &'static str {
    match wallpaper_type {
        WallpaperType::Video => "nf-md-movie_play",
        WallpaperType::Image => "nf-md-image_multiple",
        WallpaperType::Application => "nf-md-application",
    }
}
```

### 4.12 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::app_config::AppConfig;
pub use messages::command::WallpaperCommandAction;
pub use messages::command::WallpaperCommandMessage;
pub use messages::image_config::ImageConfig;
pub use messages::icon::wallpaper_type_icon;
pub use messages::monitor_process::MonitorProcess;
pub use messages::status::WallpaperStatusMessage;
pub use messages::theme::WallpaperTheme;
pub use messages::theme_config::WallpaperThemeConfig;
pub use messages::video_config::VideoConfig;
pub use messages::wallpaper_type::WallpaperType;
```

---

## 5. Service Crate (`services/wallpaper`)

### 5.1 File Structure

- `service.rs` - `WallpaperService` struct and trait implementations
- `config.rs` - `WallpaperServiceConfig` struct and parsing
- `process.rs` - Process spawning and termination logic
- `lib.rs` - `service_plugin!` macro invocation

### 5.2 Service Implementation

```rust
pub struct WallpaperService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WallpaperServiceConfig,
    pub state: Arc<RwLock<WallpaperStatusMessage>>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<WallpaperCommand>,
}

/// Internal command union for the service event loop.
pub enum WallpaperCommand {
    /// Select a theme without starting it.
    SelectTheme(String),
    /// Start the currently selected theme (stops any running theme first).
    StartSelected,
    /// Stop the currently running wallpaper process.
    StopCurrent,
    /// Refresh the status broadcast.
    Refresh,
    /// Permanently add a new theme to the configuration store.
    AddTheme(WallpaperTheme),
    /// Permanently remove a theme from the configuration store.
    RemoveTheme(String),
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<WallpaperCommandMessage>>` - Processes commands from widgets and MCP clients
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 5.3 Service Configuration

```rust
/// Configuration for the wallpaper service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WallpaperServiceConfig {
    /// List of all configured wallpaper themes.
    pub themes: Vec<WallpaperTheme>,
    /// Name of the default theme that the service starts with.
    pub default_theme: String,
    /// Path to the configuration file where themes are persisted.
    pub config_path: String,
    /// Whether to automatically start the default theme on service initialization.
    pub auto_start: bool,
    /// Grace period in milliseconds before sending SIGKILL after SIGTERM.
    pub kill_grace_period_ms: u64,
}

impl Default for WallpaperServiceConfig {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            default_theme: String::new(),
            config_path: String::from("wallpaper.toml"),
            auto_start: false,
            kill_grace_period_ms: 3000,
        }
    }
}
```

### 5.4 Service State

The service maintains the following state in `WallpaperStatusMessage`:

- **`current_theme`:** Name of the currently running theme (`None` if stopped).
- **`current_processes`:** PIDs of active wallpaper processes per monitor (empty if none). Each entry maps a monitor name to a PID, enabling per-monitor process
  management.
- **`selected_theme`:** Name of the theme currently staged/focused in the UI.
- **`themes`:** List of all configured themes.
- **`selected_theme_index`:** Index of the selected theme in the `themes` list.

### 5.5 Lifecycle Methods

#### `select_theme(name)`

Updates `selected_theme` and `selected_theme_index` without modifying the active background. The widget uses this to show a preview of the next theme without
starting it.

```rust
async fn select_theme(state: &Arc<RwLock<WallpaperStatusMessage>>, name: &str) {
    let mut current = state.write().await;
    if let Some(index) = current.themes.iter().position(|t| t.name == name) {
        current.selected_theme = Some(name.to_string());
        current.selected_theme_index = index;
    }
}
```

#### `start_selected_wallpaper_theme()`

Invokes `stop_current_wallpaper_theme()`, resolves configuration pathing and monitor targets, spawns the respective engine driver process, stores the child PID,
and sets `current_theme = selected_theme`.

```rust
async fn start_selected_wallpaper_theme(
    state: &Arc<RwLock<WallpaperStatusMessage>>,
    kill_grace_period_ms: u64,
) {
    // 1. Stop any currently running theme
    stop_current_wallpaper_theme(state, kill_grace_period_ms).await;

    // 2. Resolve the selected theme
    let theme = {
        let current = state.read().await;
        current.selected_theme.as_ref().and_then(|name| {
            current.themes.iter().find(|t| t.name == *name).cloned()
        })
    };

    let Some(theme) = theme else {
        return;
    };

    // 3. Spawn the respective engine driver process
    let monitor_pids: Vec<(u32, String)> = match theme.wallpaper_type {
        WallpaperType::Video => spawn_mpvpaper_video(&theme)
            .await
            .map(|(pid, outputs)| outputs.into_iter().map(|o| (pid, o)).collect())
            .unwrap_or_default(),
        WallpaperType::Image => spawn_mpvpaper_image(&theme)
            .await
            .map(|(pid, outputs)| outputs.into_iter().map(|o| (pid, o)).collect())
            .unwrap_or_default(),
        WallpaperType::Application => spawn_application(&theme).await,
    };

    // 4. Store per-monitor PIDs and set current_theme
    let mut current = state.write().await;
    current.current_processes.clear();
    for (pid, monitor) in &monitor_pids {
        current.current_processes.push(MonitorProcess {
            monitor: monitor.clone(),
            process_id: *pid,
        });
    }
    if !monitor_pids.is_empty() {
        current.current_theme = Some(theme.name);
    }
}
```

#### `stop_current_wallpaper_theme()`

Gracefully terminates (SIGTERM) all processes listed in `current_processes`. Polls every 100ms via `kill(pid, None)` to detect early exit. If a process does
not exit within `kill_grace_period_ms`, sends SIGKILL. Clears state variables.

```rust
async fn stop_current_wallpaper_theme(
    state: &Arc<RwLock<WallpaperStatusMessage>>,
    kill_grace_period_ms: u64,
) {
    let pids: Vec<u32> = {
        let current = state.read().await;
        current.current_processes.iter().map(|p| p.process_id).collect()
    };

    // Send SIGTERM to all unique PIDs
    let unique_pids: std::collections::HashSet<u32> = pids.into_iter().collect();
    for pid in &unique_pids {
        if *pid > 0 {
            let _ = nix::sys::signal::kill(
                Pid::from_raw(*pid as i32),
                Signal::SIGTERM,
            );
        }
    }

    // Poll every 100ms to check if processes have exited.
    // This avoids waiting the full grace period when processes
    // terminate quickly after SIGTERM, making theme switches snappier.
    if !unique_pids.is_empty() {
        let poll_interval = Duration::from_millis(100);
        let deadline = tokio::time::Instant::now() + Duration::from_millis(kill_grace_period_ms);
        loop {
            tokio::time::sleep(poll_interval).await;

            // Check which PIDs are still alive (kill with signal None is a no-op
            // that returns Err(ESRCH) if the process no longer exists).
            let alive: Vec<u32> = unique_pids
                .iter()
                .copied()
                .filter(|pid| {
                    *pid > 0
                        && nix::sys::signal::kill(Pid::from_raw(*pid as i32), None).is_ok()
                })
                .collect();

            if alive.is_empty() || tokio::time::Instant::now() >= deadline {
                // Send SIGKILL to any remaining processes
                for pid in &alive {
                    let _ = nix::sys::signal::kill(
                        Pid::from_raw(*pid as i32),
                        Signal::SIGKILL,
                    );
                }
                break;
            }
        }
    }

    let mut current = state.write().await;
    current.current_processes.clear();
    current.current_theme = None;
}
```

### 5.6 Process Spawning

#### Video Slideshow (`mpvpaper`)

```rust
/// Spawns mpvpaper for a video wallpaper theme and returns the child PID.
async fn spawn_mpvpaper_video(theme: &WallpaperTheme) -> Option<(u32, Vec<String>)> {
    let WallpaperThemeConfig::Video(config) = &theme.config else {
        return None;
    };

    let outputs = resolve_outputs(&config.outputs);
    let mut args = vec![
        outputs.join(","),
        "--loop-playlist=inf".to_string(),
    ];

    if config.shuffle {
        args.push("--shuffle".to_string());
    }
    if config.muted {
        args.push("--mute".to_string());
    } else {
        args.push(format!("--volume={}", config.volume));
    }
    args.push(format!("--speed={}", config.speed_percentage as f64 / 100.0));

    for extra in &config.extra_arguments {
        args.push(extra.to_string());
    }

    args.push(config.directory.to_string());

    spawn_process("mpvpaper", &args).map(|pid| (pid, outputs))
}
```

#### Image Slideshow (`mpvpaper`)

```rust
/// Spawns mpvpaper for an image wallpaper theme and returns the child PID.
async fn spawn_mpvpaper_image(theme: &WallpaperTheme) -> Option<(u32, Vec<String>)> {
    let WallpaperThemeConfig::Image(config) = &theme.config else {
        return None;
    };

    let outputs = resolve_outputs(&config.outputs);
    let mut args = vec![outputs.join(",")];

    // Image slideshow configuration (convert ms to seconds for mpvpaper)
    args.push(format!("--image-display-duration={}", config.display_duration_ms as f64 / 1000.0));

    if config.shuffle {
        args.push("--shuffle".to_string());
    }

    if config.transitions {
        args.push(format!("--transition-effect={}", config.transition_effect));
        args.push(format!("--transition-duration={}", config.transition_duration_ms as f64 / 1000.0));
    }

    for extra in &config.extra_arguments {
        args.push(extra.to_string());
    }

    // Use a playlist file or directory glob for images
    args.push(config.directory.to_string());

    spawn_process("mpvpaper", &args).map(|pid| (pid, outputs))
}
```

#### Application (`smearor-wrot` or custom)

```rust
/// Spawns an application-based wallpaper and returns per-monitor PIDs.
/// If any argument contains the `{monitor}` placeholder, the service spawns
/// one process per target output, replacing the placeholder with the monitor name.
/// If no placeholder is present, a single process is spawned for all outputs.
async fn spawn_application(theme: &WallpaperTheme) -> Vec<(u32, String)> {
    let WallpaperThemeConfig::Application(config) = &theme.config else {
        return Vec::new();
    };

    // Determine target outputs. Application themes default to ["ALL"] if no
    // explicit outputs are configured (applications often manage their own output).
    let outputs = resolve_outputs(&config.outputs);

    let has_placeholder = config.arguments.iter().any(|a| a.contains("{monitor}"));

    if !has_placeholder {
        // No placeholder: spawn once, assign to all outputs
        let args: Vec<String> = config.arguments.iter().map(|s| s.to_string()).collect();
        let result = spawn_process(&config.command, &args);
        if let Some(pid) = result {
            return outputs.into_iter().map(|o| (pid, o)).collect();
        }
        return Vec::new();
    }

    // Placeholder present: spawn one process per monitor
    let mut results = Vec::new();
    for monitor in &outputs {
        let args: Vec<String> = config
            .arguments
            .iter()
            .map(|s| s.replace("{monitor}", monitor))
            .collect();
        if let Some(pid) = spawn_process(&config.command, &args) {
            results.push((pid, monitor.clone()));
        }
    }
    results
}
```

#### Output Resolution

```rust
/// Resolves the outputs list. `["ALL"]` returns all connected monitor names.
fn resolve_outputs(outputs: &[String]) -> Vec<String> {
    if outputs.iter().any(|o| o == "ALL") {
        // Query connected outputs via GNOME Mutter D-Bus DisplayConfig API
        // (org.gnome.Mutter.DisplayConfig), or fall back to wlr-randr / hyprctl
        // monitors on wlroots/Hyprland compositors.
        list_connected_monitors()
    } else {
        outputs.to_vec()
    }
}
```

#### Generic Process Spawn

```rust
/// Spawns a child process and returns its PID.
fn spawn_process(command: &str, args: &[String]) -> Option<u32> {
    match tokio::process::Command::new(command)
        .args(args)
        .spawn()
    {
        Ok(child) => Some(child.id()),
        Err(error) => {
            tracing::error!("Failed to spawn wallpaper process '{command}': {error}");
            None
        }
    }
}
```

### 5.7 Background Update Loop

On initialization, the service spawns a dedicated OS thread with a single-threaded Tokio runtime. The runtime processes incoming commands and broadcasts status
updates.

```rust
async fn run_update_loop(
    config: WallpaperServiceConfig,
    state: Arc<RwLock<WallpaperStatusMessage>>,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<WallpaperCommand>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    // Auto-start the default theme if configured
    if config.auto_start && !config.default_theme.is_empty() {
        select_theme(&state, &config.default_theme).await;
        start_selected_wallpaper_theme(&state, config.kill_grace_period_ms).await;
        broadcast_status(&state, &broadcaster).await;
    }

    loop {
        tokio::select! {
            Some(command) = command_receiver.recv() => {
                match command {
                    WallpaperCommand::SelectTheme(name) => {
                        select_theme(&state, &name).await;
                        broadcast_status(&state, &broadcaster).await;
                    }
                    WallpaperCommand::StartSelected => {
                        start_selected_wallpaper_theme(&state, config.kill_grace_period_ms).await;
                        broadcast_status(&state, &broadcaster).await;
                    }
                    WallpaperCommand::StopCurrent => {
                        stop_current_wallpaper_theme(&state, config.kill_grace_period_ms).await;
                        broadcast_status(&state, &broadcaster).await;
                    }
                    WallpaperCommand::Refresh => {
                        broadcast_status(&state, &broadcaster).await;
                    }
                    WallpaperCommand::AddTheme(theme) => {
                        add_theme_to_config(&config.config_path, &theme).await;
                        let mut current = state.write().await;
                        current.themes.push(theme);
                        drop(current);
                        broadcast_status(&state, &broadcaster).await;
                    }
                    WallpaperCommand::RemoveTheme(name) => {
                        remove_theme_from_config(&config.config_path, &name).await;
                        let mut current = state.write().await;
                        current.themes.retain(|t| t.name != name);
                        if current.selected_theme.as_deref() == Some(&name) {
                            current.selected_theme = None;
                            current.selected_theme_index = 0;
                        }
                        if current.current_theme.as_deref() == Some(&name) {
                            drop(current);
                            stop_current_wallpaper_theme(&state, config.kill_grace_period_ms).await;
                        }
                        drop(current);
                        broadcast_status(&state, &broadcaster).await;
                    }
                }
            }
        }
    }
}
```

### 5.8 Required Trait Implementations

```rust
impl MessageHandler<FfiEnvelopePayload<WallpaperCommandMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<WallpaperCommandMessage>, _sender_id: &str) {
        let inner = message.into_inner();
        let command = match inner.action {
            WallpaperCommandAction::SelectTheme => WallpaperCommand::SelectTheme(inner.theme_name.to_string()),
            WallpaperCommandAction::StartSelected => WallpaperCommand::StartSelected,
            WallpaperCommandAction::StopCurrent => WallpaperCommand::StopCurrent,
            WallpaperCommandAction::Refresh => WallpaperCommand::Refresh,
        };
        let _ = self.command_sender.send(command);
    }
}

impl MessageBroadcaster for WallpaperService {}

impl MessageTopicBroadcaster for WallpaperService {}

impl PluginMetaGetter for WallpaperService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WallpaperService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for WallpaperService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            if envelope.type_id == FfiEnvelopePayload::<WallpaperCommandMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<WallpaperCommandMessage>>::handle_envelope_message(self, envelope);
            }
        }
    }
}
```

### 5.9 MCP Resources

The service registers the following MCP resources via the Plugin-Resource-Registry:

| URI                  | Description                                                        | Source type              |
|----------------------|--------------------------------------------------------------------|--------------------------|
| `wallpaper://status` | Current wallpaper status (running theme, PID, selected theme).     | `WallpaperStatusMessage` |
| `wallpaper://themes` | List of all configured wallpaper themes with their configurations. | `Vec<WallpaperTheme>`    |

Example `wallpaper://status` response:

```json
{
  "current_theme": "Halloween",
  "current_processes": [
    {
      "monitor": "DP-1",
      "process_id": 12345
    },
    {
      "monitor": "HDMI-A-1",
      "process_id": 12345
    }
  ],
  "selected_theme": "Halloween",
  "themes": [
    ...
  ],
  "selected_theme_index": 2
}
```

### 5.10 MCP Tools

The service registers the following MCP tools via the Plugin-Tool-Registry:

| Tool                               | Description                                                                                       | Parameters                                       |
|------------------------------------|---------------------------------------------------------------------------------------------------|--------------------------------------------------|
| `add_wallpaper_theme`              | Permanently appends a new theme to the configuration store.                                       | `name: string`, `type: string`, `config: object` |
| `remove_wallpaper_theme`           | Deletes a theme from the store.                                                                   | `name: string`                                   |
| `select_wallpaper_theme`           | Executes `select_theme(name)`. Updates `selected_theme` without starting.                         | `name: string`                                   |
| `stop_current_wallpaper_process`   | Stops the active background engine immediately.                                                   | -                                                |
| `start_selected_wallpaper_process` | Executes the atomic cycle of stopping the current process and spawning the staged selected theme. | -                                                |

> **MCP tool naming convention:** Tool names use `snake_case` with underscores, never dots. Dots in tool names cause schema validation failures in LLM
> gateways (Windsurf, Claude, etc.). This is consistent with existing tools like `sysinfo_refresh` and `get_current_time`.

**Example JSON schema for `add_wallpaper_theme`:**

```json
{
  "name": "add_wallpaper_theme",
  "description": "Permanently appends a new wallpaper theme to the configuration store.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Unique name of the wallpaper theme"
      },
      "type": {
        "type": "string",
        "enum": [
          "Video",
          "Image",
          "Application"
        ],
        "description": "The wallpaper engine type"
      },
      "config": {
        "type": "object",
        "description": "Type-specific configuration (VideoConfig, ImageConfig, or AppConfig)"
      },
      "description": {
        "type": "string",
        "description": "Human-readable description of the theme"
      },
      "preview_image_path": {
        "type": "string",
        "description": "File path to the preview image shown in the widget"
      }
    },
    "required": [
      "name",
      "type",
      "config"
    ]
  }
}
```

**Example JSON schema for `select_wallpaper_theme`:**

```json
{
  "name": "select_wallpaper_theme",
  "description": "Selects a wallpaper theme by name without starting it. Updates the selected_theme state.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Name of the theme to select"
      }
    },
    "required": [
      "name"
    ]
  }
}
```

**Example JSON schema for `start_selected_wallpaper_process`:**

```json
{
  "name": "start_selected_wallpaper_process",
  "description": "Starts the currently selected wallpaper theme. Stops any running theme first, then spawns the engine process.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

**Example JSON schema for `stop_current_wallpaper_process`:**

```json
{
  "name": "stop_current_wallpaper_process",
  "description": "Stops the currently running wallpaper process immediately.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

---

## 6. Widget Crate (`plugins/wallpaper`)

### 6.1 Overview

The Wallpaper Widget is a GTK4 tile that displays the preview image of the currently selected wallpaper theme. Swipe gestures cycle through available themes (
without starting them), press starts or restarts the selected theme, and long press stops the running wallpaper.

### 6.2 File Structure

- `widget.rs` - `WallpaperWidget` struct and trait implementations
- `config.rs` - `WallpaperWidgetConfig` struct and parsing
- `preview.rs` - Preview image loading and rendering
- `lib.rs` - `widget_plugin!` macro invocation

### 6.3 Widget Configuration

```rust
/// Configuration for the wallpaper widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WallpaperWidgetConfig {
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Whether to show the theme name as a label overlay.
    pub show_theme_name: bool,
    /// Whether to show the wallpaper type icon.
    pub show_type_icon: bool,
    /// Whether to show the running/stopped status indicator.
    pub show_status_indicator: bool,
    /// Preview image width in pixels.
    pub preview_width: i32,
    /// Preview image height in pixels.
    pub preview_height: i32,
    /// Fallback icon when no preview image is available.
    pub fallback_icon: String,
    /// Message topic for single-click (opens the wallpaper area).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
}

impl Default for WallpaperWidgetConfig {
    fn default() -> Self {
        Self {
            width: 120,
            height: 120,
            show_theme_name: true,
            show_type_icon: true,
            show_status_indicator: true,
            preview_width: 100,
            preview_height: 100,
            fallback_icon: "nf-md-wallpaper".to_string(),
            click_topic: None,
            click_payload: None,
        }
    }
}
```

### 6.4 Widget Implementation

```rust
pub struct WallpaperWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WallpaperWidgetConfig,
    pub current_status: Rc<RefCell<Option<WallpaperStatusMessage>>>,
    pub preview_image: Rc<RefCell<Option<gtk4::Picture>>>,
    pub theme_label: Rc<RefCell<Option<gtk4::Label>>>,
    pub status_icon: Rc<RefCell<Option<gtk4::Label>>>,
}
```

> **GTK widget references:** GTK4 widgets (`gtk4::Box`, `gtk4::Picture`, `gtk4::Label`) are **not** `Send` or `Sync`. They must not be stored in
`Arc<RwLock<...>>` inside the plugin struct. Instead, widget references are captured inside `glib::clone!` closures or passed directly to
`glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state (`config`, `current_status`).

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<WallpaperStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 6.5 Gesture Handling

The widget uses GTK4 gesture controllers for touch/mouse interactions:

| Gesture        | Action                                                       | Command Sent                       |
|----------------|--------------------------------------------------------------|------------------------------------|
| **Swipe up**   | Select previous theme (does not start/restart)               | `SelectTheme(previous_theme_name)` |
| **Swipe down** | Select next theme (does not start/restart)                   | `SelectTheme(next_theme_name)`     |
| **Press**      | Start (if not running) or restart (if running) current theme | `StartSelected`                    |
| **Long press** | Stop the currently running wallpaper                         | `StopCurrent`                      |

```rust
fn setup_gestures(&self, container: &gtk4::Box) {
    let press_gesture = gtk4::GestureClick::new();
    let longpress_gesture = gtk4::GestureLongPress::new();
    let swipe_gesture = gtk4::GestureSwipe::new();

    // Press: Start or restart selected theme
    {
        let broadcaster = self.get_broadcaster();
        let status = self.current_status.clone();
        press_gesture.connect_released(move |_, _, _, _| {
            let action = WallpaperCommandAction::StartSelected;
            let message = WallpaperCommandMessage {
                action,
                theme_name: String::new(),
            };
            send_command(&broadcaster, message);
        });
    }

    // Long press: Stop current wallpaper
    {
        let broadcaster = self.get_broadcaster();
        longpress_gesture.connect_pressed(move |_, _, _| {
            let action = WallpaperCommandAction::StopCurrent;
            let message = WallpaperCommandMessage {
                action,
                theme_name: String::new(),
            };
            send_command(&broadcaster, message);
        });
    }

    // Swipe up/down: Select previous/next theme
    // GestureSwipe provides connect_swipe(velocity_x, velocity_y) which reacts
    // to the actual swipe velocity and cleanly separates vertical/horizontal direction,
    // avoiding false triggers from diagonal drags.
    {
        let broadcaster = self.get_broadcaster();
        let status = self.current_status.clone();
        swipe_gesture.connect_swipe(move |_, velocity_x, velocity_y| {
            // Only react to predominantly vertical swipes
            if velocity_y.abs() <= velocity_x.abs() {
                return;
            }
            let current = status.borrow();
            if let Some(ref current_status) = *current {
                let themes = &current_status.themes;
                if themes.is_empty() {
                    return;
                }
                let current_index = current_status.selected_theme_index;
                let new_index = if velocity_y < 0.0 {
                    // Swipe up: previous theme
                    if current_index == 0 {
                        themes.len() - 1
                    } else {
                        current_index - 1
                    }
                } else {
                    // Swipe down: next theme
                    (current_index + 1) % themes.len()
                };
                let theme_name = themes[new_index].name.to_string();
                let action = WallpaperCommandAction::SelectTheme;
                let message = WallpaperCommandMessage {
                    action,
                    theme_name,
                };
                send_command(&broadcaster, message);
            }
        });
    }

    container.add_controller(press_gesture.clone());
    container.add_controller(longpress_gesture.clone());
    container.add_controller(swipe_gesture.clone());
}
```

### 6.6 Preview Image Rendering

The widget loads the preview image from the `preview_image_path` of the currently selected theme. If the path is invalid or the file does not exist, the
fallback icon is shown.

```rust
fn update_preview(&self, theme: &WallpaperTheme) {
    let preview_path = theme.preview_image_path.to_string();
    let fallback_icon = self.config.fallback_icon.clone();

    let preview_weak = self.preview_image.clone();
    let label_weak = self.theme_label.clone();
    let show_type_icon = self.config.show_type_icon;
    let type_icon = wallpaper_type_icon(&theme.wallpaper_type);
    let theme_name = theme.name.to_string();

    glib::MainContext::default().spawn_local(async move {
        if let Some(picture) = preview_weak.borrow().as_ref() {
            if std::path::Path::new(&preview_path).exists() {
                let file = gtk4::gio::File::for_path(&preview_path);
                let texture = gtk4::Texture::from_file(&file);
                match texture {
                    Ok(tex) => picture.set_paintable(Some(&tex)),
                    Err(_) => {
                        // Show fallback icon as text
                        if let Some(label) = label_weak.borrow().as_ref() {
                            label.set_text(&fallback_icon);
                        }
                    }
                }
            } else {
                // Show fallback icon
                if let Some(label) = label_weak.borrow().as_ref() {
                    label.set_text(&fallback_icon);
                }
            }
        }

        // Update theme name label
        if let Some(label) = label_weak.borrow().as_ref() {
            if show_type_icon {
                label.set_text(&format!("{type_icon}  {theme_name}"));
            } else {
                label.set_text(&theme_name);
            }
        }
    });
}
```

### 6.7 State Synchronization

The widget subscribes to `service.wallpaper.status`. When a new `WallpaperStatusMessage` arrives:

1. The message is deserialized and stored in `current_status`.
2. The preview image is updated to reflect the `selected_theme`.
3. The theme name label is updated.
4. The status indicator (running/stopped) is updated based on `current_theme` and `current_processes`.
5. All GTK updates happen via `glib::MainContext::spawn_local`.

### 6.8 Widget Layout

```
+----------------------+
|                      |
|    [Preview Image]   |
|                      |
|  󰕧  Halloween       |
|  󰐦 Running          |
+----------------------+
```

- The preview image fills most of the tile.
- The theme name and type icon are shown as a label overlay at the bottom.
- The status indicator (running/stopped icon) is shown in the corner.

---

## 7. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
| Wallpaper Widget  |<--------|                   |-------->| Wallpaper Service |
| (tile in area)    |  Status |   Event Broker    | Command | (Singleton)       |
+---------+---------+ Broadcast +-------------------+ Broadcast +-------------------+
          |                                                 |
          | Swipe up/down: SelectTheme                      | Spawn/terminate
          | Press: StartSelected                            | mpvpaper / app
          | Long press: StopCurrent                         v
          v                                            +-------------------+
+-------------------+                                  | mpvpaper /        |
| Preview update    |                                  | smearor-wrot /    |
| (local state)     |                                  | custom process    |
+-------------------+                                  +-------------------+
```

---

## 8. Configuration Example

### 8.1 Service Registration in `services.toml`

```toml
[[services]]
id = "wallpaper"
path = "target/release/libsmearor_wallpaper_service.so"

[wallpaper]
default_theme = "Space"
auto_start = false
kill_grace_period_ms = 3000

[[wallpaper.themes]]
name = "Space"
description = "Nebula and galaxy video loops"
preview_image_path = "/path/to/previews/space.png"
wallpaper_type = "Video"

[wallpaper.themes.config.Video]
directory = "/path/to/wallpapers/space/videos"
outputs = ["ALL"]
loop_playlist = true
shuffle = true
muted = true
volume = 0
speed_percentage = 100
extra_arguments = ["--hwdec=auto"]

[[wallpaper.themes]]
name = "Nature"
description = "Forest and mountain image slideshow"
preview_image_path = "/path/to/previews/nature.png"
wallpaper_type = "Image"

[wallpaper.themes.config.Image]
directory = "/path/to/wallpapers/nature/images"
outputs = ["DP-1", "HDMI-A-1"]
display_duration_ms = 30000
shuffle = true
transitions = true
transition_effect = "fade"
transition_duration_ms = 1500
extra_arguments = []

[[wallpaper.themes]]
name = "WeatherApp"
description = "Weather application rendered as wallpaper"
preview_image_path = "/path/to/previews/weather.png"
wallpaper_type = "Application"

[wallpaper.themes.config.Application]
command = "smearor-wrot"
outputs = ["ALL"]
arguments = ["--layer", "background", "--output", "{monitor}", "/path/to/weather-app"]
```

### 8.2 Widget Configuration in `config.toml`

```toml
[[scroll_band.plugins]]
id = "wallpaper_widget"
path = "target/release/libsmearor_wallpaper_widget.so"

[wallpaper_widget]
width = 120
height = 120
show_theme_name = true
show_type_icon = true
show_status_indicator = true
preview_width = 100
preview_height = 100
fallback_icon = "nf-md-wallpaper"

# Click opens the wallpaper area
click_topic = "area.open"
click_payload = { area_id = "wallpaper_area" }
```

### 8.3 Halloween Theme Example (via MCP)

An AI client can add and start a Halloween theme via MCP tools:

```
1. add_wallpaper_theme(
     name: "Halloween",
     type: "Video",
     description: "Spooky Halloween video loops",
     preview_image_path: "/path/to/previews/halloween.png",
     config: {
       directory: "/path/to/wallpapers/halloween/videos",
       outputs: ["ALL"],
       loop_playlist: true,
       shuffle: true,
       muted: true,
       volume: 0,
       speed_percentage: 100,
       extra_arguments: ["--hwdec=auto"]
     }
   )

2. select_wallpaper_theme(name: "Halloween")

3. start_selected_wallpaper_process()
```

---

## 9. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Wallpaper feature. The order is chosen so that each layer is
built on top of already-tested foundations.

### Phase 1: Foundation — Model Crate (`model/wallpaper`)

**Goal:** Define all shared messages, topics, and configuration types.

**Order:**

1. Create the crate `model/wallpaper` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_COMMAND` and `TOPIC_STATUS`.
3. Create one file per message struct:
    - `src/messages/wallpaper_type.rs` -> `WallpaperType` enum
    - `src/messages/video_config.rs` -> `VideoConfig` struct
    - `src/messages/image_config.rs` -> `ImageConfig` struct
    - `src/messages/app_config.rs` -> `AppConfig` struct
    - `src/messages/theme_config.rs` -> `WallpaperThemeConfig` enum
    - `src/messages/theme.rs` -> `WallpaperTheme` struct
    - `src/messages/command.rs` -> `WallpaperCommandAction` and `WallpaperCommandMessage`
    - `src/messages/monitor_process.rs` -> `MonitorProcess` struct
    - `src/messages/status.rs` -> `WallpaperStatusMessage` struct
    - `src/messages/icon.rs` -> `wallpaper_type_icon` mapping function
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.
- The `wallpaper_type_icon` function returns correct icon names for all `WallpaperType` variants.

---

### Phase 2: Backend — Service Crate (`services/wallpaper`)

**Goal:** Manage wallpaper process lifecycle, theme state, and broadcast status.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/wallpaper` with a `Cargo.toml` that depends on the `model/wallpaper` crate, the project plugin API, `tokio`, `nix`, `zbus`,
   `tracing`, and `toml`.
2. Create `src/config.rs` with `WallpaperServiceConfig` and its default values.
3. Create `src/process.rs` and implement:
    - `spawn_mpvpaper_video` for video slideshow spawning.
    - `spawn_mpvpaper_image` for image slideshow spawning.
    - `spawn_application` for application-based wallpaper spawning.
    - `resolve_outputs` for monitor name resolution (GNOME Mutter D-Bus, wlr-randr, hyprctl).
    - Per-monitor PID tracking via `MonitorProcess` entries in `WallpaperStatusMessage`.
    - `spawn_process` generic helper.
4. Create `src/service.rs` with `WallpaperService` and all required trait implementations.
5. Implement `select_theme`, `start_selected_wallpaper_theme`, and `stop_current_wallpaper_theme`.
6. Implement `run_update_loop` to process incoming commands and broadcast status.
7. Implement theme persistence (`add_theme_to_config`, `remove_theme_from_config`).
8. Register MCP resources (`wallpaper://status`, `wallpaper://themes`) and MCP tools (`add_wallpaper_theme`, `remove_wallpaper_theme`, `select_wallpaper_theme`,
   `stop_current_wallpaper_process`, `start_selected_wallpaper_process`).
9. Wire `service_plugin!` in `src/lib.rs`.
10. Add unit tests for process argument construction and output resolution.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for process argument construction produce correct `mpvpaper` command lines.
- Running the service broadcasts `TOPIC_STATUS` on command.
- `select_theme` updates `selected_theme` without spawning a process.
- `start_selected_wallpaper_theme` stops any running process and spawns the new one.
- `stop_current_wallpaper_theme` sends SIGTERM to all per-monitor PIDs and clears state.
- MCP resources return valid JSON when queried.
- MCP tools execute the correct actions and return results.
- Theme persistence writes and reads themes from the config file correctly.

---

### Phase 3: Display — Widget Crate (`plugins/wallpaper`)

**Goal:** Provide a GTK4 tile with preview image, swipe selection, and press/longpress actions.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/wallpaper` with a `Cargo.toml` that depends on `model/wallpaper`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with `WallpaperWidgetConfig` including all display flags.
3. Create `src/preview.rs` and implement preview image loading and fallback icon rendering.
4. Create `src/widget.rs` with `WallpaperWidget` and all required trait implementations.
5. Implement gesture handling using `GestureSwipe` for directional swipes and `GestureClick`/`GestureLongPress` for press/long-press:
    - Swipe up: select previous theme (`SelectTheme`).
    - Swipe down: select next theme (`SelectTheme`).
    - Press: start or restart selected theme (`StartSelected`).
    - Long press: stop current wallpaper (`StopCurrent`).
6. Subscribe to `TOPIC_STATUS` and update `current_status` + re-render preview on every message.
7. Wire `widget_plugin!` in `src/lib.rs`.
8. Add an integration test that verifies the widget accepts `TOPIC_STATUS` and renders the preview.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The widget displays the preview image for the selected theme.
- The widget shows the fallback icon when no preview image is available.
- The widget shows the theme name and type icon as a label overlay.
- The widget shows the running/stopped status indicator.
- Swipe up selects the previous theme (sends `SelectTheme` command).
- Swipe down selects the next theme (sends `SelectTheme` command).
- Press sends `StartSelected` command.
- Long press sends `StopCurrent` command.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/wallpaper` and `services/wallpaper` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `wallpaper` in `config.toml` with at least one theme of each type.
4. Add a sample widget configuration for the wallpaper widget.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The wallpaper widget receives messages and renders correctly.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that `TOPIC_STATUS` appears on the message broker.
2. Verify the widget displays the preview image for the default selected theme.
3. Verify swipe up/down cycles through themes without starting them (using `GestureSwipe` velocity-based detection).
4. Verify press starts the selected wallpaper process.
5. Verify long press stops the running wallpaper process.
6. Verify starting a new theme while one is running stops the old process first.
7. Verify MCP resources return valid JSON.
8. Verify `add_wallpaper_theme` MCP tool adds a theme to the config store.
9. Verify `remove_wallpaper_theme` MCP tool removes a theme from the config store.
10. Verify `select_wallpaper_theme` MCP tool updates the selected theme.
11. Verify `start_selected_wallpaper_process` MCP tool starts the selected theme.
12. Verify `stop_current_wallpaper_process` MCP tool stops the running process.
13. Run `cargo test` for all three crates.
14. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- The widget renders correctly for all wallpaper types.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.
- Process lifecycle (start, stop, restart) works reliably with per-monitor PID tracking.
- SIGTERM is sent first, with 100ms polling for early exit detection and SIGKILL fallback after the grace period.
- MCP tools return valid JSON and execute the correct actions.
- Theme persistence survives service restarts.

---

### Summary of Order

```
Phase 1: model/wallpaper
    |
    v
Phase 2: services/wallpaper
    |
    v
Phase 3: plugins/wallpaper
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

### Rationale

- **Model first:** Message formats and type definitions must exist before the service or widget can use them.
- **Service second:** The widget needs a running publisher to test against. Process lifecycle is the core logic.
- **Widget third:** The display widget depends on the service's status topic.
- **Wiring fourth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 10. Technical Notes

- **mpvpaper:** A Wayland wallpaper daemon that uses mpv to render video or image slideshows on the Layer Shell background layer. It supports all mpv options
  for playback control, transitions, and hardware decoding.
- **smearor-wrot:** A wrapper that renders an application on the Layer Shell background layer. This allows any GTK4 application to be used as a wallpaper.
- **Layer Shell background layer:** All wallpaper types render on the `background` layer of the Wayland Layer Shell protocol, which places them behind all other
  windows and panels.
- **Process management:** The service uses `tokio::process::Command` for spawning and `nix::sys::signal::kill` for termination. SIGTERM is sent first for
  graceful shutdown. The service polls every 100ms via `kill(pid, None)` to detect early exit, so theme switches proceed immediately once all processes
  have terminated — without waiting the full grace period. SIGKILL is sent only if the deadline elapses. Per-monitor PIDs are tracked in `current_processes`
  to support multi-monitor setups where different processes may run on different outputs.
- **Application `{monitor}` placeholder:** When an `Application` theme's `arguments` contain the literal `{monitor}`, the service spawns one process per
  target output, replacing the placeholder with each monitor name (e.g., `DP-1`, `HDMI-A-1`). This enables applications like `smearor-wrot` to render
  fullscreen on each monitor independently. If no placeholder is present, a single process is spawned and assigned to all outputs.
- **Gesture detection:** The widget uses `gtk4::GestureSwipe` instead of `gtk4::GestureDrag` for directional swipe detection. `GestureSwipe` provides
  `connect_swipe(velocity_x, velocity_y)` which reacts to actual swipe velocity and cleanly separates vertical from horizontal movement, avoiding false
  triggers from diagonal drags.
- **Output resolution:** The `outputs` field in `VideoConfig` and `ImageConfig` supports `["ALL"]` to target all connected monitors. The service queries
  connected monitors at spawn time via the GNOME Mutter D-Bus DisplayConfig API (`org.gnome.Mutter.DisplayConfig`), with fallback support for `wlr-randr`
  (wlroots) and `hyprctl` (Hyprland) compositors.
- **Theme persistence:** Themes added via the MCP `add_wallpaper_theme` tool are written to the configured config file (`wallpaper.toml`) so they persist across
  service restarts.
- **No polling in the widget:** The widget updates exclusively through incoming messages. Process management only happens in the service.
- **GTK widget ownership:** GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget
  references are captured in `glib::clone!` closures or `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.
- **MCP tool naming:** Tool names use `snake_case` with underscores, never dots. Dots cause schema validation failures in LLM gateways. This is consistent with
  existing tools (`sysinfo_refresh`, `get_current_time`, `weather_refresh`).
- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in `model/power`, `model/audio`, and `model/app-launcher`.
- **FFI integer types for floats:** To maximize FFI safety, all floating-point values are represented as integers in `#[stabby::stabby]` structs. Playback
  speed uses `speed_percentage: u32` (100 = 1.0x), display and transition durations use milliseconds (`u32`) instead of seconds (`f32`). This avoids
  potential ABI instability with floating-point representations across compiler invocations.

---

## 11. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/wallpaper`, `services/wallpaper`, and `plugins/wallpaper`.
- **One struct per file:** Each message struct and each enum lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String` and optional strings use
  `stabby::option::Option<stabby::string::String>` to maintain ABI stability across compiler invocations. Floating-point values are represented as integers
  (`speed_percentage`, `display_duration_ms`, `transition_duration_ms`) to avoid FFI representation issues.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `tokio`, `nix`, `zbus`, and `tracing`; the widget uses `gtk4` and `glib`.

---

*End of document.*
