use hyprland::dispatch::FullscreenType;
use smearor_hyprland_model::HyprlandFullscreenType;

pub(crate) fn convert_fullscreen_type(fullscreen_type: HyprlandFullscreenType) -> FullscreenType {
    match fullscreen_type {
        HyprlandFullscreenType::Real => FullscreenType::Real,
        HyprlandFullscreenType::Maximize => FullscreenType::Maximize,
        HyprlandFullscreenType::NoParam => FullscreenType::NoParam,
    }
}
