use crate::config::AppLauncherConfig;
use crate::desktop_entry::DesktopEntry;
use adw::prelude::ObjectExt;
use gtk4::Align;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::WidgetExt;
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
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::PluginMetaRaw;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;

pub struct AppLauncherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AppLauncherConfig,
    pub app_name: String,
    pub icon_name: String,
    pub led_indicator: Arc<RwLock<Option<gtk4::Box>>>,
}

impl AppLauncherWidget {
    pub fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        debug!("AppLauncherWidget config: {config:?}");
        let meta_raw = PluginMetaRaw::try_from(&config)?;
        let config = AppLauncherConfig::parse(&config.config).map_err(|e| PluginConstructionError::FailedToParseWidgetConfig(e.to_string().into()))?;
        let mut app_name = meta_raw.display_name.to_string();
        let mut icon_name = meta_raw.icon_name.unwrap_or_default().to_string();

        // Parse `.desktop` file
        if let Some(entry) = DesktopEntry::parse(&config.desktop_file_path) {
            app_name = entry.name;
            icon_name = entry.icon;
        } else {
            error!("Could not load .desktop file at: {}", config.desktop_file_path);
        }

        if icon_name.is_empty() {
            icon_name = "system-run".to_string(); // fallback
        }

        Ok(AppLauncherWidget {
            meta: PluginMeta::new(meta_raw.id, app_name.clone(), Some(icon_name.clone())),
            config,
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
        let main_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .width_request(100)
            .height_request(100)
            .halign(Align::Center)
            .valign(Align::Center)
            .css_classes(["app-launcher-tile"])
            .build();

        // Render Icon
        let image = gtk4::Image::from_icon_name(&self.icon_name);
        image.set_pixel_size(48);
        main_box.append(&image);

        // Render Name
        let label = Label::builder()
            .label(&self.app_name)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(12)
            .css_classes(["app-launcher-label"])
            .build();
        main_box.append(&label);

        // LED Indicator Box to show if application is running
        let led_box = gtk4::Box::builder()
            .width_request(8)
            .height_request(8)
            .halign(Align::Center)
            .css_classes(["app-launcher-led", "led-unlit"])
            .build();
        main_box.append(&led_box);

        *self.led_indicator.write().unwrap() = Some(led_box);

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
        let message_broadcaster = self.get_broadcaster();
        click_gesture.connect_released(move |gesture, n_clicks, _, _| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            info!("Click released {n_clicks}");
            message_broadcaster.broadcast_message(TOPIC_COMMAND, DesktopFileCommandMessage::exec(&desktop_file_inner));
            gesture.set_state(EventSequenceState::Claimed);
        });

        let main_box_inner = main_box.downgrade();
        longpress_gesture.connect_begin(move |_, _| {
            if let Some(main_box) = main_box_inner.upgrade() {
                main_box.add_css_class("longpress");
            }
        });
        let desktop_file_inner = self.config.desktop_file_path.clone();
        let message_broadcaster = self.get_broadcaster();
        longpress_gesture.connect_pressed(move |gesture, _n_clicks, _| {
            message_broadcaster.broadcast_message(TOPIC_COMMAND, DesktopFileCommandMessage::terminate(&desktop_file_inner));
            gesture.set_state(EventSequenceState::Claimed);
        });

        let main_box_inner = main_box.downgrade();
        longpress_gesture.connect_end(move |_, _| {
            if let Some(main_box) = main_box_inner.upgrade() {
                main_box.remove_css_class("longpress");
            }
        });
        longpress_gesture.connect_cancelled(move |gesture| {
            gesture.set_state(EventSequenceState::None);
        });

        main_box.add_controller(click_gesture);
        main_box.add_controller(longpress_gesture);

        main_box.upcast::<Widget>()
    }
}
