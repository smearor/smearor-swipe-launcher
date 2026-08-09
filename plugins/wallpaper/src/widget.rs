use crate::config::WallpaperWidgetConfig;
use crate::personalization::PersonalizationOverride;
use crate::preview::update_preview;
use gtk4::Align;
use gtk4::Image;
use gtk4::Label;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
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
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::apply_widget_scaled_css;
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_info_label_scaled;
use smearor_swipe_launcher_plugin_api::build_main_label_scaled;
use smearor_swipe_launcher_plugin_api::build_spacer_scaled;
use smearor_swipe_launcher_plugin_api::sanitize_scale;
use smearor_wallpaper_model::TOPIC_STATUS;
use smearor_wallpaper_model::WallpaperCommandMessage;
use smearor_wallpaper_model::WallpaperStatusMessage;
use smearor_wallpaper_model::WallpaperThemeInfo;
use smearor_wallpaper_model::wallpaper_type_icon;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which view the wallpaper widget is currently displaying.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetView {
    /// Compact view showing current wallpaper thumbnail and name.
    #[default]
    Compact,
    /// Grid view showing a 3×3 wallpaper selection grid.
    Grid,
}

pub struct WallpaperWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WallpaperWidgetConfig,
    pub preview_image: Rc<RefCell<Option<Image>>>,
    pub fallback_image: Rc<RefCell<Option<Image>>>,
    pub theme_label: Rc<RefCell<Option<Label>>>,
    pub status_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<WallpaperStatusMessage>>>,
    pub widget_view: Rc<RefCell<WidgetView>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
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
            fallback_image: Rc::new(RefCell::new(None)),
            theme_label: Rc::new(RefCell::new(None)),
            status_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            widget_view: Rc::new(RefCell::new(WidgetView::Compact)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_initial_status(&self) {
        self.get_broadcaster().broadcast_message_to_topic(WallpaperCommandMessage::refresh());
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Broadcast a WidgetUpdateMessage so headless/Web instances re-render this widget.
    pub fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);
    }

    /// Switches to the Grid (expanded selection) view.
    pub fn expand_view(&self) {
        let mut view = self.widget_view.borrow_mut();
        if *view == WidgetView::Grid {
            return;
        }
        *view = WidgetView::Grid;
        drop(view);
        self.broadcast_widget_update();
    }

    /// Switches to the Compact (thumbnail) view.
    pub fn collapse_view(&self) {
        let mut view = self.widget_view.borrow_mut();
        if *view == WidgetView::Compact {
            return;
        }
        *view = WidgetView::Compact;
        drop(view);
        self.broadcast_widget_update();
    }

    /// Toggles between Compact and Grid views.
    pub fn toggle_view(&self) {
        let current = *self.widget_view.borrow();
        match current {
            WidgetView::Compact => self.expand_view(),
            WidgetView::Grid => self.collapse_view(),
        }
    }

    fn update_ui(&self, status: &WallpaperStatusMessage) {
        let preview_image = self.preview_image.clone();
        let fallback_image = self.fallback_image.clone();
        let theme_label = self.theme_label.clone();
        let status_label = self.status_label.clone();
        let show_theme_name = self.config.show_theme_name;
        let show_type_icon = self.config.show_type_icon;
        let show_status_indicator = self.config.show_status_indicator;
        let fallback_icon = self.config.fallback_icon.clone();
        let status = status.clone();

        MainContext::default().spawn_local(async move {
            let theme_info: Option<WallpaperThemeInfo> = status.themes.get(status.selected_theme_index).cloned();

            let (preview_path, preview_icon, theme_name, type_icon, status_text) = match &theme_info {
                Some(theme) => {
                    let icon = wallpaper_type_icon(&theme.wallpaper_type);
                    let name: String = theme.name.to_string();
                    let preview: String = theme.preview_image_path.to_string();
                    let p_icon: String = theme.preview_icon.to_string();
                    let is_running = status.is_running();
                    let current: String = status.current_theme.as_ref().map(|t| t.to_string()).unwrap_or_default();
                    let st = if is_running && current == name {
                        "\u{f03a7}".to_string()
                    } else if is_running {
                        format!("\u{f03a7} {}", current)
                    } else {
                        "\u{f0156}".to_string()
                    };
                    (preview, p_icon, name, icon.to_string(), st)
                }
                None => (String::new(), String::new(), "No theme".to_string(), "\u{f1c5}".to_string(), "N/A".to_string()),
            };

            let effective_fallback = if preview_icon.is_empty() { &fallback_icon } else { &preview_icon };
            update_preview(&preview_image, &fallback_image, &preview_path, effective_fallback, &fallback_icon);

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
}

impl DefaultFallback for WallpaperWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::DoublePress => {
                broadcaster.broadcast_message_to_topic(WallpaperCommandMessage::start_selected());
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(WallpaperCommandMessage::stop_current());
            }
            ActionKind::SwipeUp | ActionKind::ScrollUp | ActionKind::MiddleClick => {
                self.select_prev_theme();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.select_next_theme();
            }
            ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
            ActionKind::Expand => {
                self.expand_view();
            }
            ActionKind::Collapse => {
                self.collapse_view();
            }
            ActionKind::ToggleView => {
                self.toggle_view();
            }
        }
    }
}

