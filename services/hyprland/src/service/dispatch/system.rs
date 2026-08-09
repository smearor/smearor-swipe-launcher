use crate::service::converters::lock_type::convert_lock_type;
use crate::service::converters::window_identifier::OwnedWindowIdentifier;
use crate::service::ensure_hyprland_instance_signature;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use smearor_hyprland_model::CustomDispatchMessage;
use smearor_hyprland_model::GlobalDispatchMessage;
use smearor_hyprland_model::HyprlandSystemDispatchMessage;
use smearor_hyprland_model::LockGroupsDispatchMessage;
use smearor_hyprland_model::PassDispatchMessage;
use smearor_hyprland_model::SetCursorDispatchMessage;
use smearor_hyprland_model::SystemDispatchKind;
use smearor_hyprland_model::SystemDispatchOps;
use tracing::error;

pub(crate) async fn handle_dispatch_system(message: HyprlandSystemDispatchMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = handle_system_dispatch(message.kind, message.ops).await {
        error!("Hyprland system dispatch failed: {error}");
    }
}

async fn handle_system_dispatch(kind: SystemDispatchKind, ops: SystemDispatchOps) -> hyprland::Result<()> {
    match kind {
        SystemDispatchKind::AddMaster => Dispatch::call_async(DispatchType::AddMaster).await,
        SystemDispatchKind::BringActiveToTop => Dispatch::call_async(DispatchType::BringActiveToTop).await,
        SystemDispatchKind::Custom => match ops.custom.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_custom(payload).await,
            None => Ok(()),
        },
        SystemDispatchKind::Exit => Dispatch::call_async(DispatchType::Exit).await,
        SystemDispatchKind::ForceRendererReload => Dispatch::call_async(DispatchType::ForceRendererReload).await,
        SystemDispatchKind::Global => match ops.global.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_global(payload).await,
            None => Ok(()),
        },
        SystemDispatchKind::LockGroups => match ops.lock_groups.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_lock_groups(payload).await,
            None => Ok(()),
        },
        SystemDispatchKind::MoveOutOfGroup => Dispatch::call_async(DispatchType::MoveOutOfGroup).await,
        SystemDispatchKind::OrientationBottom => Dispatch::call_async(DispatchType::OrientationBottom).await,
        SystemDispatchKind::OrientationCenter => Dispatch::call_async(DispatchType::OrientationCenter).await,
        SystemDispatchKind::OrientationLeft => Dispatch::call_async(DispatchType::OrientationLeft).await,
        SystemDispatchKind::OrientationNext => Dispatch::call_async(DispatchType::OrientationNext).await,
        SystemDispatchKind::OrientationPrev => Dispatch::call_async(DispatchType::OrientationPrev).await,
        SystemDispatchKind::OrientationRight => Dispatch::call_async(DispatchType::OrientationRight).await,
        SystemDispatchKind::OrientationTop => Dispatch::call_async(DispatchType::OrientationTop).await,
        SystemDispatchKind::Pass => match ops.pass.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_pass(payload).await,
            None => Ok(()),
        },
        SystemDispatchKind::RemoveMaster => Dispatch::call_async(DispatchType::RemoveMaster).await,
        SystemDispatchKind::SetCursor => match ops.set_cursor.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_set_cursor(payload).await,
            None => Ok(()),
        },
    }
}

async fn handle_custom(payload: CustomDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::Custom(&payload.name, &payload.value)).await
}

async fn handle_global(payload: GlobalDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::Global(&payload.key)).await
}

async fn handle_lock_groups(payload: LockGroupsDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::LockGroups(convert_lock_type(payload.lock_type))).await
}

async fn handle_pass(payload: PassDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::Pass(OwnedWindowIdentifier::from(&payload.window_identifier).as_ref())).await
}

async fn handle_set_cursor(payload: SetCursorDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::SetCursor(&payload.theme, payload.size)).await
}
