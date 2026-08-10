use crate::service::ensure_hyprland_instance_signature;
use crate::service::shared_state::HyprlandSharedState;
use hyprland::data::FullscreenMode;
use hyprland::shared::HyprData;
use smearor_hyprland_model::WindowEntry;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::error;

/// Handle a `WindowsRequest` by querying all windows from Hyprland
/// and storing them in `shared_state.last_windows`.
pub(crate) async fn handle_windows_request(shared_state: &Arc<Mutex<HyprlandSharedState>>) {
    ensure_hyprland_instance_signature();

    let windows = tokio::task::spawn_blocking(|| match hyprland::data::Clients::get() {
        Ok(clients) => {
            let entries: Vec<WindowEntry> = clients
                .into_iter()
                .map(|c| WindowEntry {
                    class: c.class.clone(),
                    title: c.title.clone(),
                    address: c.address.to_string(),
                    workspace_id: c.workspace.id,
                    monitor: c.monitor,
                    floating: c.floating,
                    fullscreen_mode: fullscreen_mode_to_string(c.fullscreen),
                    pinned: c.pinned,
                    mapped: c.mapped,
                    pid: c.pid,
                    is_active: c.focus_history_id == 0,
                })
                .collect();
            entries
        }
        Err(error) => {
            error!("Hyprland service: failed to query windows: {error}");
            Vec::new()
        }
    })
    .await
    .unwrap_or_default();

    if let Ok(mut guard) = shared_state.lock() {
        guard.last_windows = Some(windows);
    }
}

/// Convert a `FullscreenMode` to a lowercase string representation.
fn fullscreen_mode_to_string(mode: FullscreenMode) -> String {
    match mode {
        FullscreenMode::None => "none".to_string(),
        FullscreenMode::Maximized => "maximized".to_string(),
        FullscreenMode::Fullscreen => "fullscreen".to_string(),
        FullscreenMode::MaximizedFullscreen => "maximized_fullscreen".to_string(),
    }
}
