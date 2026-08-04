use crate::config::ButtonConfig;
use crate::personalization::PersonalizationOverride;
use gtk4::Align;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureDrag;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::*;
use regex::Regex;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::rc::Rc;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

pub struct ButtonWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ButtonConfig,
    pub(crate) label_widget: std::cell::RefCell<Option<Label>>,
    pub(crate) info_label_widget: std::cell::RefCell<Option<Label>>,
    pub(crate) icon_widget: std::cell::RefCell<Option<Image>>,
    pub(crate) button_widget: std::cell::RefCell<Option<Button>>,
    pub(crate) internal_state: std::rc::Rc<std::cell::RefCell<Option<serde_json::Value>>>,
    pub(crate) personalization: std::rc::Rc<std::cell::RefCell<PersonalizationOverride>>,
}

impl ButtonWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        // You can't create a button here, because the GTK thread is not running yet.
        trace!("ButtonWidget plugin config: {config:?}");
        let meta = PluginMeta::try_from(&config)?;
        trace!("ButtonWidget meta: {meta:?}");
        let config = ButtonConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        trace!("ButtonWidget button config: {config:?}");
        let widget = Self {
            meta,
            core_context,
            config: config.clone(),
            label_widget: std::cell::RefCell::new(None),
            info_label_widget: std::cell::RefCell::new(None),
            icon_widget: std::cell::RefCell::new(None),
            button_widget: std::cell::RefCell::new(None),
            internal_state: std::rc::Rc::new(std::cell::RefCell::new(None)),
            personalization: std::rc::Rc::new(std::cell::RefCell::new(PersonalizationOverride::default())),
        };

        // Send initial one-shot request
        if let (Some(topic), Some(payload)) = (&config.actions.init.topic, &config.actions.init.payload) {
            let broadcaster = widget.get_broadcaster();
            let payload_str = payload.to_string();
            if let Some(instance) = &config.actions.init.instance {
                broadcaster.broadcast_string_to_instance(instance, topic, &payload_str);
            } else {
                broadcaster.broadcast_string(topic, &payload_str);
            }
            debug!("ButtonWidget sent init one-shot to topic {}", topic);
        }

        widget.register_mcp_capabilities();
        widget.request_personalization_status();

        Ok(widget)
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

    fn update_internal_state(&self, payload: &str) {
        let json: serde_json::Value = match serde_json::from_str(payload) {
            Ok(value) => value,
            Err(e) => {
                debug!("ButtonWidget failed to parse internal state JSON: {}", e);
                return;
            }
        };

        // If the payload is an HttpResponseMessage, extract and parse the body field
        let state = if let Some(body) = json.get("body").and_then(|v| v.as_str()) {
            match serde_json::from_str(body) {
                Ok(inner) => inner,
                Err(e) => {
                    debug!("ButtonWidget failed to parse body as JSON: {}", e);
                    return;
                }
            }
        } else {
            json
        };

        trace!("ButtonWidget updating internal state: {}", state);
        *self.internal_state.borrow_mut() = Some(state.clone());

        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);

        let config = self.config.clone();
        let icon_weak = self.icon_widget.borrow().as_ref().map(|icon| icon.downgrade());
        let label_weak = self.label_widget.borrow().as_ref().map(|label| label.downgrade());
        let button_weak = self.button_widget.borrow().as_ref().map(|btn| btn.downgrade());

        glib::MainContext::default().spawn_local(async move {
            if let Some(icon_expr) = &config.state_icon {
                let resolved = resolve_state_expression(icon_expr, &state);
                if let Some(icon) = icon_weak.and_then(|weak| weak.upgrade()) {
                    update_icon(&icon, &resolved, &config);
                }
            }

            if let Some(css_class) = &config.state_css_class {
                let is_active = is_state_truthy(&state, &config.state_icon);
                if let Some(button) = button_weak.and_then(|weak| weak.upgrade()) {
                    if is_active {
                        button.add_css_class(css_class);
                    } else {
                        button.remove_css_class(css_class);
                    }
                }
            }

            if let Some(label_expr) = &config.state_label {
                let text = resolve_state_format(label_expr, &state);
                if let Some(label) = label_weak.and_then(|weak| weak.upgrade()) {
                    label.set_text(&text);
                }
            }
        });
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }
}

impl DefaultFallback for ButtonWidget {
    fn default_fallback(&self, _kind: &ActionKind, _broadcaster: &MessageBroadcasterInner) {}
}

/// Resolve a state expression against the internal state JSON.
/// Supports: "literal", "{field}", "{field?true_value:false_value}"
fn resolve_state_expression(expr: &str, state: &serde_json::Value) -> String {
    if !expr.starts_with('{') {
        return expr.to_string();
    }

    let inner = &expr[1..expr.len().saturating_sub(1)];

    if let Some((condition, values)) = inner.split_once('?') {
        let (true_val, false_val) = values.split_once(':').unwrap_or((values, ""));
        let field_value = &state[condition];
        if is_truthy(field_value) {
            true_val.to_string()
        } else {
            false_val.to_string()
        }
    } else {
        state[inner].as_str().unwrap_or(expr).to_string()
    }
}

