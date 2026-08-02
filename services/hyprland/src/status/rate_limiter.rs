use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use smearor_hyprland_model::GroupEvent;
use smearor_hyprland_model::HyprlandGroupStatusMessage;
use smearor_hyprland_model::HyprlandLayerStatusMessage;
use smearor_hyprland_model::HyprlandSystemStatusMessage;
use smearor_hyprland_model::HyprlandWindowStatusMessage;
use smearor_hyprland_model::HyprlandWorkspaceStatusMessage;
use smearor_hyprland_model::LayerEvent;
use smearor_hyprland_model::SystemEvent;
use smearor_hyprland_model::WindowEvent;
use smearor_hyprland_model::WorkspaceEvent;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::PluginMeta;

/// Minimum interval between broadcasts of the same high-frequency event variant.
pub const RATE_LIMIT_MS: u64 = 50;

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
    fn is_high_frequency(&self) -> bool {
        matches!(self, StatusVariant::ActiveWindowChanged | StatusVariant::WindowTitleChanged)
    }
}

/// Per-variant rate limiter with trailing-edge debounce for high-frequency status events.
///
/// High-frequency variants use a throttle with trailing edge: if an event arrives
/// within the debounce window, it is stored as the trailing event. After the window
/// expires, the trailing event is flushed automatically. This prevents stale UI state
/// that would occur with a pure drop/throttle approach.
pub struct RateLimiter {
    last_broadcast: HashMap<StatusVariant, Instant>,
    /// Pending trailing event per variant, to be flushed after the debounce window.
    trailing: HashMap<StatusVariant, StatusEvent>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_broadcast: HashMap::new(),
            trailing: HashMap::new(),
        }
    }

    /// Try to broadcast an event immediately. If rate-limited, store it as trailing.
    /// Returns `Some(event)` if it should be broadcast now, `None` if it was stored as trailing.
    pub fn try_event(&mut self, event: StatusEvent) -> Option<StatusEvent> {
        let variant = StatusVariant::from(&event);
        if !variant.is_high_frequency() {
            return Some(event);
        }
        let now = Instant::now();
        if let Some(last) = self.last_broadcast.get(&variant) {
            if now.duration_since(*last) < Duration::from_millis(RATE_LIMIT_MS) {
                self.trailing.insert(variant, event);
                return None;
            }
        }
        self.last_broadcast.insert(variant, now);
        self.trailing.remove(&variant);
        Some(event)
    }

    /// Check if any trailing events are ready to be flushed after their debounce window expired.
    /// Returns `Some(event)` if a trailing event should be broadcast, `None` otherwise.
    pub fn flush_trailing(&mut self) -> Option<StatusEvent> {
        let now = Instant::now();
        for (variant, _event) in self.trailing.iter() {
            if let Some(last) = self.last_broadcast.get(variant) {
                if now.duration_since(*last) >= Duration::from_millis(RATE_LIMIT_MS) {
                    let variant = *variant;
                    let event = self.trailing.remove(&variant)?;
                    self.last_broadcast.insert(variant, now);
                    return Some(event);
                }
            }
        }
        None
    }

    /// Process a single Hyprland-specific status event.
    /// Applies rate limiting for high-frequency variants, then broadcasts.
    /// If an event is dropped by the rate limiter, it is stored as the pending trailing
    /// event. The trailing event is flushed by the periodic flush interval in the
    /// worker's `tokio::select!` loop (see `spawn_event_worker`), not here.
    pub fn process_event(&mut self, event: StatusEvent, core_context: &Option<FfiCoreContext>, meta: &PluginMeta) {
        if let Some(pending) = self.try_event(event) {
            Self::broadcast_event(core_context, meta, pending);
        }
    }

    /// Broadcast a status event to all launcher instances via the core context.
    /// Converts the internal `StatusEvent` to the appropriate `Hyprland*StatusMessage` type.
    pub fn broadcast_event(core_context: &Option<FfiCoreContext>, meta: &PluginMeta, event: StatusEvent) {
        let Some(ctx) = core_context else {
            return;
        };
        let broadcaster = MessageBroadcasterInner {
            meta: meta.clone(),
            core_context: Some(ctx.clone()),
        };
        match event {
            StatusEvent::Window(event) => {
                broadcaster.broadcast_message_to_topic(HyprlandWindowStatusMessage { event });
            }
            StatusEvent::Workspace(event) => {
                broadcaster.broadcast_message_to_topic(HyprlandWorkspaceStatusMessage { event });
            }
            StatusEvent::Group(event) => {
                broadcaster.broadcast_message_to_topic(HyprlandGroupStatusMessage { event });
            }
            StatusEvent::Layer(event) => {
                broadcaster.broadcast_message_to_topic(HyprlandLayerStatusMessage { event });
            }
            StatusEvent::System(event) => {
                broadcaster.broadcast_message_to_topic(HyprlandSystemStatusMessage { event });
            }
        }
    }
}
