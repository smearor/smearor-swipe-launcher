use hyprland::dispatch::Position;
use smearor_hyprland_model::HyprlandPosition;
use smearor_hyprland_model::HyprlandPositionKind;

pub(crate) fn convert_position(pos: HyprlandPosition) -> Position {
    match pos.kind {
        HyprlandPositionKind::Delta => Position::Delta(pos.x, pos.y),
        HyprlandPositionKind::Exact => Position::Exact(pos.x, pos.y),
    }
}
