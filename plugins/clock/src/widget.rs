use crate::clock::Clock;
use crate::config::ClockConfig;
use adw::StatusPage;
use gtk4::glib::ControlFlow;
use gtk4::glib::timeout_add_seconds_local;
use serde_json;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::mpsc;
use tokio::runtime::Runtime;
use tokio::time::interval;

pub(crate) struct ClockWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) clock: Arc<Clock>,
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) status_page: Arc<RwLock<Option<StatusPage>>>,
    pub(crate) time_sender: mpsc::Sender<String>,
    pub(crate) time_receiver: Option<mpsc::Receiver<String>>,
}

impl ClockWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        let clock_config: ClockConfig =
            serde_json::from_value(config.config.clone()).map_err(|e| PluginConstructionError::FailedToParseWidgetConfig(e.to_string().into()))?;
        let runtime = Arc::new(Runtime::new().map_err(|e| PluginConstructionError::FailedToCreateRuntime(e.to_string().into()))?);
        let (time_sender, time_receiver) = mpsc::channel();
        Ok(ClockWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            clock: Arc::new(Clock::new(clock_config)),
            runtime,
            status_page: Arc::new(RwLock::new(None)),
            time_sender,
            time_receiver: Some(time_receiver),
        })
    }

    pub(crate) fn start_time_update(&self, time_receiver: mpsc::Receiver<String>) {
        let clock = self.clock.clone();
        let runtime = self.runtime.clone();
        let time_sender = self.time_sender.clone();
        runtime.spawn(async move {
            let mut interval = interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let _ = time_sender.send(clock.get_current_time());
            }
        });

        let status_page = self.status_page.clone();
        timeout_add_seconds_local(1, move || {
            if let Ok(time_str) = time_receiver.try_recv() {
                if let Ok(page_guard) = status_page.read() {
                    if let Some(page) = page_guard.as_ref() {
                        page.set_title(&time_str);
                    }
                }
            }
            ControlFlow::Continue
        });
    }
}
