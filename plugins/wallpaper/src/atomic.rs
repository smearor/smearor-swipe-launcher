use crate::labels::WallpaperLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use smearor_wallpaper_model::TOPIC_STATUS;
use smearor_wallpaper_model::WallpaperCommandMessage;
use smearor_wallpaper_model::WallpaperStatusMessage;
use smearor_wallpaper_model::WallpaperThemeInfo;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which wallpaper action an atomic widget represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicView {
    /// Wallpaper selector — preview image, selects/applies wallpaper on click.
    Selector,
    /// Cycle to next wallpaper on click.
    Next,
    /// Cycle to previous wallpaper on click.
    Previous,
    /// Set random wallpaper on click.
    Random,
    /// Current wallpaper thumbnail, opens wallpaper settings on click.
    Current,
}

impl FromStr for AtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wallpaper_selector" => Ok(Self::Selector),
            "wallpaper_next" => Ok(Self::Next),
            "wallpaper_previous" => Ok(Self::Previous),
            "wallpaper_random" => Ok(Self::Random),
            "wallpaper_current" => Ok(Self::Current),
            _ => Err(format!("Unknown wallpaper atomic view: {s}")),
        }
    }
}

impl AtomicView {
    /// Returns the label key for this view.
    fn label(&self) -> WallpaperLabel {
        match self {
            Self::Selector => WallpaperLabel::Selector,
            Self::Next => WallpaperLabel::Next,
            Self::Previous => WallpaperLabel::Previous,
            Self::Random => WallpaperLabel::Random,
            Self::Current => WallpaperLabel::Current,
        }
    }

    /// Returns the Nerd Font icon name for this view.
    fn icon_name(&self) -> &'static str {
        match self {
            Self::Selector => "nf-md-image_multiple",
            Self::Next => "nf-md-skip_next",
            Self::Previous => "nf-md-skip_previous",
            Self::Random => "nf-md-shuffle",
            Self::Current => "nf-md-wallpaper",
        }
    }

    /// Returns the fallback icon codepoint for this view.
    fn fallback_icon_char(&self) -> char {
        match self {
            Self::Selector => '\u{f1c5}',
            Self::Next => '\u{f04ad}',
            Self::Previous => '\u{f04ac}',
            Self::Random => '\u{f04b6}',
            Self::Current => '\u{f1c5}',
        }
    }
}

/// Atomic wallpaper widget that renders a single wallpaper action.
///
/// Subscribes to `service.wallpaper.status` and renders only the action specified
/// at construction time. Click triggers the action, longpress opens the selection area.
pub struct WallpaperAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: AtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<WallpaperStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl WallpaperAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();
        let view = AtomicView::from_str(widget_name).unwrap_or(AtomicView::Current);

        let widget = WallpaperAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn update_ui(&self, status: &WallpaperStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.render_view_data(status, &override_data);
        let icon_char = resolve_icon_codepoint(self.view.icon_name()).unwrap_or(self.view.fallback_icon_char());

        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, self.config.text_colors.main_text_color());
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, self.config.text_colors.info_text_color());
        }
    }

    /// Extract graphic rendering data from the latest status.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            let icon_char = resolve_icon_codepoint(self.view.icon_name()).unwrap_or(self.view.fallback_icon_char());
            return AtomicGraphicData::new(icon_char, self.view.label().localized_label(Locale::default()), "Loading...".to_string());
        };

        let override_data = self.personalization.borrow().clone();
        let view_data = self.render_view_data(status, &override_data);
        let icon_char = resolve_icon_codepoint(self.view.icon_name()).unwrap_or(self.view.fallback_icon_char());
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.main_text_color = self.config.text_colors.main_text_color().map(|c| c.to_rgba());
        data.info_text_color = self.config.text_colors.info_text_color().map(|c| c.to_rgba());
        data
    }

    /// Renders the view data (icon, main text, info text) for this atomic view.
    fn render_view_data(&self, status: &WallpaperStatusMessage, override_data: &PersonalizationOverride) -> AtomicGraphicData {
        let locale = override_data.locale;
        let theme_info: Option<WallpaperThemeInfo> = status.themes.get(status.selected_theme_index).cloned();

        match self.view {
            AtomicView::Selector | AtomicView::Current => {
                let theme_name = match &theme_info {
                    Some(theme) => theme.name.to_string(),
                    None => WallpaperLabel::NoTheme.localized_label(locale),
                };
                let info_text = if status.is_running() {
                    WallpaperLabel::Running.localized_label(locale)
                } else {
                    WallpaperLabel::Stopped.localized_label(locale)
                };
                AtomicGraphicData::new(self.view.fallback_icon_char(), theme_name, info_text)
            }
            AtomicView::Next => {
                let info_text = theme_info.as_ref().map(|t| t.name.to_string()).unwrap_or_default();
                AtomicGraphicData::new(self.view.fallback_icon_char(), WallpaperLabel::Next.localized_label(locale), info_text)
            }
            AtomicView::Previous => {
                let info_text = theme_info.as_ref().map(|t| t.name.to_string()).unwrap_or_default();
                AtomicGraphicData::new(self.view.fallback_icon_char(), WallpaperLabel::Previous.localized_label(locale), info_text)
            }
            AtomicView::Random => {
                let info_text = theme_info.as_ref().map(|t| t.name.to_string()).unwrap_or_default();
                AtomicGraphicData::new(self.view.fallback_icon_char(), WallpaperLabel::Random.localized_label(locale), info_text)
            }
        }
    }
}

atomic_widget_impl! {
    widget: WallpaperAtomicWidget,
    status: WallpaperStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "wallpaper-atomic",
    mcp_description: "Wallpaper atomic widget",
    css_prefix: "wallpaper",
    default_icon: '\u{f1c5}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: WallpaperCommandMessage::refresh(),
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for WallpaperAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("wallpaper atomic widget: received personalization status");
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
