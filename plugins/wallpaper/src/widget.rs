use crate::config::WallpaperWidgetConfig;
use crate::preview::update_preview;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::GestureSwipe;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Picture;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
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
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_wallpaper_model::WallpaperCommandMessage;
use smearor_wallpaper_model::WallpaperStatusMessage;
use smearor_wallpaper_model::WallpaperThemeInfo;
use smearor_wallpaper_model::wallpaper_type_icon;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::trace;

pub struct WallpaperWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WallpaperWidgetConfig,
    pub preview_image: Rc<RefCell<Option<Picture>>>,
    pub theme_label: Rc<RefCell<Option<Label>>>,
    pub status_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<WallpaperStatusMessage>>>,
}

impl WallpaperWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: WallpaperWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget = WallpaperWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            preview_image: Rc::new(RefCell::new(None)),
            theme_label: Rc::new(RefCell::new(None)),
            status_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
        };
        Ok(widget)
    }

    fn update_ui(&self, status: &WallpaperStatusMessage) {
        let preview_image = self.preview_image.clone();
        let theme_label = self.theme_label.clone();
        let status_label = self.status_label.clone();
        let show_theme_name = self.config.show_theme_name;
        let show_type_icon = self.config.show_type_icon;
        let show_status_indicator = self.config.show_status_indicator;
        let fallback_icon = self.config.fallback_icon.clone();
        let status = status.clone();

        MainContext::default().spawn_local(async move {
            let theme_info: Option<WallpaperThemeInfo> = status.themes.get(status.selected_theme_index).cloned();

            let (preview_path, theme_name, type_icon, status_text) = match &theme_info {
                Some(theme) => {
                    let icon = wallpaper_type_icon(&theme.wallpaper_type);
                    let name: String = theme.name.to_string();
                    let preview: String = theme.preview_image_path.to_string();
                    let is_running = status.is_running();
                    let current: String = status.current_theme.as_ref().map(|t| t.to_string()).unwrap_or_default();
                    let st = if is_running && current == name {
                        "\u{f03a7}".to_string()
                    } else if is_running {
                        format!("\u{f03a7} {}", current)
                    } else {
                        "\u{f0156}".to_string()
                    };
                    (preview, name, icon.to_string(), st)
                }
                None => (String::new(), "No theme".to_string(), "\u{f1c5}".to_string(), "N/A".to_string()),
            };

            update_preview(&preview_image, &theme_label, &preview_path, &fallback_icon);

            if show_theme_name && let Some(ref label) = *theme_label.borrow() {
                if show_type_icon {
                    label.set_text(&format!("{type_icon}  {theme_name}"));
                } else {
                    label.set_text(&theme_name);
                }
            }
            if show_status_indicator && let Some(ref label) = *status_label.borrow() {
                label.set_text(&status_text);
            }
        });
    }

    fn select_next_theme(&self) {
        let latest_status = self.latest_status.clone();
        let broadcaster = self.get_broadcaster();

        MainContext::default().spawn_local(async move {
            let status = latest_status.borrow().clone();
            if let Some(status) = status {
                if status.themes.is_empty() {
                    return;
                }
                let next_index = (status.selected_theme_index + 1) % status.themes.len();
                if let Some(theme) = status.themes.get(next_index) {
                    let name: String = theme.name.to_string();
                    let command = WallpaperCommandMessage::select_theme(&name);
                    broadcaster.broadcast_message_to_topic(command);
                }
            }
        });
    }

    fn select_prev_theme(&self) {
        let latest_status = self.latest_status.clone();
        let broadcaster = self.get_broadcaster();

        MainContext::default().spawn_local(async move {
            let status = latest_status.borrow().clone();
            if let Some(status) = status {
                if status.themes.is_empty() {
                    return;
                }
                let prev_index = if status.selected_theme_index == 0 {
                    status.themes.len() - 1
                } else {
                    status.selected_theme_index - 1
                };
                if let Some(theme) = status.themes.get(prev_index) {
                    let name: String = theme.name.to_string();
                    let command = WallpaperCommandMessage::select_theme(&name);
                    broadcaster.broadcast_message_to_topic(command);
                }
            }
        });
    }
}

