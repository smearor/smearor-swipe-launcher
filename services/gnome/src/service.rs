use crate::config::GnomeWorkspaceServiceConfig;
use crate::workspace::WorkspaceEvent;
use crate::workspace::run_workspace_polling;
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

/// GNOME workspace tracking service plugin.
///
/// Uses D-Bus to poll the active workspace via `org.gnome.Shell.Eval` every
/// 500ms (configurable). When the workspace changes, resolves the monitor
/// index via `org.gnome.Mutter.DisplayConfig` and broadcasts a
/// `WorkspaceChangedEvent` on `TOPIC_WORKSPACE_CHANGED`.
pub struct GnomeWorkspaceService {
    /// Plugin metadata.
    pub meta: PluginMeta,
    /// Optional core context for broadcasting messages.
    pub core_context: Option<FfiCoreContext>,
    /// Shared configuration for the service.
    pub config: Arc<GnomeWorkspaceServiceConfig>,
}

impl GnomeWorkspaceService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!(
            "GNOME workspace service: initializing, core_context is {}",
            if core_context.is_some() { "Some" } else { "None" }
        );

        let service_config: GnomeWorkspaceServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let service_config = Arc::new(service_config);
        let service = GnomeWorkspaceService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: service_config,
        };

        if service.config.enable_workspace_tracking {
            let event_core_context = service.core_context;
            let event_meta = service.meta.clone();
            let poll_interval = service.config.poll_interval_ms;

            let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();

            // D-Bus polling thread — connects to the GNOME session D-Bus and
            // polls the active workspace at the configured interval.
            std::thread::spawn(move || {
                run_workspace_polling(event_sender, poll_interval);
            });

            // Event worker thread — receives workspace change events from the
            // polling thread and broadcasts them to the launcher core.
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(error) => {
                        error!("GNOME service: failed to create event worker runtime: {error}");
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

impl MessageBroadcaster for GnomeWorkspaceService {}

impl PluginMetaGetter for GnomeWorkspaceService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for GnomeWorkspaceService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for GnomeWorkspaceService {
    fn on_message(&mut self, _message: *mut core::ffi::c_void) {
        // The GNOME workspace service does not handle incoming messages.
    }
}
