use super::event::MonitorEvent;
use crate::service::HyprlandSharedState;
use hyprland::shared::HyprData;
use smearor_model_compositor::MonitorChangeType;
use smearor_model_compositor::MonitorChangedEvent;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::trace;
use tracing::warn;

/// Process a single monitor event. Atomic, focused.
pub async fn process_event(event: MonitorEvent, core_context: &Option<FfiCoreContext>, meta: &PluginMeta, shared_state: &Arc<Mutex<HyprlandSharedState>>) {
    match event {
        MonitorEvent::Added(name) => {
            trace!("Monitor added: {}", name);
            let monitor_index = resolve_monitor_index_by_name(&name).await.unwrap_or(0);
            let event = MonitorChangedEvent {
                monitor_index,
                connector_name: name.into(),
                change_type: MonitorChangeType::Connected,
            };
            if let Ok(mut guard) = shared_state.lock() {
                guard.latest_monitor_changed = Some(event.clone());
            }
            broadcast_event(core_context, meta, event);
        }
        MonitorEvent::Removed(name) => {
            trace!("Monitor removed: {}", name);
            let monitor_index = resolve_monitor_index_by_name(&name).await.unwrap_or(0);
            let event = MonitorChangedEvent {
                monitor_index,
                connector_name: name.into(),
                change_type: MonitorChangeType::Disconnected,
            };
            if let Ok(mut guard) = shared_state.lock() {
                guard.latest_monitor_changed = Some(event.clone());
            }
            broadcast_event(core_context, meta, event);
        }
    }
}

/// Resolve a monitor's Hyprland ID by its connector name.
async fn resolve_monitor_index_by_name(name: &str) -> Option<u32> {
    let monitors = match hyprland::data::Monitors::get() {
        Ok(monitors) => monitors,
        Err(error) => {
            warn!("Failed to query monitors for '{name}': {error}");
            return None;
        }
    };
    for monitor in monitors {
        if monitor.name == name {
            return Some(monitor.id as u32);
        }
    }
    None
}

/// Broadcast an event to all launcher instances via the core context.
fn broadcast_event<T>(core_context: &Option<FfiCoreContext>, meta: &PluginMeta, event: T)
where
    T: Clone + MessageTopic + TypedMessage,
{
    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: meta.clone(),
        core_context: Some(ctx.clone()),
    };
    broadcaster.broadcast_message_to_topic(event);
}
