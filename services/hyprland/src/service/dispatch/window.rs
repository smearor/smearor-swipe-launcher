use crate::service::converters::corner::convert_corner;
use crate::service::converters::cycle_direction::convert_cycle_direction;
use crate::service::converters::direction::convert_direction;
use crate::service::converters::focus_master_param::convert_focus_master_param;
use crate::service::converters::monitor_identifier::convert_monitor_identifier;
use crate::service::converters::position::convert_position;
use crate::service::converters::swap_with_master_param::convert_swap_with_master_param;
use crate::service::converters::window_identifier::OwnedWindowIdentifier;
use crate::service::converters::window_move::OwnedWindowMove;
use crate::service::converters::window_switch_direction::convert_window_switch_direction;
use crate::service::ensure_hyprland_instance_signature;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use hyprland::dispatch::FloatValue;
use smearor_hyprland_model::ChangeGroupActiveDispatchMessage;
use smearor_hyprland_model::ChangeSplitRatioDispatchMessage;
use smearor_hyprland_model::CloseWindowDispatchMessage;
use smearor_hyprland_model::CycleWindowDispatchMessage;
use smearor_hyprland_model::ExecDispatchMessage;
use smearor_hyprland_model::FocusMasterDispatchMessage;
use smearor_hyprland_model::FocusMonitorDispatchMessage;
use smearor_hyprland_model::FocusWindowDispatchMessage;
use smearor_hyprland_model::HyprlandWindowDispatchMessage;
use smearor_hyprland_model::MoveActiveDispatchMessage;
use smearor_hyprland_model::MoveCursorDispatchMessage;
use smearor_hyprland_model::MoveCursorToCornerDispatchMessage;
use smearor_hyprland_model::MoveFocusDispatchMessage;
use smearor_hyprland_model::MoveIntoGroupDispatchMessage;
use smearor_hyprland_model::MoveWindowDispatchMessage;
use smearor_hyprland_model::MoveWindowPixelDispatchMessage;
use smearor_hyprland_model::ResizeActiveDispatchMessage;
use smearor_hyprland_model::ResizeWindowPixelDispatchMessage;
use smearor_hyprland_model::SwapWindowDispatchMessage;
use smearor_hyprland_model::SwapWithMasterDispatchMessage;
use smearor_hyprland_model::WindowDispatchKind;
use smearor_hyprland_model::WindowDispatchOps;
use tracing::error;

pub(crate) async fn handle_dispatch_window(message: HyprlandWindowDispatchMessage) {
    ensure_hyprland_instance_signature();
    let result = handle_window_dispatch(message.kind, message.ops).await;
    if let Err(error) = result {
        error!("Hyprland window dispatch failed: {error}");
    }
}

