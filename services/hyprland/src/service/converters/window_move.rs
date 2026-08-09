use super::direction::convert_direction;
use super::monitor_identifier::convert_monitor_identifier;
use hyprland::dispatch::WindowMove;
use smearor_hyprland_model::HyprlandDirection;
use smearor_hyprland_model::HyprlandMonitorIdentifier;
use smearor_hyprland_model::HyprlandWindowMove;
use smearor_hyprland_model::HyprlandWindowMoveKind;

pub(crate) struct OwnedWindowMove {
    monitor_name: Option<String>,
    direction: HyprlandDirection,
    kind: HyprlandWindowMoveKind,
    monitor: HyprlandMonitorIdentifier,
}

impl OwnedWindowMove {
    pub(crate) fn as_ref(&self) -> WindowMove<'_> {
        match self.kind {
            HyprlandWindowMoveKind::Direction => WindowMove::Direction(convert_direction(self.direction)),
            HyprlandWindowMoveKind::Monitor => {
                let name_ref = self.monitor_name.as_ref().map(|n| n.as_str());
                WindowMove::Monitor(convert_monitor_identifier(&self.monitor, name_ref))
            }
        }
    }
}

impl From<&HyprlandWindowMove> for OwnedWindowMove {
    fn from(wm: &HyprlandWindowMove) -> Self {
        let name: Option<stabby::string::String> = wm.monitor.name.clone().into();
        OwnedWindowMove {
            monitor_name: name.map(|n| n.to_string()),
            direction: wm.direction,
            kind: wm.kind,
            monitor: wm.monitor.clone(),
        }
    }
}