impl WallpaperWidget {
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

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for WallpaperWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("wallpaper widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            color_scheme: Some(status.color_scheme),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
        self.broadcast_widget_update();
    }
}

impl AcceptTopic<FfiEnvelope> for WallpaperWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_PERSONALIZATION_STATUS || topic == TOPIC_MCP_INVOKE_TOOL
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

impl WidgetPlugin for WallpaperWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                trace!("wallpaper widget: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == WallpaperStatusMessage::TYPE_ID {
                    MessageHandler::<WallpaperStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<InvokeToolMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for WallpaperWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let show_labels = !config.icon_config.icon_only();
        let scale = sanitize_scale(config.dimensions.scale.unwrap_or(1.0));

        let content_box = build_content_box(config.layout.spacing_scaled(scale), &["menu_button_inner"]);

        // Line 0: Preview image (Image with Paintable, or fallback icon)
        let preview_image = Image::builder()
            .css_classes(["wallpaper-preview", "nerd-icon"])
            .halign(Align::Center)
            .valign(Align::Center)
            .pixel_size(config.icon_config.icon_size_scaled(scale))
            .build();
        content_box.append(&preview_image);

        // Fallback icon (hidden by default, shown when no preview available)
        let fallback_image = Image::builder()
            .css_classes(["wallpaper-fallback-icon", "nerd-icon"])
            .halign(Align::Center)
            .valign(Align::Center)
            .pixel_size(config.icon_config.icon_size_scaled(scale))
            .build();
        if let Some(color) = config.icon_config.icon_color() {
            apply_icon_color(&fallback_image, color);
        }
        content_box.append(&fallback_image);

        // Line 1: Theme name (main text)
        let theme_label = build_main_label_scaled(if show_labels { "Loading..." } else { "" }, config.text_colors.main_text_color(), true, Some(12), scale);
        content_box.append(&theme_label);

        // Line 2: Status (info text)
        let status_label = build_info_label_scaled(
            if config.show_status_indicator { "N/A" } else { "" },
            config.text_colors.info_text_color(),
            false,
            None,
            scale,
        );
        content_box.append(&status_label);

        // Line 3: Spacer
        let spacer = build_spacer_scaled(16, scale);
        content_box.append(&spacer);

        *self.preview_image.borrow_mut() = Some(preview_image);
        *self.fallback_image.borrow_mut() = Some(fallback_image);
        *self.theme_label.borrow_mut() = Some(theme_label);
        *self.status_label.borrow_mut() = Some(status_label);

        let button = config.dimensions.build_button_scaled(config.mode, &content_box, "max-width-", scale);

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            preview_image: self.preview_image.clone(),
            fallback_image: self.fallback_image.clone(),
            theme_label: self.theme_label.clone(),
            status_label: self.status_label.clone(),
            latest_status: self.latest_status.clone(),
            widget_view: self.widget_view.clone(),
            personalization: self.personalization.clone(),
        });

        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        if scale != 1.0 {
            apply_widget_scaled_css(&button_widget, scale);
        }
        let message_broadcaster = self.get_broadcaster();
        widget_self.attach_gesture_handlers(&button_widget, &self.config.actions, &message_broadcaster, &GestureHandlersConfiguration::default());

        button_widget
    }
}
