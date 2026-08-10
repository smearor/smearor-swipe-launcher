use crate::service::ensure_hyprland_instance_signature;
use crate::service::shared_state::HyprlandSharedState;
use hyprland::shared::HyprData;
use smearor_hyprland_model::MonitorEntry;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::error;

/// Handle a `MonitorsRequest` by querying all monitors from Hyprland
/// and storing them in `shared_state.last_monitors`.
pub(crate) async fn handle_monitors_request(shared_state: &Arc<Mutex<HyprlandSharedState>>) {
    ensure_hyprland_instance_signature();

    let monitors = tokio::task::spawn_blocking(|| match hyprland::data::Monitors::get() {
        Ok(monitors) => {
            let entries: Vec<MonitorEntry> = monitors
                .into_iter()
                .map(|m| MonitorEntry {
                    monitor_index: m.id as u32,
                    connector_name: m.name.clone(),
                    width: m.width as u32,
                    height: m.height as u32,
                    refresh_rate: m.refresh_rate,
                    x: m.x,
                    y: m.y,
                    active_workspace_id: m.active_workspace.id,
                    active_workspace_name: m.active_workspace.name.clone(),
                    scale: m.scale,
                    transform: transform_to_string(m.transform),
                    focused: m.focused,
                    dpms_status: m.dpms_status,
                    vrr: m.vrr,
                    disabled: m.disabled,
                })
                .collect();
            entries
        }
        Err(error) => {
            error!("Hyprland service: failed to query monitors: {error}");
            Vec::new()
        }
    })
    .await
    .unwrap_or_default();

    if let Ok(mut guard) = shared_state.lock() {
        guard.last_monitors = Some(monitors);
    }
}

/// Convert a `hyprland::data::Transforms` to a string representation.
fn transform_to_string(transform: hyprland::data::Transforms) -> String {
    match transform {
        hyprland::data::Transforms::Normal => "normal".to_string(),
        hyprland::data::Transforms::Normal90 => "90".to_string(),
        hyprland::data::Transforms::Normal180 => "180".to_string(),
        hyprland::data::Transforms::Normal270 => "270".to_string(),
        hyprland::data::Transforms::Flipped => "flipped".to_string(),
        hyprland::data::Transforms::Flipped90 => "flipped_90".to_string(),
        hyprland::data::Transforms::Flipped180 => "flipped_180".to_string(),
        hyprland::data::Transforms::Flipped270 => "flipped_270".to_string(),
    }
}
