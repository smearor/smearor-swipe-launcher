use std::process::Command;
use tracing::debug;
use tracing::warn;

/// Detect the active workspace index and total workspace count via `wmctrl -d`.
///
/// `wmctrl -d` output format:
/// ```text
/// 0  * DG: 1920x1080  VP: 0,0  WA: 0,0 1920x1080  Workspace 1
/// 1  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  Workspace 2
/// ```
///
/// The line containing `*` indicates the current desktop. The first field
/// is the desktop index. Returns `(active_index, max_index)` where
/// `max_index` is the highest desktop index seen.
pub fn detect_active_workspace() -> Option<(i32, i32)> {
    let output = Command::new("wmctrl").arg("-d").output().ok()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        debug!("wmctrl -d failed with status {} (stderr: {})", output.status, stderr);
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_wmctrl_desktops(&stdout)
}

/// Switch to a workspace by index via `wmctrl -s <index>`.
///
/// Returns `true` if the command executed successfully.
pub fn switch_workspace(workspace_id: i32) -> bool {
    debug!("wmctrl: switching to workspace {}", workspace_id);
    match Command::new("wmctrl").arg("-s").arg(workspace_id.to_string()).output() {
        Ok(output) => {
            if output.status.success() {
                true
            } else {
                warn!("wmctrl -s {} failed: {}", workspace_id, String::from_utf8_lossy(&output.stderr).trim());
                false
            }
        }
        Err(error) => {
            warn!("wmctrl -s {} failed to execute: {}", workspace_id, error);
            false
        }
    }
}

/// Check whether `wmctrl` is available and functional on the system.
///
/// Tests `wmctrl -d` (the actual command used for workspace detection) and
/// requires it to exit successfully. On Wayland, `wmctrl` may be installed
/// but unable to communicate with the compositor, so a mere exit-code-1
/// tolerance is insufficient.
pub fn is_available() -> bool {
    Command::new("wmctrl").arg("-d").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Parse `wmctrl -d` output to find the active workspace and max workspace index.
///
/// Returns `Some((active_index, max_index))` if the active workspace could be
/// determined, or `None` if parsing failed or no active workspace was found.
fn parse_wmctrl_desktops(output: &str) -> Option<(i32, i32)> {
    let mut active_workspace: Option<i32> = None;
    let mut max_workspace: i32 = 0;

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let index: i32 = parts[0].parse().ok()?;

        if index > max_workspace {
            max_workspace = index;
        }

        // The second field is "*" for the current desktop, "-" for others.
        if parts.len() >= 2 && parts[1] == "*" {
            active_workspace = Some(index);
        }
    }

    if let Some(active) = active_workspace {
        Some((active, max_workspace))
    } else {
        debug!("wmctrl -d: no active workspace marker found");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::parse_wmctrl_desktops;

    #[test]
    fn test_parse_active_first() {
        let output = "0  * DG: 1920x1080  VP: 0,0  WA: 0,0 1920x1080  Workspace 1\n1  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  Workspace 2\n";
        let result = parse_wmctrl_desktops(output);
        assert_eq!(result, Some((0, 1)));
    }

    #[test]
    fn test_parse_active_second() {
        let output = "0  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  Workspace 1\n1  * DG: 1920x1080  VP: 0,0  WA: 0,0 1920x1080  Workspace 2\n";
        let result = parse_wmctrl_desktops(output);
        assert_eq!(result, Some((1, 1)));
    }

    #[test]
    fn test_parse_no_active() {
        let output = "0  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  Workspace 1\n1  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  Workspace 2\n";
        let result = parse_wmctrl_desktops(output);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_four_workspaces() {
        let output = "0  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  1\n1  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  2\n2  * DG: 1920x1080  VP: 0,0  WA: 0,0 1920x1080  3\n3  - DG: 1920x1080  VP: N/A  WA: 0,0 1920x1080  4\n";
        let result = parse_wmctrl_desktops(output);
        assert_eq!(result, Some((2, 3)));
    }
}
