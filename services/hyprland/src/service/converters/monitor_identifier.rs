use super::direction::convert_direction;
use hyprland::dispatch::MonitorIdentifier;
use smearor_hyprland_model::HyprlandMonitorIdentifier;
use smearor_hyprland_model::HyprlandMonitorIdentifierKind;

pub(crate) fn convert_monitor_identifier<'a>(id: &'a HyprlandMonitorIdentifier, name: Option<&'a str>) -> MonitorIdentifier<'a> {
    match id.kind {
        HyprlandMonitorIdentifierKind::Current => MonitorIdentifier::Current,
        HyprlandMonitorIdentifierKind::Direction => MonitorIdentifier::Direction(convert_direction(id.direction)),
        HyprlandMonitorIdentifierKind::Id => MonitorIdentifier::Id(id.id as i128),
        HyprlandMonitorIdentifierKind::Name => MonitorIdentifier::Name(name.unwrap_or("")),
        HyprlandMonitorIdentifierKind::Relative => MonitorIdentifier::Relative(id.relative),
    }
}
