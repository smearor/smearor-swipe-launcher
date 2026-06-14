use crate::config::ButtonConfig;
use gtk4::Align;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::prelude::*;
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
use tracing::debug;

pub struct ButtonWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ButtonConfig,
}

impl ButtonWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        // You can't create a button here, because the GTK thread is not running yet.
        debug!("ButtonWidget plugin config: {config:?}");
        let meta = PluginMeta::try_from(&config)?;
        debug!("ButtonWidget meta: {meta:?}");
        let config = ButtonConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        debug!("ButtonWidget button config: {config:?}");
        Ok(Self { meta, core_context, config })
    }
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
        let _ = adw::init();

        let button_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(30)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["menu_button_inner"])
            .build();

        if let Some(icon_name) = &self.config.icon {
            let icon = Image::from_icon_name(icon_name);
            icon.set_pixel_size(self.config.icon_size);
            button_box.append(&icon);
        }

        if !self.config.icon_only || self.config.icon.is_none() {
            let label = Label::new(Some(&self.config.text));
            button_box.append(&label);
        }

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.width)
            .child(&button_box)
            .build();

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

        button.clone().upcast::<Widget>()
    }
}
