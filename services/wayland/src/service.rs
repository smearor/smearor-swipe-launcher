use crate::config::WaylandWorkspaceServiceConfig;
use crate::monitor::MonitorEvent;
use crate::monitor::spawn_monitor_worker;
use crate::workspace::WorkspaceEvent;
use crate::workspace::run_workspace_event_loop;
use crate::workspace::spawn_workspace_worker;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

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
            let ws_core_context = service.core_context;
            let ws_meta = service.meta.clone();
            let mon_core_context = service.core_context;
            let mon_meta = service.meta.clone();

            let (workspace_sender, workspace_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();
            let (monitor_sender, monitor_receiver) = mpsc::unbounded_channel::<MonitorEvent>();

            // Wayland event listener thread — connects to the display and
            // dispatches protocol events using the ext-workspace-unstable-v1
            // protocol and wl_output for monitor events.
            std::thread::spawn(move || {
                run_workspace_event_loop(workspace_sender, monitor_sender);
            });

            // Workspace event worker thread
            spawn_workspace_worker(workspace_receiver, ws_core_context, ws_meta);

            // Monitor event worker thread
            spawn_monitor_worker(monitor_receiver, mon_core_context, mon_meta);
        }

        Ok(service)
    }
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
