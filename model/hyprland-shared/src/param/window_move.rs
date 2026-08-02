use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::identifier::monitor_identifier::HyprlandMonitorIdentifier;
use crate::param::direction::HyprlandDirection;

/// The kind of window move, matching `hyprland::dispatch::WindowMove` variants.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HyprlandWindowMoveKind {
    #[default]
    Direction,
    Monitor,
}

/// Parameters for moving a window, either by direction or to a specific monitor.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HyprlandWindowMove {
    /// The kind of move.
    pub kind: HyprlandWindowMoveKind,
    /// Direction value for the Direction variant.
    pub direction: HyprlandDirection,
    /// Monitor identifier for the Monitor variant.
    pub monitor: HyprlandMonitorIdentifier,
}

impl TypedMessage for HyprlandWindowMoveKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowMoveKind");
}

impl TypedMessage for HyprlandWindowMove {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandWindowMove");
}
