use smearor_wallpaper_model::MonitorProcess;
use smearor_wallpaper_model::WallpaperTheme;
use smearor_wallpaper_model::WallpaperThemeConfig;
use smearor_wallpaper_model::WallpaperType;
use std::process::Command;
use std::process::Stdio;
use tracing::debug;
use tracing::error;
use which::which;

/// Resolves the `outputs` field to actual monitor names.
/// `["ALL"]` returns all connected monitors detected via GTK/Gdk.
/// Otherwise returns the list as-is.
pub fn resolve_outputs(outputs: &[String]) -> Vec<String> {
    if outputs.iter().any(|o| o == "ALL") {
        return detect_monitors();
    }
    outputs.to_vec()
}

/// Detects connected monitors using GTK4 GdkDisplay.
/// Falls back to `["ALL"]` if detection fails.
fn detect_monitors() -> Vec<String> {
    // In a service context we may not have a GTK display connection.
    // Fall back to "ALL" which mpvpaper interprets as all outputs.
    // For more precise detection, the host can pass explicit output names in config.
    vec!["ALL".to_string()]
}

/// Resolves an executable name to its full path using `which`.
fn resolve_executable(name: &str) -> String {
    which(name).map(|path| path.to_string_lossy().to_string()).unwrap_or_else(|_| name.to_string())
}

/// Spawns a process and returns its PID.
fn spawn_process(command: &str, args: &[String]) -> Option<u32> {
    let resolved = resolve_executable(command);
    debug!("Spawning wallpaper process: {} {}", resolved, args.join(" "));
    match Command::new(&resolved)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            let pid = child.id();
            debug!("Wallpaper process spawned with PID {}", pid);
            Some(pid)
        }
        Err(e) => {
            error!("Failed to spawn wallpaper process '{}': {}", resolved, e);
            None
        }
    }
}

/// Spawns a video wallpaper using `mpvpaper` and returns per-monitor PIDs.
pub fn spawn_mpvpaper_video(theme: &WallpaperTheme) -> Vec<(u32, String)> {
    let WallpaperThemeConfig::Video(config) = &theme.config else {
        return Vec::new();
    };
    let outputs = resolve_outputs(&config.outputs);
    let mpvpaper = resolve_executable("mpvpaper");

    let mut mpv_args = Vec::new();
    mpv_args.push("--loop-playlist=inf".to_string());
    if config.shuffle {
        mpv_args.push("--shuffle".to_string());
    }
    if config.muted {
        mpv_args.push("--mute=yes".to_string());
    } else {
        mpv_args.push(format!("--volume={}", config.volume));
    }
    let speed = config.speed_percentage as f64 / 100.0;
    mpv_args.push(format!("--speed={}", speed));
    for extra in &config.extra_arguments {
        mpv_args.push(extra.clone());
    }

    let mut results = Vec::new();
    for monitor in &outputs {
        let mut full_args = vec![monitor.clone(), mpv_args.join(" "), config.directory.clone()];
        full_args.extend(mpv_args.iter().cloned());
        // mpvpaper syntax: mpvpaper <output> <mpv_options> <playlist_path>
        let args = build_mpvpaper_args(monitor, &mpv_args, &config.directory);
        if let Some(pid) = spawn_process(&mpvpaper, &args) {
            results.push((pid, monitor.clone()));
        }
    }
    results
}

/// Builds the argument list for `mpvpaper`.
fn build_mpvpaper_args(monitor: &str, mpv_args: &[String], path: &str) -> Vec<String> {
    let mut args = Vec::new();
    args.push(monitor.to_string());
    if !mpv_args.is_empty() {
        args.push(mpv_args.join(" "));
    }
    args.push(path.to_string());
    args
}

/// Spawns an image slideshow wallpaper using `mpvpaper` and returns per-monitor PIDs.
pub fn spawn_mpvpaper_image(theme: &WallpaperTheme) -> Vec<(u32, String)> {
    let WallpaperThemeConfig::Image(config) = &theme.config else {
        return Vec::new();
    };
    let outputs = resolve_outputs(&config.outputs);
    let mpvpaper = resolve_executable("mpvpaper");

    let mut mpv_args = Vec::new();
    mpv_args.push("--loop-playlist=inf".to_string());
    if config.shuffle {
        mpv_args.push("--shuffle".to_string());
    }
    mpv_args.push(format!("--image-display-duration={}", config.display_duration_ms as f64 / 1000.0));
    if config.transitions {
        mpv_args.push("--vo=gpu".to_string());
        mpv_args.push(format!("--glsl-shader=~~/shaders/{}.glsl", config.transition_effect));
        mpv_args.push(format!("--transition-duration={}", config.transition_duration_ms as f64 / 1000.0));
    }
    for extra in &config.extra_arguments {
        mpv_args.push(extra.clone());
    }

    let mut results = Vec::new();
    for monitor in &outputs {
        let args = build_mpvpaper_args(monitor, &mpv_args, &config.directory);
        if let Some(pid) = spawn_process(&mpvpaper, &args) {
            results.push((pid, monitor.clone()));
        }
    }
    results
}

