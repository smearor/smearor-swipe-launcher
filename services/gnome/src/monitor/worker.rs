use crate::monitor::event::MonitorEvent;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use tokio::sync::mpsc;
use tracing::debug;

/// Spawn the monitor event worker thread that broadcasts `MonitorChangedEvent`s
/// to the launcher core.
pub fn spawn_monitor_worker(mut event_receiver: mpsc::UnboundedReceiver<MonitorEvent>, core_context: Option<FfiCoreContext>, meta: PluginMeta) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                tracing::error!("GNOME monitor worker: failed to create runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            while let Some(event) = event_receiver.recv().await {
                match event {
                    MonitorEvent::Changed(event) => {
                        debug!("Broadcasting monitor changed event: {:?}", event);
                        broadcast_event(&core_context, &meta, event);
                    }
                }
            }
        });
    });
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
        core_context: Some(*ctx),
    };
    broadcaster.broadcast_message_to_topic(event);
}
