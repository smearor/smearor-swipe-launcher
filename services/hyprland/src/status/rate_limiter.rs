use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use super::event::StatusEvent;
use super::event::StatusVariant;
use crate::service::HyprlandSharedState;
use smearor_hyprland_model::HyprlandGroupStatusMessage;
use smearor_hyprland_model::HyprlandLayerStatusMessage;
use smearor_hyprland_model::HyprlandSystemStatusMessage;
use smearor_hyprland_model::HyprlandWindowStatusMessage;
use smearor_hyprland_model::HyprlandWorkspaceStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::PluginMeta;

/// Minimum interval between broadcasts of the same high-frequency event variant.
pub const RATE_LIMIT_MS: u64 = 50;

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
    pub fn process_event(
        &mut self,
        event: StatusEvent,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        shared_state: &Arc<Mutex<HyprlandSharedState>>,
    ) {
        cache_event(&event, shared_state);
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

/// Cache a status event into the shared state for MCP resource queries.
fn cache_event(event: &StatusEvent, shared_state: &Arc<Mutex<HyprlandSharedState>>) {
    if let Ok(mut guard) = shared_state.lock() {
        match event {
            StatusEvent::Window(e) => guard.latest_window_event = Some(e.clone()),
            StatusEvent::Workspace(e) => guard.latest_workspace_event = Some(e.clone()),
            StatusEvent::Group(e) => guard.latest_group_event = Some(e.clone()),
            StatusEvent::Layer(e) => guard.latest_layer_event = Some(e.clone()),
            StatusEvent::System(e) => guard.latest_system_event = Some(e.clone()),
        }
    }
}
