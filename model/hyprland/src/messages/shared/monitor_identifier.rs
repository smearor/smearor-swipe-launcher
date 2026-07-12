use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::messages::shared::direction::HyprlandDirection;

/// The kind of monitor identifier, matching `hyprland::dispatch::MonitorIdentifier` variants.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandMonitorIdentifierKind {
    #[default]
    Current,
    Direction,
    Id,
    Name,
    Relative,
}

/// Identifies a monitor by direction, id, name, current, or relative offset.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct HyprlandMonitorIdentifier {
    /// The kind of identifier.
    pub kind: HyprlandMonitorIdentifierKind,
    /// Direction value for the Direction variant.
    pub direction: HyprlandDirection,
    /// Numeric value for the Id variant.
    pub id: i32,
    /// Name value for the Name variant.
    pub name: stabby::option::Option<stabby::string::String>,
    /// Relative offset for the Relative variant.
    pub relative: i32,
}

impl TypedMessage for HyprlandMonitorIdentifierKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandMonitorIdentifierKind");
}

impl TypedMessage for HyprlandMonitorIdentifier {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandMonitorIdentifier");
}
