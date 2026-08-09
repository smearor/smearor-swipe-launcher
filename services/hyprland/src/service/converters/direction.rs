use hyprland::dispatch::Direction;
use smearor_hyprland_model::HyprlandDirection;

pub(crate) fn convert_direction(direction: HyprlandDirection) -> Direction {
    match direction {
        HyprlandDirection::Up => Direction::Up,
        HyprlandDirection::Down => Direction::Down,
        HyprlandDirection::Left => Direction::Left,
        HyprlandDirection::Right => Direction::Right,
    }
}