/// Check if a JSON value is truthy
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        serde_json::Value::Null => false,
        _ => true,
    }
}

/// Check if the state JSON is truthy for state_css_class purposes.
/// If state_icon contains a condition like "{ison?...}", evaluates that field.
/// Otherwise evaluates the root state value.
fn is_state_truthy(state: &serde_json::Value, state_icon: &Option<String>) -> bool {
    if let Some(icon_expr) = state_icon {
        if icon_expr.starts_with('{') {
            let inner = &icon_expr[1..icon_expr.len().saturating_sub(1)];
            if let Some((condition, _)) = inner.split_once('?') {
                return is_truthy(&state[condition]);
            }
        }
    }
    is_truthy(state)
}

/// Resolve template expressions in a JSON payload against the internal state.
/// Supports {field}, {field+N}, {field-N} in string values (clamped 0-100 for arithmetic).
fn resolve_payload_template(payload: &serde_json::Value, state: &serde_json::Value) -> serde_json::Value {
    match payload {
        serde_json::Value::String(s) => serde_json::Value::String(resolve_template_string(s, state)),
        serde_json::Value::Object(map) => {
            let resolved: serde_json::Map<String, serde_json::Value> = map.iter().map(|(k, v)| (k.clone(), resolve_payload_template(v, state))).collect();
            serde_json::Value::Object(resolved)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(|v| resolve_payload_template(v, state)).collect()),
        other => other.clone(),
    }
}

/// Resolve template expressions in a single string against the internal state.
/// Supports: {field}, {field+N}, {field-N} (arithmetic clamped to 0-100).
fn resolve_template_string(s: &str, state: &serde_json::Value) -> String {
    let re = Regex::new(r"\{(\w+)(?:([+-])(\d+))?\}").expect("invalid regex");
    re.replace_all(s, |caps: &regex::Captures| {
        let key = &caps[1];
        let value = &state[key];
        if let Some(num) = value.as_f64() {
            if let Some(op) = caps.get(2) {
                let step: f64 = caps[3].parse().unwrap_or(0.0);
                let result = if op.as_str() == "+" { num + step } else { num - step };
                format!("{:.0}", result.clamp(0.0, 100.0))
            } else {
                format!("{:.0}", num)
            }
        } else if let Some(str_val) = value.as_str() {
            str_val.to_string()
        } else {
            value.to_string()
        }
    })
    .to_string()
}

/// Resolve a format string against the internal state JSON.
/// Uses the same {key} replacement syntax as label_format.
fn resolve_state_format(format: &str, state: &serde_json::Value) -> String {
    let mut result = format.to_string();
    if let Some(object) = state.as_object() {
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
}

/// Update an icon widget with a new icon name, handling both Nerd Font and standard icons.
fn update_icon(icon: &Image, icon_name: &str, config: &ButtonConfig) {
    if icon_name.starts_with("nf-") {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            icon.set_icon_name(Some(&gtk_icon_name));
        }
    } else {
        icon.set_icon_name(Some(icon_name));
    }
    icon.set_pixel_size(config.icon_config.icon_size());
    if let Some(color) = config.icon_config.icon_color() {
        apply_icon_color(icon, color);
    }
}

