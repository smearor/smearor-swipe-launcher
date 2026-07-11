use crate::config::GnomeWorkspaceServiceConfig;
use crate::monitor::MonitorEvent;
use crate::monitor::run_monitor_polling;
use crate::monitor::spawn_monitor_worker;
use crate::workspace::WorkspaceEvent;
use crate::workspace::run_workspace_polling;
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
            let ws_core_context = service.core_context;
            let ws_meta = service.meta.clone();
            let poll_interval = service.config.poll_interval_ms;
            let enable_workspace_lifecycle = service.config.enable_workspace_lifecycle;

            let (ws_sender, ws_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();

            std::thread::spawn(move || {
                run_workspace_polling(ws_sender, poll_interval, enable_workspace_lifecycle);
            });

            spawn_workspace_worker(ws_receiver, ws_core_context, ws_meta);
        }

        if service.config.enable_monitor_events {
            let mon_core_context = service.core_context;
            let mon_meta = service.meta.clone();
            let poll_interval = service.config.poll_interval_ms;

            let (mon_sender, mon_receiver) = mpsc::unbounded_channel::<MonitorEvent>();

            std::thread::spawn(move || {
                run_monitor_polling(mon_sender, poll_interval);
            });

            spawn_monitor_worker(mon_receiver, mon_core_context, mon_meta);
        }

        Ok(service)
    }
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
