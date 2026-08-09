use hyprland::dispatch::WindowIdentifier;
use hyprland::shared::Address;
use smearor_hyprland_model::HyprlandWindowIdentifier;

pub(crate) struct OwnedWindowIdentifier {
    process_id: u32,
    address: Option<String>,
    class_regex: Option<String>,
    title: Option<String>,
    kind: OwnedWindowIdentifierKind,
}

#[derive(Clone, Copy)]
pub(crate) enum OwnedWindowIdentifierKind {
    ProcessId,
    Address,
    ClassRegularExpression,
    Title,
}

impl OwnedWindowIdentifier {
    pub(crate) fn as_ref(&self) -> WindowIdentifier<'_> {
        match self.kind {
            OwnedWindowIdentifierKind::ProcessId => WindowIdentifier::ProcessId(self.process_id),
            OwnedWindowIdentifierKind::Address => WindowIdentifier::Address(Address::new(self.address.as_deref().unwrap_or(""))),
            OwnedWindowIdentifierKind::ClassRegularExpression => WindowIdentifier::ClassRegularExpression(self.class_regex.as_deref().unwrap_or("")),
            OwnedWindowIdentifierKind::Title => WindowIdentifier::Title(self.title.as_deref().unwrap_or("")),
        }
    }
}

impl From<&HyprlandWindowIdentifier> for OwnedWindowIdentifier {
    fn from(id: &HyprlandWindowIdentifier) -> Self {
        id.match_ref(
            |pid| OwnedWindowIdentifier {
                process_id: *pid,
                address: None,
                class_regex: None,
                title: None,
                kind: OwnedWindowIdentifierKind::ProcessId,
            },
            |addr| OwnedWindowIdentifier {
                process_id: 0,
                address: Some(addr.to_string()),
                class_regex: None,
                title: None,
                kind: OwnedWindowIdentifierKind::Address,
            },
            |s| OwnedWindowIdentifier {
                process_id: 0,
                address: None,
                class_regex: Some(s.to_string()),
                title: None,
                kind: OwnedWindowIdentifierKind::ClassRegularExpression,
            },
            |s| OwnedWindowIdentifier {
                process_id: 0,
                address: None,
                class_regex: None,
                title: Some(s.to_string()),
                kind: OwnedWindowIdentifierKind::Title,
            },
        )
    }
}

pub(crate) fn convert_window_identifier_opt(id: &Option<HyprlandWindowIdentifier>) -> Option<OwnedWindowIdentifier> {
    id.as_ref().map(Into::into)
}