impl AcceptTopic<FfiEnvelope> for ButtonWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        if let Some(label_topic) = &self.config.label_topic {
            if topic == label_topic {
                return true;
            }
        }
        if let Some(state_topic) = &self.config.state_topic {
            if topic == state_topic {
                return true;
            }
        }
        topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl MessageHandler<String> for ButtonWidget {
    fn handle_message(&self, message: String, _sender_id: &str) {
        self.update_label_from_message(&message);
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for ButtonWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("button widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            time_format: Some(status.time_format),
            date_format: Some(status.date_format),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
    }
}

impl MessageBroadcaster for ButtonWidget {}

impl WidgetPlugin for ButtonWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();

            if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                return;
            }

            if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                return;
            }

            let type_id = smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String");
            if envelope.type_id == type_id && !envelope.payload.is_null() {
                let payload = &*(envelope.payload as *const String);

                if let Some(label_topic) = &self.config.label_topic {
                    if topic == *label_topic {
                        self.update_label_from_message(payload);
                    }
                }

                if let Some(state_topic) = &self.config.state_topic {
                    if topic == *state_topic {
                        self.update_internal_state(payload);
                    }
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
            .spacing(self.config.layout.spacing_or_default())
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["menu_button_inner"])
            .build();

        let needs_icon_ref = self.config.state_icon.is_some();
        if let Some(icon_name) = &self.config.icon {
            if icon_name.starts_with("nf-") {
                if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
                    let icon = Image::from_icon_name(&gtk_icon_name);
                    icon.set_pixel_size(self.config.icon_config.icon_size());
                    icon.add_css_class("nerd-icon");
                    for class in &self.config.layout.css_classes {
                        icon.add_css_class(class);
                    }
                    if let Some(color) = self.config.icon_config.icon_color() {
                        apply_icon_color(&icon, color);
                    }
                    button_box.append(&icon);
                    if needs_icon_ref {
                        *self.icon_widget.borrow_mut() = Some(icon);
                    }
                }
            } else {
                let icon = Image::from_icon_name(icon_name);
                icon.set_pixel_size(self.config.icon_config.icon_size());
                icon.add_css_class("nerd-icon");
                if let Some(color) = self.config.icon_config.icon_color() {
                    apply_icon_color(&icon, color);
                }
                button_box.append(&icon);
                if needs_icon_ref {
                    *self.icon_widget.borrow_mut() = Some(icon);
                }
            }
        } else if needs_icon_ref {
            let icon = Image::new();
            icon.set_pixel_size(self.config.icon_config.icon_size());
            if let Some(color) = self.config.icon_config.icon_color() {
                apply_icon_color(&icon, color);
            }
            button_box.append(&icon);
            *self.icon_widget.borrow_mut() = Some(icon);
        }

        let needs_label_ref = self.config.state_label.is_some();
        let show_main_label = !self.config.icon_config.icon_only() || self.config.icon.is_none() || needs_label_ref;

        let label_text = if show_main_label {
            if self.config.label_topic.is_some() {
                self.config.label_fallback.clone().unwrap_or_else(|| self.config.main_text.clone())
            } else {
                self.config.main_text.clone()
            }
        } else {
            String::new()
        };
        let label = Label::new(Some(&label_text));
        if show_main_label {
            label.add_css_class("widget-main-text");
        }
        label.set_height_request(20);
        apply_text_color(&label, self.config.text_colors.main_text_color());
        button_box.append(&label);
        *self.label_widget.borrow_mut() = Some(label);

        let show_info_label = !self.config.icon_config.icon_only() || self.config.icon.is_none();
        let info_text = if show_info_label { self.config.info_text.clone() } else { String::new() };
        let info_label = Label::new(Some(&info_text));
        if show_info_label {
            info_label.add_css_class("widget-info-text");
        }
        info_label.set_height_request(16);
        apply_text_color(&info_label, self.config.text_colors.info_text_color());
        button_box.append(&info_label);
        *self.info_label_widget.borrow_mut() = Some(info_label);

        let spacer = Label::new(Some(""));
        spacer.set_height_request(16);
        button_box.append(&spacer);

        let mut css_classes = vec!["scroll-item", "menu-button"];
        css_classes.extend(self.config.layout.css_classes.iter().map(String::as_str));
        let button = Button::builder()
            .css_classes(css_classes.as_slice())
            .width_request(self.config.dimensions.width_or_default())
            .child(&button_box)
            .build();

        if let Some(tooltip) = &self.config.tooltip {
            button.set_tooltip_text(Some(tooltip));
        }

        if self.config.state_css_class.is_some() {
            *self.button_widget.borrow_mut() = Some(button.clone());
        }

        let broadcaster = self.get_broadcaster();
        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            label_widget: std::cell::RefCell::new(None),
            info_label_widget: std::cell::RefCell::new(None),
            icon_widget: std::cell::RefCell::new(None),
            button_widget: std::cell::RefCell::new(None),
            internal_state: self.internal_state.clone(),
            personalization: self.personalization.clone(),
        });
        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        widget_self.attach_gesture_handlers(
            &button_widget,
            &widget_self.config.actions,
            &broadcaster,
            &GestureHandlersConfiguration {
                swipe_threshold: 30.0,
                drag_enabled: false,
                longpress_css_class: Some("longpress-active".to_string()),
                group_gestures: false,
                ..Default::default()
            },
        );

        let swipe_up_topic = self.config.actions.swipe_up.topic.clone();
        let swipe_up_payload = self.config.actions.swipe_up.payload.clone();
        let swipe_up_instance = self.config.actions.swipe_up.instance.clone();
        let swipe_down_topic = self.config.actions.swipe_down.topic.clone();
        let swipe_down_payload = self.config.actions.swipe_down.payload.clone();
        let swipe_down_instance = self.config.actions.swipe_down.instance.clone();
        let has_swipe = swipe_up_topic.is_some() || swipe_down_topic.is_some();
        let internal_state = self.internal_state.clone();
        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let message_broadcaster = self.get_broadcaster();
        drag_gesture.connect_drag_end(move |gesture, _offset_x, offset_y| {
            const SWIPE_THRESHOLD: f64 = 30.0;
            if offset_y.abs() < SWIPE_THRESHOLD {
                return;
            }
            let state = internal_state.borrow().clone();
            if offset_y < 0.0 {
                if let (Some(topic), Some(payload)) = (swipe_up_topic.clone(), swipe_up_payload.clone()) {
                    let resolved = if let Some(ref s) = state {
                        debug!("ButtonWidget swipe-up resolving template against state: {}", s);
                        resolve_payload_template(&payload, s)
                    } else {
                        debug!("ButtonWidget swipe-up: no internal state, sending raw payload");
                        payload
                    };
                    let payload_str = resolved.to_string();
                    if let Some(instance) = swipe_up_instance.clone() {
                        message_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                    } else {
                        message_broadcaster.broadcast_string(&topic, &payload_str);
                    }
                    gesture.set_state(EventSequenceState::Claimed);
                }
            } else if let (Some(topic), Some(payload)) = (swipe_down_topic.clone(), swipe_down_payload.clone()) {
                let resolved = if let Some(ref s) = state {
                    resolve_payload_template(&payload, s)
                } else {
                    payload
                };
                let payload_str = resolved.to_string();
                if let Some(instance) = swipe_down_instance.clone() {
                    message_broadcaster.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    message_broadcaster.broadcast_string(&topic, &payload_str);
                }
                gesture.set_state(EventSequenceState::Claimed);
            }
        });
        if has_swipe {
            button_widget.add_controller(drag_gesture);
        }

        button_widget
    }
}

