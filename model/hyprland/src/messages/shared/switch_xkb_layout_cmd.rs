use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// The kind of switch-xkb-layout command, matching `hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes` variants.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandSwitchXkbLayoutCmdKind {
    #[default]
    Next,
    Previous,
    Id,
}

/// Parameters for switching the XKB keyboard layout.
#[stabby::stabby(no_opt)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HyprlandSwitchXkbLayoutCmd {
    /// The kind of command.
    pub kind: HyprlandSwitchXkbLayoutCmdKind,
    /// Layout id for the Id variant.
    pub id: u8,
}

impl TypedMessage for HyprlandSwitchXkbLayoutCmdKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandSwitchXkbLayoutCmdKind");
}

impl TypedMessage for HyprlandSwitchXkbLayoutCmd {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandSwitchXkbLayoutCmd");
}
