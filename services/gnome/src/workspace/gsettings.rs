use gio::Settings;
use gio::prelude::SettingsExt;
use gio::prelude::SettingsExtManual;
use tracing::debug;
use tracing::warn;

/// Read workspace names from GSettings (`org.gnome.desktop.wm.preferences`).
///
/// Returns the configured workspace names. If no names are configured,
/// returns empty strings for each workspace index.
pub fn read_workspace_names(count: i32) -> Vec<String> {
    let settings = Settings::new("org.gnome.desktop.wm.preferences");
    let configured_names: Vec<String> = settings.strv("workspace-names").into_iter().map(|s| s.to_string()).collect();

    let mut names = Vec::with_capacity(count as usize);
    for i in 0..count {
        let name = configured_names
            .get(i as usize)
            .cloned()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| i.to_string());
        names.push(name);
    }
    names
}

/// Read the configured number of workspaces from GSettings.
///
/// Reads `org.gnome.desktop.wm.preferences::num-workspaces`.
/// Only meaningful when `dynamic-workspaces` is false.
pub fn read_workspace_count() -> i32 {
    let settings = Settings::new("org.gnome.desktop.wm.preferences");
    settings.int("num-workspaces")
}

/// Check if GNOME is configured to use dynamic workspaces.
///
/// Reads `org.gnome.mutter::dynamic-workspaces`.
pub fn is_dynamic_workspaces() -> bool {
    let settings = Settings::new("org.gnome.mutter");
    settings.boolean("dynamic-workspaces")
}

/// Build a complete workspace list using GSettings and Introspect data.
///
/// - If dynamic workspaces are enabled, uses the max workspace ID from
///   Introspect windows as the count (since empty workspaces may not appear).
/// - If dynamic workspaces are disabled, uses `num-workspaces` from GSettings.
/// - Workspace names come from `workspace-names` in GSettings, falling back
///   to the index as a string.
pub fn build_workspace_list(introspect_count: i32, active_workspace_id: i32) -> Vec<(i32, String)> {
    let dynamic = is_dynamic_workspaces();
    debug!(
        "GSettings: dynamic_workspaces={}, introspect_count={}, active={}",
        dynamic, introspect_count, active_workspace_id
    );

    let count = if dynamic {
        // With dynamic workspaces, use the max workspace ID from windows.
        // Add 1 to account for the active workspace even if it has no windows
        // (unlikely, but safe). Also ensure at least active+1 workspaces.
        introspect_count.max(active_workspace_id + 1).max(1)
    } else {
        let configured = read_workspace_count();
        if configured == 0 {
            warn!("GSettings: num-workspaces is 0, falling back to introspect count");
            introspect_count.max(1)
        } else {
            configured
        }
    };

    let names = read_workspace_names(count);
    names.into_iter().enumerate().map(|(i, name)| (i as i32, name)).collect()
}