impl smearor_swipe_launcher_plugin_api::WebRenderer for ButtonWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let config = &self.config;

        let icon_html = if let Some(icon) = &config.icon {
            let icon_class = if icon.starts_with("nf-") {
                format!("nerd-icon nerd-{}", icon)
            } else {
                format!("icon icon-{}", icon)
            };
            let color_style = if let Some(color) = config.icon_config.icon_color() {
                format!(
                    " color: rgba({}, {}, {}, {});",
                    (color.r * 255.0).round() as u8,
                    (color.g * 255.0).round() as u8,
                    (color.b * 255.0).round() as u8,
                    color.a
                )
            } else {
                String::new()
            };
            format!(
                r#"<span class="button-icon {}" style="font-size: {}px;{}"></span>"#,
                icon_class,
                config.icon_config.icon_size(),
                color_style
            )
        } else {
            String::new()
        };

        let label_html = if !config.icon_config.icon_only() || config.icon.is_none() {
            let label_text = if let Some(fallback) = &config.label_fallback {
                fallback.clone()
            } else {
                config.main_text.clone()
            };
            let info_html = if !config.info_text.is_empty() {
                format!(r#"<span class="widget-info-text">{}</span>"#, html_escape(&config.info_text))
            } else {
                String::new()
            };
            format!(r#"<span class="widget-main-text">{}</span>{}"#, html_escape(&label_text), info_html)
        } else {
            String::new()
        };

        let state = self.internal_state.borrow();
        let active_class = if let Some(state_css_class) = &config.state_css_class {
            let is_active = state.as_ref().map(|s| is_state_truthy(s, &config.state_icon)).unwrap_or(config.active);
            if is_active { format!(" {}", state_css_class) } else { String::new() }
        } else if config.active {
            " active".to_string()
        } else {
            String::new()
        };

        let css_classes = if config.layout.css_classes.is_empty() {
            String::new()
        } else {
            format!(" {}", config.layout.css_classes.join(" "))
        };

        let disabled_attr = if config.enabled { String::new() } else { " disabled".to_string() };

        let action_source_attr = if config.actions.click.topic.is_some() || config.actions.longpress.topic.is_some() {
            r#" data-action-source="true""#.to_string()
        } else {
            String::new()
        };

        let click_action_attr = if config.actions.click.topic.is_some() {
            r#" data-click-action="click""#.to_string()
        } else {
            String::new()
        };

        let longpress_action_attr = if config.actions.longpress.topic.is_some() {
            r#" data-longpress-action="longpress""#.to_string()
        } else {
            String::new()
        };

        let tooltip_attr = config.tooltip.as_ref().map(|t| format!(r#" title="{}""#, html_escape(t))).unwrap_or_default();

        format!(
            r#"<button class="web-button{}{}" data-plugin-id="{}" data-instance-id="{}"{}{}{}{}{}><div class="button-inner" style="gap: {}px;">{}{}</div></button>"#,
            active_class,
            css_classes,
            html_escape(plugin_id),
            html_escape(instance_id),
            action_source_attr,
            click_action_attr,
            longpress_action_attr,
            tooltip_attr,
            disabled_attr,
            config.layout.spacing_or_default(),
            icon_html,
            label_html
        )
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
