use hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes;
use smearor_hyprland_model::HyprlandSwitchXkbLayoutCmd;
use smearor_hyprland_model::HyprlandSwitchXkbLayoutCmdKind;

pub(crate) fn convert_switch_xkb_layout_cmd(cmd: HyprlandSwitchXkbLayoutCmd) -> SwitchXKBLayoutCmdTypes {
    match cmd.kind {
        HyprlandSwitchXkbLayoutCmdKind::Next => SwitchXKBLayoutCmdTypes::Next,
        HyprlandSwitchXkbLayoutCmdKind::Previous => SwitchXKBLayoutCmdTypes::Previous,
        HyprlandSwitchXkbLayoutCmdKind::Id => SwitchXKBLayoutCmdTypes::Id(cmd.id),
    }
}
