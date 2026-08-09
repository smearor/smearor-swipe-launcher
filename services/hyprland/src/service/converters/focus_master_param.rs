use hyprland::dispatch::FocusMasterParam;
use smearor_hyprland_model::HyprlandFocusMasterParam;

pub(crate) fn convert_focus_master_param(param: HyprlandFocusMasterParam) -> FocusMasterParam {
    match param {
        HyprlandFocusMasterParam::Master => FocusMasterParam::Master,
        HyprlandFocusMasterParam::Auto => FocusMasterParam::Auto,
    }
}
