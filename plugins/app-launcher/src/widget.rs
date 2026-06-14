use crate::config::AppLauncherConfig;
use adw::gdk::pango::EllipsizeMode;
use adw::prelude::ObjectExt;
use freedesktop_entry_parser::Entry;
use gtk4::Align;
use gtk4::Button;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::WidgetExt;
use serde_json::Value;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::DesktopFileStatus;
use smearor_app_launcher_model::DesktopFileStatusMessage;
use smearor_app_launcher_model::TOPIC_COMMAND;
use smearor_app_launcher_model::TOPIC_STATUS;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::PluginMetaRaw;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::info;

pub struct AppLauncherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AppLauncherConfig,
    pub desktop_entry: Entry,
    pub app_name: String,
    pub icon_name: String,
    pub led_indicator: Arc<RwLock<Option<gtk4::Box>>>,
}

impl AppLauncherWidget {
    pub fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!("AppLauncherWidget config: {config:?}");
        let meta_raw = PluginMetaRaw::try_from(&config)?;
        let config = AppLauncherConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let mut app_name = meta_raw.display_name.to_string();
        let mut icon_name = meta_raw.icon_name.unwrap_or_default().to_string();

        // Parse `.desktop` file
        let desktop_entry = match Entry::parse_file(&config.desktop_file_path) {
            Ok(entry) => entry,
            Err(e) => {
                return Err(PluginConstructionErrorWrapper::new(
                    PluginConstructionError::Custom,
                    format!("AppLauncher Service: Failed to parse desktop file {}: {e}", config.desktop_file_path).into(),
                ));
            }
        };
        if let Some(name) = desktop_entry.get("Desktop Entry", "Name").and_then(|names| names.first()) {
            app_name = name.clone();
        }
        match desktop_entry.get("Desktop Entry", "Icon").and_then(|names| names.first()) {
            Some(icon) => icon_name = icon.clone(),
            None => {
                // fallback
                if icon_name.is_empty() {
                    icon_name = "system-run".to_string();
                }
            }
        }

        Ok(AppLauncherWidget {
            meta: PluginMeta::new(meta_raw.id, app_name.clone(), Some(icon_name.clone())),
            config,
            desktop_entry,
            app_name,
            icon_name,
            core_context,
            led_indicator: Arc::new(RwLock::new(None)),
        })
    }
}

impl MessageHandler<FfiEnvelopePayload<DesktopFileStatusMessage>> for AppLauncherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<DesktopFileStatusMessage>) {
        if message.desktop_file != self.config.desktop_file_path {
            return;
        }
        info!("AppLauncher Widget {} status updated for {}: {:?}", self.meta.id, message.desktop_file, message.status);
        if let Ok(guard) = self.led_indicator.read() {
            if let Some(led) = guard.as_ref() {
                match message.status {
                    DesktopFileStatus::Running => {
                        led.remove_css_class("led-unlit");
                        led.add_css_class("led-lit");
                    }
                    DesktopFileStatus::Stopped => {
                        led.remove_css_class("led-lit");
                        led.add_css_class("led-unlit");
                    }
                }
            }
        }
    }

    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS
    }
}

impl MessageBroadcaster<DesktopFileCommandMessage> for AppLauncherWidget {}

impl MessageBroadcaster<Value> for AppLauncherWidget {}

impl PluginMetaGetter for AppLauncherWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for AppLauncherWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetBuilder for AppLauncherWidget {
    fn build_widget(&mut self) -> Widget {
        let _ = adw::init();

        let main_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(30)
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["app-launcher-tile"])
            .build();

        // Render Icon
        let image = Image::from_icon_name(&self.icon_name);
        image.set_pixel_size(self.config.icon_size);
        main_box.append(&image);

        if !self.config.icon_only {
            // Render Name
            let label = Label::builder()
                .label(&self.app_name)
                .ellipsize(EllipsizeMode::End)
                .max_width_chars(12)
                .css_classes(["app-launcher-label"])
                .build();
            main_box.append(&label);
        }

        // LED Indicator Box to show if application is running
        let led_box = gtk4::Box::builder()
            .width_request(8)
            .height_request(8)
            .halign(Align::Center)
            .css_classes(["app-launcher-led", "led-unlit"])
            .build();
        // main_box.append(&led_box);

        *self.led_indicator.write().unwrap() = Some(led_box);

        let button = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.config.width)
            .child(&main_box)
            .build();

        // Gestures - Click to Launch
        let longpress_gesture = GestureLongPress::builder()
            .propagation_phase(PropagationPhase::Capture)
            // Extra long because of the parent scroll window widget has a drag gesture
            .delay_factor(2.0)
            .build();

        let click_gesture = GestureClick::builder().propagation_phase(PropagationPhase::Capture).build();
        longpress_gesture.group_with(&click_gesture);

        click_gesture.connect_pressed(move |_, _, _, _| {});

        let desktop_file_inner = self.config.desktop_file_path.clone();
        let message_broadcaster_desktop_file_command = MessageBroadcaster::<DesktopFileCommandMessage>::get_broadcaster(self);
        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let message_broadcaster_generic = MessageBroadcaster::<Value>::get_broadcaster(self);
        click_gesture.connect_released(move |gesture, n_clicks, _, _| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            info!("Click released {n_clicks}");
            message_broadcaster_desktop_file_command.broadcast_message(TOPIC_COMMAND, DesktopFileCommandMessage::exec(&desktop_file_inner));
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                message_broadcaster_generic.broadcast_message(&topic, &payload);
            }
            gesture.set_state(EventSequenceState::Claimed);
        });

        let button_inner = button.downgrade();
        longpress_gesture.connect_begin(move |_, _| {
            if let Some(button) = button_inner.upgrade() {
                button.add_css_class("menu-button-longpress");
            }
        });
        let desktop_file_inner = self.config.desktop_file_path.clone();
        let message_broadcaster_desktop_file_command = MessageBroadcaster::<DesktopFileCommandMessage>::get_broadcaster(self);
        let long_press_topic = self.config.longpress_topic.clone();
        let long_press_payload = self.config.longpress_payload.clone();
        let message_broadcaster_generic = MessageBroadcaster::<Value>::get_broadcaster(self);
        longpress_gesture.connect_pressed(move |gesture, _n_clicks, _| {
            message_broadcaster_desktop_file_command.broadcast_message(TOPIC_COMMAND, DesktopFileCommandMessage::terminate(&desktop_file_inner));
            if let (Some(topic), Some(payload)) = (long_press_topic.clone(), long_press_payload.clone()) {
                message_broadcaster_generic.broadcast_message(&topic, &payload);
                gesture.set_state(EventSequenceState::Claimed);
            }
            gesture.set_state(EventSequenceState::Claimed);
        });

        let button_inner = button.downgrade();
        longpress_gesture.connect_end(move |_, _| {
            if let Some(button) = button_inner.upgrade() {
                button.remove_css_class("menu-button-longpress");
            }
        });
        longpress_gesture.connect_cancelled(move |gesture| {
            gesture.set_state(EventSequenceState::None);
        });

        button.add_controller(click_gesture);
        button.add_controller(longpress_gesture);

        button.clone().upcast::<Widget>()

        // main_box.upcast::<Widget>()
    }
}
