use crate::clock::Clock;
use crate::config::ClockConfig;
use adw::StatusPage;
use adw::prelude::Cast;
use gtk4::GestureClick;
use gtk4::Widget;
use gtk4::glib::ControlFlow;
use gtk4::glib::timeout_add_seconds_local;
use gtk4::prelude::WidgetExt;
use serde_json;
use serde_json::Value;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::mpsc;
use tokio::runtime::Runtime;
use tokio::time::interval;
use tracing::debug;

pub(crate) struct ClockWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ClockConfig,
    pub(crate) clock: Arc<Clock>,
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) status_page: Arc<RwLock<Option<StatusPage>>>,
    pub(crate) time_sender: mpsc::Sender<String>,
    pub(crate) time_receiver: Option<mpsc::Receiver<String>>,
}

impl ClockWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let clock_config: ClockConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let runtime =
            Arc::new(Runtime::new().map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToCreateRuntime, e.to_string().into()))?);
        let (time_sender, time_receiver) = mpsc::channel();
        Ok(ClockWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: clock_config.clone(),
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
                let _ = time_sender.send(clock.get_current_time_1());
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

impl MessageHandler<FfiEnvelope> for ClockWidget {
    fn handle_message(&self, message: FfiEnvelope) {
        let topic = message.topic.to_string();
        let payload = message.payload.to_string();
        debug!("Clock widget {} received message on topic '{}' with payload '{}'", self.meta.id, topic, payload);
    }
}

impl MessageBroadcaster<Value> for ClockWidget {}

impl PluginMetaGetter for ClockWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for ClockWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetBuilder for ClockWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();
        let mut status_page = StatusPage::builder().title(self.clock.get_current_time_1());
        if let Some(current_time_2) = self.clock.get_current_time_2() {
            debug!("Current time 2: {}", current_time_2);
            status_page = status_page.description(current_time_2);
        }
        if let Some(width) = self.config.width {
            status_page = status_page.width_request(width);
        }

        let status_page = status_page.build();
        status_page.add_css_class("smart-desk-clock");

        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let message_broadcaster = self.get_broadcaster();
        let gesture = GestureClick::new();
        gesture.connect_released(move |_gesture, _, _, _| {
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                message_broadcaster.broadcast_message(&topic, &payload);
            }
        });
        status_page.add_controller(gesture);

        *self.status_page.write().unwrap() = Some(status_page.clone());
        if let Some(time_receiver) = self.time_receiver.take() {
            self.start_time_update(time_receiver);
        }
        status_page.upcast::<Widget>()
    }
}
