use crate::service::ensure_hyprland_instance_signature;
use crate::service::shared_state::HyprlandSharedState;
use hyprland::data::Devices;
use hyprland::data::Workspace;
use hyprland::shared::HyprData;
use hyprland::shared::HyprDataActive;
use smearor_hyprland_model::HyprlandStateMessage;
use smearor_hyprland_model::HyprlandWindowEventData;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use std::sync::Arc;
use std::sync::Mutex;

/// Handle a `HyprlandStateRequestMessage` by querying the current Hyprland state
/// and broadcasting a `HyprlandStateMessage`.
///
/// Synchronous IPC calls are wrapped in `tokio::task::spawn_blocking` to prevent
/// blocking the async event worker when the compositor is under load.
pub(crate) async fn handle_state_request(broadcaster: &MessageBroadcasterInner, shared_state: &Arc<Mutex<HyprlandSharedState>>) {
    ensure_hyprland_instance_signature();

    let state = tokio::task::spawn_blocking(|| {
        let active_window = hyprland::data::Clients::get()
            .ok()
            .and_then(|clients| clients.into_iter().find(|c| c.focus_history_id == 0))
            .map(|c| HyprlandWindowEventData {
                window_class: c.class.clone().into(),
                window_title: c.title.clone().into(),
                window_address: c.address.to_string().into(),
                workspace_id: stabby::option::Option::Some(c.workspace.id),
            });

        let is_fullscreen = active_window
            .as_ref()
            .is_some_and(|_w| Workspace::get_active().ok().map(|ws| ws.fullscreen).unwrap_or(false));

        let keyboard_layout = Devices::get()
            .ok()
            .and_then(|devices| devices.keyboards.first().map(|k| k.active_keymap.clone().into()));

        let sub_map = String::new().into();

        let ignore_group_lock = false;
        let groups_locked = false;

        HyprlandStateMessage {
            active_window: active_window.into(),
            is_fullscreen,
            keyboard_layout: keyboard_layout.into(),
            sub_map,
            ignore_group_lock,
            groups_locked,
        }
    })
    .await
    .unwrap_or_default();

    if let Ok(mut guard) = shared_state.lock() {
        guard.last_state = Some(state.clone());
    }

    broadcaster.broadcast_message_to_topic(state);
}
