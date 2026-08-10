use crate::service::ensure_hyprland_instance_signature;
use crate::service::shared_state::HyprlandSharedState;
use hyprland::shared::HyprData;
use smearor_hyprland_model::VersionResponse;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::error;

/// Handle a `VersionRequest` by querying Hyprland version info
/// and storing it in `shared_state.last_version`.
pub(crate) async fn handle_version_request(shared_state: &Arc<Mutex<HyprlandSharedState>>) {
    ensure_hyprland_instance_signature();

    let version = tokio::task::spawn_blocking(|| match hyprland::data::Version::get() {
        Ok(v) => Some(VersionResponse {
            tag: v.tag.clone(),
            branch: v.branch.clone(),
            commit: v.commit.clone(),
            dirty: v.dirty,
            commit_message: v.commit_message.clone(),
            commit_date: v.commit_date.clone(),
            commits: v.commits.clone(),
            build_aquamarine: v.build_aquamarine.clone(),
            flags: v.flags.clone(),
        }),
        Err(error) => {
            error!("Hyprland service: failed to query version: {error}");
            None
        }
    })
    .await
    .unwrap_or(None);

    if let Some(version) = version {
        if let Ok(mut guard) = shared_state.lock() {
            guard.last_version = Some(version);
        }
    }
}
