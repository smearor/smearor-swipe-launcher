use crate::clock::Clock;
use crate::config::ClockConfig;
use adw::StatusPage;
use adw::prelude::Cast;
use gtk4::GestureClick;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::WidgetExt;
use serde_json;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

pub(crate) struct ClockWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ClockConfig,
    pub(crate) clock: Arc<Clock>,
    pub(crate) status_page: Arc<RwLock<Option<StatusPage>>>,
    pub(crate) time_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
}

impl ClockWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let clock_config: ClockConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        Ok(ClockWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: clock_config.clone(),
            clock: Arc::new(Clock::new(clock_config)),
            status_page: Arc::new(RwLock::new(None)),
            time_receiver: None,
        })
    }

    pub(crate) fn start_time_update(&mut self) {
        let (time_sender, time_receiver) = tokio::sync::mpsc::unbounded_channel::<String>();
        self.time_receiver = Some(time_receiver);

        let clock = self.clock.clone();
        thread::spawn(move || {
            loop {
                let time_str = clock.get_current_time_1();
                if time_sender.send(time_str).is_err() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        if let Some(mut rx) = self.time_receiver.take() {
            let status_page = self.status_page.clone();
            MainContext::default().spawn_local(async move {
                while let Some(time_str) = rx.recv().await {
                    if let Ok(page_guard) = status_page.read() {
                        if let Some(page) = page_guard.as_ref() {
                            page.set_title(&time_str);
                        }
                    }
                }
            });
        }
    }
}

impl MessageHandler<FfiEnvelope> for ClockWidget {
    fn handle_message(&self, _message: FfiEnvelope, _sender_id: &str) {}
}

impl AcceptTopic<FfiEnvelope> for ClockWidget {
    fn accept_topic(&self, _topic: &str) -> bool {
        false
    }
}

impl MessageBroadcaster for ClockWidget {}

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

impl Plugin for ClockWidget {}

impl WidgetBuilder for ClockWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();
        let mut status_page = StatusPage::builder().title(self.clock.get_current_time_1());
        if let Some(current_time_2) = self.clock.get_current_time_2() {
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
                let payload_str = payload.to_string();
                message_broadcaster.broadcast_string(&topic, &payload_str);
            }
        });
        status_page.add_controller(gesture);

        *self.status_page.write().unwrap() = Some(status_page.clone());
        self.start_time_update();
        status_page.upcast::<Widget>()
    }
}
