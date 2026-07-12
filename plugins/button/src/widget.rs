use crate::config::ButtonConfig;
use gtk4::Align;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureDrag;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::gio;
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
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use tracing::debug;

pub struct ButtonWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ButtonConfig,
    pub(crate) label_widget: std::cell::RefCell<Option<Label>>,
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
        Ok(Self {
            meta,
            core_context,
            config,
            label_widget: std::cell::RefCell::new(None),
        })
    }

    fn update_label_from_message(&self, payload: &str) {
        let format = self.config.label_format.clone();
        let label_weak = self.label_widget.borrow().as_ref().map(|label| label.downgrade());

        let payload_inner = payload.to_owned();
        glib::MainContext::default().spawn_local(async move {
            if let Some(label) = label_weak.and_then(|weak| weak.upgrade()) {
                let text = if let Some(format) = format {
                    let json: serde_json::Value = match serde_json::from_str(&payload_inner) {
                        Ok(value) => value,
                        Err(_) => {
                            label.set_text(&payload_inner);
                            return;
                        }
                    };
                    let mut result = format;
                    if let Some(object) = json.as_object() {
                        for (key, value) in object {
                            let replacement = match value {
                                serde_json::Value::Number(number) => {
                                    if let Some(integer) = number.as_i64() {
                                        format!("{}", integer)
                                    } else if let Some(float) = number.as_f64() {
                                        format!("{}", float)
                                    } else {
                                        String::new()
                                    }
                                }
                                serde_json::Value::String(string) => string.clone(),
                                _ => value.to_string(),
                            };
                            result = result.replace(&format!("{{{}}}", key), &replacement);
                            if let Some(float) = value.as_f64() {
                                result = result.replace(&format!("{{{}:.0}}", key), &format!("{:.0}", float));
                                result = result.replace(&format!("{{{}:.1}}", key), &format!("{:.1}", float));
                                result = result.replace(&format!("{{{}:.2}}", key), &format!("{:.2}", float));
                            }
                        }
                    }
                    result
                } else {
                    payload_inner.to_string()
                };
                label.set_text(&text);
            }
        });
    }
}

impl AcceptTopic<FfiEnvelope> for ButtonWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        if let Some(label_topic) = &self.config.label_topic {
            return topic == label_topic;
        }
        false
    }
}

impl MessageHandler<String> for ButtonWidget {
    fn handle_message(&self, message: String, _sender_id: &str) {
        self.update_label_from_message(&message);
    }
}

impl MessageBroadcaster for ButtonWidget {}

impl Plugin for ButtonWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            if let Some(label_topic) = &self.config.label_topic {
                if envelope.topic.to_string() == *label_topic
                    && envelope.type_id == smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String")
                    && !envelope.payload.is_null()
                {
                    let payload = &*(envelope.payload as *const String);
                    self.update_label_from_message(payload);
                }
            }
        }
    }
}

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
            .spacing(self.config.spacing)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["menu_button_inner"])
            .build();

        if let Some(icon_name) = &self.config.icon {
            if icon_name.starts_with("nf-") {
                if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
                    let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
                    if gio::resources_lookup_data(&resource_path, gio::ResourceLookupFlags::NONE).is_ok() {
                        let icon = Image::from_resource(&resource_path);
                        icon.set_pixel_size(self.config.icon_size);
                        for class in &self.config.css_classes {
                            icon.add_css_class(class);
                        }
                        button_box.append(&icon);
                    } else {
                        debug!("GResource not found for {}", resource_path);
                    }
                }
            } else {
                let icon = Image::from_icon_name(icon_name);
                icon.set_pixel_size(self.config.icon_size);
                button_box.append(&icon);
            }
        }

        if !self.config.icon_only || self.config.icon.is_none() {
            let label_text = if self.config.label_topic.is_some() {
                self.config.label_fallback.clone().unwrap_or_else(|| self.config.text.clone())
            } else {
                self.config.text.clone()
            };
            let label = Label::new(Some(&label_text));
            button_box.append(&label);
            *self.label_widget.borrow_mut() = Some(label);
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

        let swipe_up_topic = self.config.swipe_up_topic.clone();
        let swipe_up_payload = self.config.swipe_up_payload.clone();
        let swipe_up_instance = self.config.swipe_up_instance.clone();
        let swipe_down_topic = self.config.swipe_down_topic.clone();
        let swipe_down_payload = self.config.swipe_down_payload.clone();
        let swipe_down_instance = self.config.swipe_down_instance.clone();
        let has_swipe = swipe_up_topic.is_some() || swipe_down_topic.is_some();
        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let message_broadcaster = self.get_broadcaster();
        drag_gesture.connect_drag_end(move |gesture, _offset_x, offset_y| {
            const SWIPE_THRESHOLD: f64 = 30.0;
            if offset_y.abs() < SWIPE_THRESHOLD {
                return;
            }
            if offset_y < 0.0 {
                if let (Some(topic), Some(payload)) = (swipe_up_topic.clone(), swipe_up_payload.clone()) {
                    let payload_str = payload.to_string();
                    if let Some(instance) = swipe_up_instance.clone() {
                        message_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                    } else {
                        message_broadcaster.broadcast_string(&topic, &payload_str);
                    }
                    gesture.set_state(EventSequenceState::Claimed);
                }
            } else if let (Some(topic), Some(payload)) = (swipe_down_topic.clone(), swipe_down_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = swipe_down_instance.clone() {
                    message_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    message_broadcaster.broadcast_string(&topic, &payload_str);
                }
                gesture.set_state(EventSequenceState::Claimed);
            }
        });
        if has_swipe {
            button.add_controller(drag_gesture);
        }

        button.clone().upcast::<Widget>()
    }
}
