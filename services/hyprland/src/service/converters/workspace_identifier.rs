use hyprland::dispatch::FirstEmpty;
use smearor_hyprland_model::HyprlandWorkspaceIdentifier;
use smearor_hyprland_model::HyprlandWorkspaceIdentifierKind;
use smearor_hyprland_model::HyprlandWorkspaceIdentifierWithSpecial;

pub(crate) struct OwnedWorkspaceIdentifierWithSpecial {
    id: i32,
    name: Option<String>,
    special_name: Option<String>,
    kind: HyprlandWorkspaceIdentifierKind,
}

impl OwnedWorkspaceIdentifierWithSpecial {
    pub(crate) fn as_ref(&self) -> hyprland::dispatch::WorkspaceIdentifierWithSpecial<'_> {
        match self.kind {
            HyprlandWorkspaceIdentifierKind::Id => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(self.id),
            HyprlandWorkspaceIdentifierKind::Relative => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Relative(self.id),
            HyprlandWorkspaceIdentifierKind::RelativeMonitor => hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitor(self.id),
            HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => {
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(self.id)
            }
            HyprlandWorkspaceIdentifierKind::RelativeOpen => hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeOpen(self.id),
            HyprlandWorkspaceIdentifierKind::Previous => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Previous,
            HyprlandWorkspaceIdentifierKind::Empty => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Empty(FirstEmpty {
                on_monitor: false,
                next: false,
            }),
            HyprlandWorkspaceIdentifierKind::Name => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(self.name.as_deref().unwrap_or("")),
            HyprlandWorkspaceIdentifierKind::Special => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(self.special_name.as_deref()),
        }
    }
}

impl From<&HyprlandWorkspaceIdentifierWithSpecial> for OwnedWorkspaceIdentifierWithSpecial {
    fn from(id: &HyprlandWorkspaceIdentifierWithSpecial) -> Self {
        let name: Option<stabby::string::String> = id.name.clone().into();
        let special_name: Option<stabby::string::String> = id.special_name.clone().into();
        OwnedWorkspaceIdentifierWithSpecial {
            id: id.id,
            name: name.map(|n| n.to_string()),
            special_name: special_name.map(|n| n.to_string()),
            kind: id.kind,
        }
    }
}

impl From<&HyprlandWorkspaceIdentifier> for OwnedWorkspaceIdentifierWithSpecial {
    fn from(id: &HyprlandWorkspaceIdentifier) -> Self {
        id.match_ref(
            || OwnedWorkspaceIdentifierWithSpecial {
                id: 0,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::Previous,
            },
            || OwnedWorkspaceIdentifierWithSpecial {
                id: 0,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::Empty,
            },
            |i| OwnedWorkspaceIdentifierWithSpecial {
                id: *i,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::Id,
            },
            |i| OwnedWorkspaceIdentifierWithSpecial {
                id: *i,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::Relative,
            },
            |i| OwnedWorkspaceIdentifierWithSpecial {
                id: *i,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::RelativeMonitor,
            },
            |i| OwnedWorkspaceIdentifierWithSpecial {
                id: *i,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty,
            },
            |i| OwnedWorkspaceIdentifierWithSpecial {
                id: *i,
                name: None,
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::RelativeOpen,
            },
            |s| OwnedWorkspaceIdentifierWithSpecial {
                id: 0,
                name: Some(s.to_string()),
                special_name: None,
                kind: HyprlandWorkspaceIdentifierKind::Name,
            },
        )
    }
}
