use crate::service::converters::fullscreen_type::convert_fullscreen_type;
use crate::service::ensure_hyprland_instance_signature;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::ToggleDispatchKind;
use smearor_hyprland_model::ToggleDispatchOps;
use smearor_hyprland_model::ToggleDpmsDispatchMessage;
use smearor_hyprland_model::ToggleFullscreenDispatchMessage;
use tracing::error;

pub(crate) async fn handle_dispatch_toggle(message: HyprlandToggleDispatchMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = handle_toggle_dispatch(message.kind, message.ops).await {
        error!("Hyprland toggle dispatch failed: {error}");
    }
}

async fn handle_toggle_dispatch(kind: ToggleDispatchKind, ops: ToggleDispatchOps) -> hyprland::Result<()> {
    match kind {
        ToggleDispatchKind::ToggleDpms => match ops.toggle_dpms.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_toggle_dpms(payload).await,
            None => Ok(()),
        },
        ToggleDispatchKind::ToggleFakeFullscreen => Dispatch::call_async(DispatchType::ToggleFakeFullscreen).await,
        ToggleDispatchKind::ToggleFloating => handle_toggle_floating().await,
        ToggleDispatchKind::ToggleFullscreen => match ops.toggle_fullscreen.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_toggle_fullscreen(payload).await,
            None => Ok(()),
        },
        ToggleDispatchKind::ToggleGroup => Dispatch::call_async(DispatchType::ToggleGroup).await,
        ToggleDispatchKind::ToggleOpaque => Dispatch::call_async(DispatchType::ToggleOpaque).await,
        ToggleDispatchKind::TogglePin => Dispatch::call_async(DispatchType::TogglePin).await,
        ToggleDispatchKind::TogglePseudo => Dispatch::call_async(DispatchType::TogglePseudo).await,
        ToggleDispatchKind::ToggleSplit => Dispatch::call_async(DispatchType::ToggleSplit).await,
    }
}

async fn handle_toggle_floating() -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ToggleFloating(None)).await
}

async fn handle_toggle_fullscreen(payload: ToggleFullscreenDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ToggleFullscreen(convert_fullscreen_type(payload.fullscreen_type))).await
}

async fn handle_toggle_dpms(payload: ToggleDpmsDispatchMessage) -> hyprland::Result<()> {
    let name_ref = payload.name.as_ref().map(|n| n.as_str());
    Dispatch::call_async(DispatchType::ToggleDPMS(payload.on, name_ref)).await
}
