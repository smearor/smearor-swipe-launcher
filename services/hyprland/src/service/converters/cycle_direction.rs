use hyprland::dispatch::CycleDirection;
use smearor_hyprland_model::HyprlandCycleDirection;

pub(crate) fn convert_cycle_direction(dir: HyprlandCycleDirection) -> CycleDirection {
    match dir {
        HyprlandCycleDirection::Next => CycleDirection::Next,
        HyprlandCycleDirection::Previous => CycleDirection::Previous,
    }
}