async fn handle_window_dispatch(kind: WindowDispatchKind, ops: WindowDispatchOps) -> hyprland::Result<()> {
    match kind {
        WindowDispatchKind::CenterWindow => Dispatch::call_async(DispatchType::CenterWindow).await,
        WindowDispatchKind::ChangeGroupActive => match ops.change_group_active.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_change_group_active(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::ChangeSplitRatio => match ops.change_split_ratio.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_change_split_ratio(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::CloseWindow => match ops.close_window.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_close_window(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::CycleWindow => match ops.cycle_window.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_cycle_window(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::Exec => match ops.exec.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_exec(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::FocusCurrentOrLast => Dispatch::call_async(DispatchType::FocusCurrentOrLast).await,
        WindowDispatchKind::FocusMaster => match ops.focus_master.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_focus_master(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::FocusMonitor => match ops.focus_monitor.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_focus_monitor(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::FocusUrgentOrLast => Dispatch::call_async(DispatchType::FocusUrgentOrLast).await,
        WindowDispatchKind::FocusWindow => match ops.focus_window.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_focus_window(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::KillActiveWindow => Dispatch::call_async(DispatchType::KillActiveWindow).await,
        WindowDispatchKind::MoveActive => match ops.move_active.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_active(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::MoveCursor => match ops.move_cursor.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_cursor(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::MoveCursorToCorner => match ops.move_cursor_to_corner.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_cursor_to_corner(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::MoveFocus => match ops.move_focus.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_focus(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::MoveIntoGroup => match ops.move_into_group.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_into_group(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::MoveWindow => match ops.move_window.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_window(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::MoveWindowPixel => match ops.move_window_pixel.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_window_pixel(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::ResizeActive => match ops.resize_active.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_resize_active(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::ResizeWindowPixel => match ops.resize_window_pixel.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_resize_window_pixel(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::SwapWindow => match ops.swap_window.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_swap_window(payload).await,
            None => Ok(()),
        },
        WindowDispatchKind::SwapWithMaster => match ops.swap_with_master.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_swap_with_master(payload).await,
            None => Ok(()),
        },
    }
}

async fn handle_exec(payload: ExecDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::Exec(&payload.command)).await
}

async fn handle_move_focus(payload: MoveFocusDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveFocus(convert_direction(payload.direction))).await
}

async fn handle_change_group_active(payload: ChangeGroupActiveDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ChangeGroupActive(convert_window_switch_direction(payload.direction))).await
}

async fn handle_change_split_ratio(payload: ChangeSplitRatioDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ChangeSplitRatio(FloatValue::Exact(payload.ratio))).await
}

async fn handle_close_window(payload: CloseWindowDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::CloseWindow(OwnedWindowIdentifier::from(&payload.window_identifier).as_ref())).await
}

async fn handle_cycle_window(payload: CycleWindowDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::CycleWindow(convert_cycle_direction(payload.cycle_direction))).await
}

async fn handle_focus_master(payload: FocusMasterDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::FocusMaster(convert_focus_master_param(payload.param))).await
}

async fn handle_focus_monitor(payload: FocusMonitorDispatchMessage) -> hyprland::Result<()> {
    let name_opt: Option<stabby::string::String> = payload.monitor_identifier.name.clone().into();
    let name_string = name_opt.map(|n| n.to_string());
    let name_ref = name_string.as_ref().map(|n| n.as_str());
    let monitor = convert_monitor_identifier(&payload.monitor_identifier, name_ref);
    Dispatch::call_async(DispatchType::FocusMonitor(monitor)).await
}

async fn handle_focus_window(payload: FocusWindowDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::FocusWindow(OwnedWindowIdentifier::from(&payload.window_identifier).as_ref())).await
}

async fn handle_move_active(payload: MoveActiveDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveActive(convert_position(payload.position))).await
}

async fn handle_move_cursor(payload: MoveCursorDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveCursor(payload.x, payload.y)).await
}

async fn handle_move_cursor_to_corner(payload: MoveCursorToCornerDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveCursorToCorner(convert_corner(payload.corner))).await
}

async fn handle_move_into_group(payload: MoveIntoGroupDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveIntoGroup(convert_direction(payload.direction))).await
}

async fn handle_move_window(payload: MoveWindowDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveWindow(OwnedWindowMove::from(&payload.window_move).as_ref())).await
}

async fn handle_move_window_pixel(payload: MoveWindowPixelDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveWindowPixel(
        convert_position(payload.position),
        OwnedWindowIdentifier::from(&payload.window_identifier).as_ref(),
    ))
    .await
}

async fn handle_resize_active(payload: ResizeActiveDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ResizeActive(convert_position(payload.position))).await
}

async fn handle_resize_window_pixel(payload: ResizeWindowPixelDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ResizeWindowPixel(
        convert_position(payload.position),
        OwnedWindowIdentifier::from(&payload.window_identifier).as_ref(),
    ))
    .await
}

async fn handle_swap_window(payload: SwapWindowDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::SwapNext(convert_cycle_direction(payload.cycle_direction))).await
}

async fn handle_swap_with_master(payload: SwapWithMasterDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::SwapWithMaster(convert_swap_with_master_param(payload.param))).await
}
