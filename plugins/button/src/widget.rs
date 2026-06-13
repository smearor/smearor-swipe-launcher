use crate::config::ButtonConfig;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureLongPress;
use gtk4::Widget;
use gtk4::prelude::*;
use serde_json::Value;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::info;

pub struct ButtonWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ButtonConfig,
}

impl ButtonWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        // You can't create a button here, because the GTK thread is not running yet.
        debug!("ButtonWidget plugin config: {config:?}");
        let meta = PluginMeta::try_from(&config)?;
        debug!("ButtonWidget meta: {meta:?}");
        let config = ButtonConfig::parse(&config.config).map_err(|e| PluginConstructionError::FailedToParseWidgetConfig(e.to_string().into()))?;
        debug!("ButtonWidget button config: {config:?}");
        Ok(Self { meta, core_context, config })
    }

    fn setup_handlers(&self) {}
}

impl MessageHandler<FfiEnvelope> for ButtonWidget {
    fn handle_message(&self, _message: FfiEnvelope) {}

    fn accept_topic(&self, _topic: &str) -> bool {
        false
    }
}

impl MessageBroadcaster<Value> for ButtonWidget {}

impl PluginMetaGetter for ButtonWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for ButtonWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetBuilder for ButtonWidget {
    fn build_widget(&mut self) -> Widget {
        info!("build widget 1");
        let button = Button::new();
        info!("build widget 2");
        button.set_label(&self.config.text);
        info!("build widget 3");
        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let message_broadcaster = self.get_broadcaster();
        button.connect_clicked(move |_| {
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                message_broadcaster.broadcast_message(&topic, &payload);
            }
        });

        let long_press_topic = self.config.longpress_topic.clone();
        let long_press_payload = self.config.longpress_payload.clone();
        let long_press_gesture = GestureLongPress::new();
        let message_broadcaster = self.get_broadcaster();
        long_press_gesture.connect_pressed(move |gesture, _, _| {
            if let (Some(topic), Some(payload)) = (long_press_topic.clone(), long_press_payload.clone()) {
                message_broadcaster.broadcast_message(&topic, &payload);
                gesture.set_state(EventSequenceState::Claimed);
            }
        });

        button.add_controller(long_press_gesture);
        info!("build widget 4");
        button.clone().upcast::<Widget>()
    }
}
