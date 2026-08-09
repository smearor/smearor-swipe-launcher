use hyprland::dispatch::WindowSwitchDirection;
use smearor_hyprland_model::HyprlandWindowSwitchDirection;

pub(crate) fn convert_window_switch_direction(dir: HyprlandWindowSwitchDirection) -> WindowSwitchDirection {
    match dir {
        HyprlandWindowSwitchDirection::Back => WindowSwitchDirection::Back,
        HyprlandWindowSwitchDirection::Forward => WindowSwitchDirection::Forward,
    }
}
