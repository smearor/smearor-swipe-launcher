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
use smearor_swipe_launcher_plugin_api::resolve_nerd_font;
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

impl AcceptTopic<FfiEnvelope> for ButtonWidget {
    fn accept_topic(&self, _topic: &str) -> bool {
        false
    }
}

impl MessageHandler<FfiEnvelope> for ButtonWidget {
    fn handle_message(&self, _message: FfiEnvelope, _sender_id: &str) {}
}

impl MessageBroadcaster for ButtonWidget {}

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

impl Plugin for ButtonWidget {}

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
            if icon_name.starts_with("nf-") {
                if let Some(glyph) = resolve_nerd_font(icon_name) {
                    let markup = format!(r#"<span font_desc="NerdFontsSymbolsOnly {}">{}</span>"#, self.config.icon_size, glyph);
                    let icon_label = Label::new(None);
                    icon_label.set_markup(&markup);
                    for class in &self.config.css_classes {
                        icon_label.add_css_class(class);
                    }
                    button_box.append(&icon_label);
                }
            } else {
                let icon = Image::from_icon_name(icon_name);
                icon.set_pixel_size(self.config.icon_size);
                button_box.append(&icon);
            }
        }

        if !self.config.icon_only || self.config.icon.is_none() {
            let label = Label::new(Some(&self.config.text));
            button_box.append(&label);
        }

        let mut css_classes = vec!["scroll-item", "menu-button"];
        css_classes.extend(self.config.css_classes.iter().map(String::as_str));
        let button = Button::builder()
            .css_classes(css_classes.as_slice())
            .width_request(self.config.width)
            .child(&button_box)
            .build();

        if let Some(tooltip) = &self.config.tooltip {
            button.set_tooltip_text(Some(tooltip));
        }

        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let click_instance = self.config.click_instance.clone();
        let message_broadcaster = self.get_broadcaster();
        button.connect_clicked(move |_| {
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = click_instance.clone() {
                    message_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    message_broadcaster.broadcast_string(&topic, &payload_str);
                }
            }
        });

        let long_press_topic = self.config.longpress_topic.clone();
        let long_press_payload = self.config.longpress_payload.clone();
        let long_press_instance = self.config.longpress_instance.clone();
        let long_press_gesture = GestureLongPress::new();
        let message_broadcaster = self.get_broadcaster();
        let button_weak = button.downgrade();
        long_press_gesture.connect_pressed(move |gesture, _, _| {
            if let Some(btn) = button_weak.upgrade() {
                btn.add_css_class("longpress-active");
            }
            if let (Some(topic), Some(payload)) = (long_press_topic.clone(), long_press_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = long_press_instance.clone() {
                    message_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    message_broadcaster.broadcast_string(&topic, &payload_str);
                }
                gesture.set_state(EventSequenceState::Claimed);
            }
        });
        let button_weak = button.downgrade();
        long_press_gesture.connect_cancelled(move |_gesture| {
            if let Some(btn) = button_weak.upgrade() {
                btn.remove_css_class("longpress-active");
            }
        });
        button.add_controller(long_press_gesture);

        button.clone().upcast::<Widget>()
    }
}