impl MessageHandler<WallpaperStatusMessage> for WallpaperWidget {
    fn handle_message(&self, message: WallpaperStatusMessage, _sender_id: &str) {
        trace!("wallpaper widget: status update current_theme={:?}", message.current_theme);
        *self.latest_status.borrow_mut() = Some(message.clone());
        self.update_ui(&message);
    }
}

impl MessageBroadcaster for WallpaperWidget {}

impl PluginMetaGetter for WallpaperWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WallpaperWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for WallpaperWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                trace!("wallpaper widget: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == WallpaperStatusMessage::TYPE_ID {
                    MessageHandler::<WallpaperStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for WallpaperWidget {
    fn build_widget(&mut self) -> Widget {
        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .css_classes(["wallpaper-widget"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        if let Some(width) = self.config.width {
            outer_box.set_width_request(width);
        }
        if let Some(height) = self.config.height {
            outer_box.set_height_request(height);
        }

        let picture = Picture::builder()
            .css_classes(["wallpaper-preview"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        if let Some(pw) = self.config.preview_width {
            picture.set_size_request(pw, -1);
        }
        if let Some(ph) = self.config.preview_height {
            picture.set_size_request(-1, ph);
        }
        outer_box.append(&picture);

        let theme_label = Label::builder()
            .css_classes(["wallpaper-theme-name"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(12)
            .build();
        if self.config.show_theme_name {
            theme_label.set_text("Loading...");
            outer_box.append(&theme_label);
        }

        let status_label = Label::builder().css_classes(["wallpaper-status"]).build();
        if self.config.show_status_indicator {
            status_label.set_text("N/A");
            outer_box.append(&status_label);
        }

        *self.preview_image.borrow_mut() = Some(picture);
        *self.theme_label.borrow_mut() = Some(theme_label);
        *self.status_label.borrow_mut() = Some(status_label);

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            preview_image: self.preview_image.clone(),
            theme_label: self.theme_label.clone(),
            status_label: self.status_label.clone(),
            latest_status: self.latest_status.clone(),
        });

        // Click gesture — start selected wallpaper
        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_for_click = self.get_broadcaster();
        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let click_instance = self.config.click_instance.clone();
        click_gesture.connect_released(move |gesture, _n_press, _x, _y| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            let command = WallpaperCommandMessage::start_selected();
            broadcaster_for_click.broadcast_message_to_topic(command);
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                let instance = click_instance.as_deref().unwrap_or("");
                broadcaster_for_click.broadcast_string_to_instance(instance, &topic, &payload_str);
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        outer_box.add_controller(click_gesture);

        // Swipe gesture — cycle themes up/down
        let swipe_gesture = GestureSwipe::builder().propagation_phase(PropagationPhase::Capture).build();
        let widget_for_swipe = widget_self.clone();
        swipe_gesture.connect_swipe(move |gesture, velocity_x, velocity_y| {
            if velocity_y.abs() <= velocity_x.abs() {
                return;
            }
            gesture.set_state(EventSequenceState::Claimed);
            if velocity_y < 0.0 {
                widget_for_swipe.select_prev_theme();
            } else {
                widget_for_swipe.select_next_theme();
            }
        });
        outer_box.add_controller(swipe_gesture);

        // Long press gesture — stop current wallpaper
        let longpress_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_for_longpress = self.get_broadcaster();
        let longpress_topic = self.config.longpress_topic.clone();
        let longpress_payload = self.config.longpress_payload.clone();
        let longpress_instance = self.config.longpress_instance.clone();
        longpress_gesture.connect_pressed(move |gesture, _x, _y| {
            let command = WallpaperCommandMessage::stop_current();
            broadcaster_for_longpress.broadcast_message_to_topic(command);
            if let (Some(topic), Some(payload)) = (longpress_topic.clone(), longpress_payload.clone()) {
                let payload_str = payload.to_string();
                let instance = longpress_instance.as_deref().unwrap_or("");
                broadcaster_for_longpress.broadcast_string_to_instance(instance, &topic, &payload_str);
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        outer_box.add_controller(longpress_gesture);

        outer_box.upcast::<Widget>()
    }
}
