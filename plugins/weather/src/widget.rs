use crate::config::WeatherWidgetConfig;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_weather_model::TOPIC_STATUS;
use smearor_weather_model::WeatherCommandMessage;
use smearor_weather_model::WeatherStatusMessage;
use smearor_weather_model::WeatherView;
use smearor_weather_model::weather_code_description;
use smearor_weather_model::weather_code_icon_day_night;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

pub struct WeatherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WeatherWidgetConfig,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub temp_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<WeatherStatusMessage>>>,
}

impl WeatherWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: WeatherWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget = WeatherWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            icon_label: Rc::new(RefCell::new(None)),
            temp_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            latest_status: Rc::new(RefCell::new(None)),
        };
        widget.register_mcp_capabilities();
        Ok(widget)
    }

    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource = RegisterResourceMessage::new(
            "weather://widget",
            "Weather Widget",
            "Current weather data displayed by the weather widget.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource);

        let tool = RegisterToolMessage::new(
            "weather_widget_refresh",
            "Force the weather widget to request a data refresh.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool);
    }

    fn update_ui(&self, status: &WeatherStatusMessage) {
        let icon_label = self.icon_label.clone();
        let temp_label = self.temp_label.clone();
        let info_label = self.info_label.clone();
        let current_view = self.current_view.clone();
        let views = self.config.views.clone();
        let status = status.clone();

        MainContext::default().spawn_local(async move {
            let view_index = *current_view.borrow();
            let view = views.get(view_index).copied().unwrap_or(WeatherView::Current);

            let (icon_text, temp_text, info_text) = render_view(&status, view);

            if let Some(ref label) = *icon_label.borrow() {
                label.set_text(&icon_text);
            }
            if let Some(ref label) = *temp_label.borrow() {
                label.set_text(&temp_text);
            }
            if let Some(ref label) = *info_label.borrow() {
                label.set_text(&info_text);
            }
        });
    }

    fn cycle_view(&self) {
        let current_view = self.current_view.clone();
        let latest_status = self.latest_status.clone();
        let icon_label = self.icon_label.clone();
        let temp_label = self.temp_label.clone();
        let info_label = self.info_label.clone();
        let views = self.config.views.clone();

        MainContext::default().spawn_local(async move {
            if views.is_empty() {
                return;
            }
            let mut idx = current_view.borrow_mut();
            *idx = (*idx + 1) % views.len();
            let view = views[*idx];
            drop(idx);
            let _ = view;

            if let Some(ref status) = *latest_status.borrow() {
                let (icon_text, temp_text, info_text) = render_view(status, view);
                if let Some(ref label) = *icon_label.borrow() {
                    label.set_text(&icon_text);
                }
                if let Some(ref label) = *temp_label.borrow() {
                    label.set_text(&temp_text);
                }
                if let Some(ref label) = *info_label.borrow() {
                    label.set_text(&info_text);
                }
            }
        });
    }
}

fn render_view(status: &WeatherStatusMessage, view: WeatherView) -> (String, String, String) {
    if status.is_stale {
        let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Stale data".to_string());
        return ("\u{f07b}".to_string(), "--".to_string(), format!("Stale: {error}"));
    }

    if !status.success {
        let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "No data".to_string());
        return ("\u{f07b}".to_string(), "--".to_string(), error);
    }

    match view {
        WeatherView::Current => {
            let temp = status
                .current
                .temperature
                .as_ref()
                .copied()
                .map(|t| format!("{:.0}\u{b0}C", t))
                .unwrap_or_else(|| "--".to_string());
            let code = status.current.weather_code.as_ref().copied().unwrap_or(999);
            let is_day = status.current.is_day.as_ref().copied().unwrap_or(true);
            let icon = weather_code_icon_day_night(code, is_day).to_string();
            let desc = weather_code_description(code).to_string();
            (icon, temp, desc)
        }
        WeatherView::ForecastToday => {
            let max = status
                .daily
                .today
                .temperature_max
                .as_ref()
                .copied()
                .map(|t| format!("{:.0}", t))
                .unwrap_or_else(|| "--".to_string());
            let min = status
                .daily
                .today
                .temperature_min
                .as_ref()
                .copied()
                .map(|t| format!("{:.0}", t))
                .unwrap_or_else(|| "--".to_string());
            let code = status.daily.today.weather_code.as_ref().copied().unwrap_or(999);
            let icon = weather_code_icon_day_night(code, true).to_string();
            (icon, format!("{max}/{min}\u{b0}"), "Today".to_string())
        }
        WeatherView::ForecastTomorrow => {
            let max = status
                .daily
                .tomorrow
                .temperature_max
                .as_ref()
                .copied()
                .map(|t| format!("{:.0}", t))
                .unwrap_or_else(|| "--".to_string());
            let min = status
                .daily
                .tomorrow
                .temperature_min
                .as_ref()
                .copied()
                .map(|t| format!("{:.0}", t))
                .unwrap_or_else(|| "--".to_string());
            let code = status.daily.tomorrow.weather_code.as_ref().copied().unwrap_or(999);
            let icon = weather_code_icon_day_night(code, true).to_string();
            (icon, format!("{max}/{min}\u{b0}"), "Tomorrow".to_string())
        }
        WeatherView::Wind => {
            let speed = status
                .current
                .wind_speed
                .as_ref()
                .copied()
                .map(|s| format!("{:.0} km/h", s))
                .unwrap_or_else(|| "--".to_string());
            let dir = status
                .current
                .wind_direction
                .as_ref()
                .copied()
                .map(|d| format!("{:.0}\u{b0}", d))
                .unwrap_or_else(|| "".to_string());
            ("\u{f050}".to_string(), speed, dir)
        }
        WeatherView::Humidity => {
            let humidity = status
                .current
                .relative_humidity
                .as_ref()
                .copied()
                .map(|h| format!("{:.0}%", h))
                .unwrap_or_else(|| "--".to_string());
            ("\u{f07a}".to_string(), humidity, "Humidity".to_string())
        }
        WeatherView::SunriseSunset => {
            let sunrise = status
                .daily
                .today
                .sunrise
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "--:--".to_string());
            let sunset = status.daily.today.sunset.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "--:--".to_string());
            ("\u{f052}".to_string(), sunrise, format!("Sunset: {sunset}"))
        }
        WeatherView::AirPollution => {
            if let Some(aq) = status.air_quality.as_ref() {
                let aqi = aq
                    .european_aqi
                    .as_ref()
                    .copied()
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "--".to_string());
                let pm = aq.pm2_5.as_ref().copied().map(|v| format!("PM2.5: {:.1}", v)).unwrap_or_else(|| "".to_string());
                ("\u{f074}".to_string(), format!("AQI: {aqi}"), pm)
            } else {
                ("\u{f074}".to_string(), "--".to_string(), "N/A".to_string())
            }
        }
        WeatherView::Pressure => {
            let pressure = status
                .current
                .pressure
                .as_ref()
                .copied()
                .map(|p| format!("{:.0} hPa", p))
                .unwrap_or_else(|| "--".to_string());
            ("\u{f079}".to_string(), pressure, "Pressure".to_string())
        }
        WeatherView::UvIndex => {
            let uv = status
                .current
                .uv_index
                .as_ref()
                .copied()
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "--".to_string());
            ("\u{f056}".to_string(), uv, "UV Index".to_string())
        }
    }
}

