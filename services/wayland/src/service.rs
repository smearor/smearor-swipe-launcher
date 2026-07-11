use crate::config::WaylandWorkspaceServiceConfig;
use crate::workspace::WorkspaceEvent;
use crate::workspace::run_workspace_event_loop;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;

/// Wayland workspace tracking service plugin.
///
/// Connects to the Wayland display and uses the `ext-workspace-unstable-v1`
/// protocol to detect workspace changes. Broadcasts `WorkspaceChangedEvent`
/// on `TOPIC_WORKSPACE_CHANGED` whenever the active workspace changes.
pub struct WaylandWorkspaceService {
    /// Plugin metadata.
    pub meta: PluginMeta,
    /// Optional core context for broadcasting messages.
    pub core_context: Option<FfiCoreContext>,
    /// Shared configuration for the service.
    pub config: Arc<WaylandWorkspaceServiceConfig>,
}

impl WaylandWorkspaceService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!(
            "Wayland workspace service: initializing, core_context is {}",
            if core_context.is_some() { "Some" } else { "None" }
        );

        let service_config: WaylandWorkspaceServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let service_config = Arc::new(service_config);
        let service = WaylandWorkspaceService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: service_config,
        };

        if service.config.enable_workspace_tracking {
            let event_core_context = service.core_context;
            let event_meta = service.meta.clone();

            let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();

            // Wayland event listener thread — connects to the display and
            // dispatches protocol events using the ext-workspace-unstable-v1
            // protocol.
            std::thread::spawn(move || {
                run_workspace_event_loop(event_sender);
            });

            // Event worker thread — receives workspace change events from the
            // listener and broadcasts them to the launcher core.
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(error) => {
                        error!("Wayland service: failed to create event worker runtime: {error}");
                        return;
                    }
                };

                rt.block_on(async move {
                    while let Some(event) = event_receiver.recv().await {
                        match event {
                            WorkspaceEvent::WorkspaceChanged(event) => {
                                debug!("Broadcasting workspace changed event: {:?}", event);
                                broadcast_event(&event_core_context, &event_meta, event);
                            }
                        }
                    }
                });
            });
        }

        Ok(service)
    }
}

/// Broadcast a `WorkspaceChangedEvent` to all launcher instances via the core context.
fn broadcast_event(core_context: &Option<FfiCoreContext>, meta: &PluginMeta, event: WorkspaceChangedEvent) {
    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: meta.clone(),
        core_context: Some(*ctx),
    };
    broadcaster.broadcast_message_to_topic(event);
}

impl MessageBroadcaster for WaylandWorkspaceService {}

impl PluginMetaGetter for WaylandWorkspaceService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WaylandWorkspaceService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for WaylandWorkspaceService {
    fn on_message(&mut self, _message: *mut core::ffi::c_void) {
        // The Wayland workspace service does not handle incoming messages.
    }
}
