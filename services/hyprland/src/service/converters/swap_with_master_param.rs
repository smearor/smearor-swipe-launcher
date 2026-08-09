use hyprland::dispatch::SwapWithMasterParam;
use smearor_hyprland_model::HyprlandSwapWithMasterParam;

pub(crate) fn convert_swap_with_master_param(param: HyprlandSwapWithMasterParam) -> SwapWithMasterParam {
    match param {
        HyprlandSwapWithMasterParam::Master => SwapWithMasterParam::Master,
        HyprlandSwapWithMasterParam::Child => SwapWithMasterParam::Child,
        HyprlandSwapWithMasterParam::Auto => SwapWithMasterParam::Auto,
    }
}
