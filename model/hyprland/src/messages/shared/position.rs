use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// The kind of position, matching `hyprland::dispatch::Position` variants.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandPositionKind {
    #[default]
    Delta,
    Exact,
}

/// Position for move-window commands, expressed as either a delta or an exact coordinate.
#[stabby::stabby(no_opt)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HyprlandPosition {
    /// The kind of position.
    pub kind: HyprlandPositionKind,
    /// The x coordinate.
    pub x: i16,
    /// The y coordinate.
    pub y: i16,
}

impl TypedMessage for HyprlandPositionKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandPositionKind");
}

impl TypedMessage for HyprlandPosition {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandPosition");
}
