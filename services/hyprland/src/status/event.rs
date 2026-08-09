use smearor_hyprland_model::GroupEvent;
use smearor_hyprland_model::LayerEvent;
use smearor_hyprland_model::SystemEvent;
use smearor_hyprland_model::WindowEvent;
use smearor_hyprland_model::WorkspaceEvent;

/// Internal non-stabby enum for routing status events through the channel and rate limiter.
/// Converted to the appropriate `Hyprland*StatusMessage` at broadcast time.
#[derive(Clone, Debug)]
pub enum StatusEvent {
    /// Window-related status event.
    Window(WindowEvent),
    /// Workspace-related status event.
    Workspace(WorkspaceEvent),
    /// Window group-related status event.
    Group(GroupEvent),
    /// Layer shell surface status event.
    Layer(LayerEvent),
    /// System-level status event.
    System(SystemEvent),
}

/// Lightweight classification of status event variants for rate limiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusVariant {
    ActiveWindowChanged,
    FullscreenStateChanged,
    WindowOpened,
    WindowClosed,
    WindowMoved,
    KeyboardLayoutChanged,
    SubMapChanged,
    LayerOpened,
    LayerClosed,
    FloatStateChanged,
    UrgentStateChanged,
    WindowTitleChanged,
    WorkspaceRenamed,
    SpecialRemoved,
    ChangedSpecial,
    Screencast,
    ConfigReloaded,
    IgnoreGroupLockStateChanged,
    LockGroupsStateChanged,
    WindowPinned,
    GroupToggled,
    WindowMovedIntoGroup,
    WindowMovedOutOfGroup,
    Unknown,
}

impl From<&StatusEvent> for StatusVariant {
    fn from(event: &StatusEvent) -> Self {
        match event {
            StatusEvent::Window(window_event) => window_event.match_ref(
                |_| StatusVariant::ActiveWindowChanged,
                |_| StatusVariant::WindowOpened,
                |_| StatusVariant::WindowClosed,
                |_| StatusVariant::WindowMoved,
                |_| StatusVariant::FloatStateChanged,
                |_| StatusVariant::UrgentStateChanged,
                |_| StatusVariant::WindowTitleChanged,
                |_| StatusVariant::WindowPinned,
            ),
            StatusEvent::Workspace(workspace_event) => workspace_event.match_ref(
                |_| StatusVariant::FullscreenStateChanged,
                |_| StatusVariant::WorkspaceRenamed,
                |_| StatusVariant::SpecialRemoved,
                |_| StatusVariant::ChangedSpecial,
                |_| StatusVariant::SubMapChanged,
            ),
            StatusEvent::Group(group_event) => group_event.match_ref(
                |_| StatusVariant::GroupToggled,
                |_| StatusVariant::WindowMovedIntoGroup,
                |_| StatusVariant::WindowMovedOutOfGroup,
                |_| StatusVariant::IgnoreGroupLockStateChanged,
                |_| StatusVariant::LockGroupsStateChanged,
            ),
            StatusEvent::Layer(layer_event) => layer_event.match_ref(|_| StatusVariant::LayerOpened, |_| StatusVariant::LayerClosed),
            StatusEvent::System(system_event) => {
                system_event.match_ref(|_| StatusVariant::KeyboardLayoutChanged, |_| StatusVariant::Screencast, |_| StatusVariant::ConfigReloaded)
            }
        }
    }
}

impl StatusVariant {
    /// Returns true if this variant is high-frequency and should be rate-limited.
    pub(crate) fn is_high_frequency(&self) -> bool {
        matches!(self, StatusVariant::ActiveWindowChanged | StatusVariant::WindowTitleChanged)
    }
}