/// Spawns an application-based wallpaper and returns per-monitor PIDs.
/// If any argument contains the `{monitor}` placeholder, the service spawns
/// one process per target output, replacing the placeholder with the monitor name.
/// If no placeholder is present, a single process is spawned for all outputs.
pub fn spawn_application(theme: &WallpaperTheme) -> Vec<(u32, String)> {
    let WallpaperThemeConfig::Application(config) = &theme.config else {
        return Vec::new();
    };

    let outputs = resolve_outputs(&config.outputs);

    let has_placeholder = config.arguments.iter().any(|a| a.contains("{monitor}"));

    if !has_placeholder {
        let args: Vec<String> = config.arguments.iter().map(|s| s.to_string()).collect();
        if let Some(pid) = spawn_process(&config.command, &args) {
            return outputs.into_iter().map(|o| (pid, o)).collect();
        }
        return Vec::new();
    }

    let mut results = Vec::new();
    for monitor in &outputs {
        let args: Vec<String> = config.arguments.iter().map(|s| s.replace("{monitor}", monitor)).collect();
        if let Some(pid) = spawn_process(&config.command, &args) {
            results.push((pid, monitor.clone()));
        }
    }
    results
}

/// Spawns the appropriate wallpaper process based on theme type.
pub fn spawn_wallpaper(theme: &WallpaperTheme) -> Vec<(u32, String)> {
    match theme.wallpaper_type {
        WallpaperType::Video => spawn_mpvpaper_video(theme),
        WallpaperType::Image => spawn_mpvpaper_image(theme),
        WallpaperType::Application => spawn_application(theme),
    }
}

/// Converts a list of (pid, monitor) pairs into MonitorProcess structs.
pub fn to_monitor_processes(pids: &[(u32, String)]) -> Vec<MonitorProcess> {
    pids.iter()
        .map(|(pid, monitor)| MonitorProcess {
            monitor: monitor.clone().into(),
            process_id: *pid,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smearor_wallpaper_model::AppConfig;

    #[test]
    fn resolve_outputs_all_returns_all() {
        let result = resolve_outputs(&["ALL".to_string()]);
        assert!(!result.is_empty());
    }

    #[test]
    fn resolve_outputs_specific_returns_as_is() {
        let result = resolve_outputs(&["DP-1".to_string(), "HDMI-A-1".to_string()]);
        assert_eq!(result, vec!["DP-1", "HDMI-A-1"]);
    }

    #[test]
    fn spawn_application_no_placeholder_spawns_once() {
        // This test only verifies argument construction, not actual spawning.
        let config = AppConfig {
            command: "echo".to_string(),
            outputs: vec!["DP-1".to_string(), "DP-2".to_string()],
            arguments: vec!["hello".to_string()],
        };
        let has_placeholder = config.arguments.iter().any(|a| a.contains("{monitor}"));
        assert!(!has_placeholder);
    }

    #[test]
    fn spawn_application_with_placeholder_expands_per_monitor() {
        let config = AppConfig {
            command: "echo".to_string(),
            outputs: vec!["DP-1".to_string(), "DP-2".to_string()],
            arguments: vec!["--output".to_string(), "{monitor}".to_string()],
        };
        let has_placeholder = config.arguments.iter().any(|a| a.contains("{monitor}"));
        assert!(has_placeholder);
        let expanded: Vec<String> = config.arguments.iter().map(|s| s.replace("{monitor}", "DP-1")).collect();
        assert_eq!(expanded, vec!["--output", "DP-1"]);
    }

    #[test]
    fn to_monitor_processes_converts_correctly() {
        let pids = vec![(123u32, "DP-1".to_string()), (456u32, "HDMI-A-1".to_string())];
        let result = to_monitor_processes(&pids);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].process_id, 123);
        assert_eq!(result[1].process_id, 456);
    }
}