impl MessageHandler<WeatherStatusMessage> for WeatherWidget {
    fn handle_message(&self, message: WeatherStatusMessage, _sender_id: &str) {
        *self.latest_status.borrow_mut() = Some(message.clone());
        self.update_ui(&message);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WeatherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("weather widget: InvokeToolMessage name={}", tool_name);
        if tool_name == "weather_widget_refresh" {
            let broadcaster = self.get_broadcaster();
            let command = WeatherCommandMessage::refresh();
            broadcaster.broadcast_message_to_topic(command);
            let response = InvokeToolResponse::success(&message.0.correlation_id, "Refresh triggered");
            broadcaster.broadcast_message_to_topic(response);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WeatherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        if message.0.uri != "weather://widget" {
            return;
        }
        let status = self.latest_status.borrow().clone();
        let response = match status {
            Some(status) => {
                let json = serde_json::json!({
                    "latitude": status.latitude,
                    "longitude": status.longitude,
                    "success": status.success,
                    "is_stale": status.is_stale,
                    "last_updated": status.last_updated.to_string(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            None => InvokeResourceResponse::error(&message.0.correlation_id, "No weather data available"),
        };
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(response);
    }
}

impl AcceptTopic<FfiEnvelope> for WeatherWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == "service.mcp.invoke_tool" || topic == "service.mcp.invoke_resource"
    }
}

impl MessageBroadcaster for WeatherWidget {}

impl PluginMetaGetter for WeatherWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WeatherWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for WeatherWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                debug!("weather widget: on_message topic={} type_id={}", topic, envelope.type_id);
                if envelope.type_id == WeatherStatusMessage::TYPE_ID {
                    MessageHandler::<WeatherStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for WeatherWidget {
    fn build_widget(&mut self) -> Widget {
        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .css_classes(["weather-widget".to_string()])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        if let Some(width) = self.config.width {
            outer_box.set_width_request(width);
        }
        if let Some(height) = self.config.height {
            outer_box.set_height_request(height);
        }

        let icon_label = Label::builder().css_classes(["weather-icon".to_string()]).build();
        let temp_label = Label::builder().css_classes(["weather-temp".to_string()]).build();
        let info_label = Label::builder().css_classes(["weather-info".to_string()]).build();

        icon_label.set_text("\u{f07b}");
        temp_label.set_text("--");
        info_label.set_text("Loading...");

        outer_box.append(&icon_label);
        outer_box.append(&temp_label);
        outer_box.append(&info_label);

        *self.icon_label.borrow_mut() = Some(icon_label.clone());
        *self.temp_label.borrow_mut() = Some(temp_label.clone());
        *self.info_label.borrow_mut() = Some(info_label.clone());

        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let longpress_topic = self.config.longpress_topic.clone();
        let longpress_payload = self.config.longpress_payload.clone();
        let message_broadcaster = self.get_broadcaster();

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            icon_label: self.icon_label.clone(),
            temp_label: self.temp_label.clone(),
            info_label: self.info_label.clone(),
            current_view: self.current_view.clone(),
            latest_status: self.latest_status.clone(),
        });

        let click_gesture = GestureClick::new();
        let widget_for_click = widget_self.clone();
        let broadcaster_for_click = message_broadcaster.clone();
        click_gesture.connect_released(move |_gesture, _n_press, _, _| {
            widget_for_click.cycle_view();
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                broadcaster_for_click.broadcast_string(&topic, &payload_str);
            }
        });
        outer_box.add_controller(click_gesture);

        let longpress_gesture = GestureLongPress::new();
        let broadcaster_for_longpress = message_broadcaster.clone();
        longpress_gesture.connect_pressed(move |_gesture, _, _| {
            if let (Some(topic), Some(payload)) = (longpress_topic.clone(), longpress_payload.clone()) {
                let payload_str = payload.to_string();
                broadcaster_for_longpress.broadcast_string(&topic, &payload_str);
            }
            let command = WeatherCommandMessage::refresh();
            broadcaster_for_longpress.broadcast_message_to_topic(command);
        });
        outer_box.add_controller(longpress_gesture);

        outer_box.upcast::<Widget>()
    }
}
