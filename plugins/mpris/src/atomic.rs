use crate::labels::MprisLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_mpris_model::MprisCommandMessage;
use smearor_mpris_model::MprisPlaybackStatus;
use smearor_mpris_model::MprisStatusMessage;
use smearor_mpris_model::TOPIC_STATUS;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which MPRIS view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MprisAtomicView {
    /// Current track title.
    Song,
    /// Current track artist.
    Artist,
    /// Current album name.
    Album,
    /// Next track button.
    Next,
    /// Previous track button.
    Previous,
    /// Play/Pause toggle.
    PlayPause,
    /// Stop button.
    Stop,
    /// Switch to next player.
    SwitchPlayer,
    /// Seek forward button.
    SeekForward,
    /// Seek backward button.
    SeekBackward,
    /// Shuffle toggle.
    Shuffle,
    /// Repeat/loop toggle.
    Repeat,
}

impl FromStr for MprisAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mpris_song" => Ok(Self::Song),
            "mpris_artist" => Ok(Self::Artist),
            "mpris_album" => Ok(Self::Album),
            "mpris_next" => Ok(Self::Next),
            "mpris_previous" => Ok(Self::Previous),
            "mpris_play_pause" => Ok(Self::PlayPause),
            "mpris_stop" => Ok(Self::Stop),
            "mpris_switch_player" => Ok(Self::SwitchPlayer),
            "mpris_seek_forward" => Ok(Self::SeekForward),
            "mpris_seek_backward" => Ok(Self::SeekBackward),
            "mpris_shuffle" => Ok(Self::Shuffle),
            "mpris_repeat" => Ok(Self::Repeat),
            _ => Err(format!("Unknown MPRIS atomic view: {s}")),
        }
    }
}

impl MprisAtomicView {
    /// Returns the default nerd font icon name for this view.
    ///
    /// Note: `PlayPause`, `Shuffle`, and `Repeat` have dynamic icons that depend
    /// on playback state and are resolved in `render` instead.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Song => "nf-fa-music",
            Self::Artist => "nf-fa-heart",
            Self::Album => "nf-fa-video_camera",
            Self::Next => "nf-fa-plus_circle",
            Self::Previous => "nf-fa-step_backward",
            Self::PlayPause => "nf-fa-play",
            Self::Stop => "nf-fa-stop",
            Self::SwitchPlayer => "nf-fa-refresh",
            Self::SeekForward => "nf-fa-forward",
            Self::SeekBackward => "nf-fa-backward",
            Self::Shuffle => "nf-fa-arrows_up_down_left_right",
            Self::Repeat => "nf-fa-arrows_up_down_left_right",
        }
    }

    /// Renders this view's display data from the given MPRIS status and personalization override.
    pub fn render(&self, status: &MprisStatusMessage, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.effective_locale();
        if !status.has_player {
            return ViewData::new("nf-fa-music".to_string(), MprisLabel::NoPlayer.localized_label(locale).to_string(), "".to_string());
        }
        let icon_name = self.icon_name();
        match self {
            Self::Song => {
                let title = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.title.is_empty() { None } else { Some(m.title.as_str()) })
                    .unwrap_or(MprisLabel::UnknownTitle.localized_label(locale));
                ViewData::new(icon_name.to_string(), title.to_string(), "".to_string())
            }
            Self::Artist => {
                let artist = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.artist.is_empty() { None } else { Some(m.artist.as_str()) })
                    .unwrap_or(MprisLabel::UnknownArtist.localized_label(locale));
                ViewData::new(icon_name.to_string(), artist.to_string(), "".to_string())
            }
            Self::Album => {
                let album = status
                    .metadata
                    .as_ref()
                    .and_then(|m| if m.album.is_empty() { None } else { Some(m.album.as_str()) })
                    .unwrap_or(MprisLabel::UnknownAlbum.localized_label(locale));
                ViewData::new(icon_name.to_string(), album.to_string(), "".to_string())
            }
            Self::Next => ViewData::new(icon_name.to_string(), "".to_string(), "Next".to_string()),
            Self::Previous => ViewData::new(icon_name.to_string(), "".to_string(), "Prev".to_string()),
            Self::PlayPause => {
                let icon = status.playback_status.playback_icon_name();
                let label = match status.playback_status {
                    MprisPlaybackStatus::Playing => "Pause",
                    MprisPlaybackStatus::Paused => "Play",
                    MprisPlaybackStatus::Stopped => "Play",
                };
                ViewData::new(icon.to_string(), label.to_string(), "".to_string())
            }
            Self::Stop => ViewData::new(icon_name.to_string(), "".to_string(), "Stop".to_string()),
            Self::SwitchPlayer => {
                let player = status
                    .active_player
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or(MprisLabel::NoPlayer.localized_label(locale));
                ViewData::new(icon_name.to_string(), player.to_string(), "".to_string())
            }
            Self::SeekForward => ViewData::new(icon_name.to_string(), "".to_string(), "Seek+".to_string()),
            Self::SeekBackward => ViewData::new(icon_name.to_string(), "".to_string(), "Seek-".to_string()),
            Self::Shuffle => {
                let icon = if status.shuffle { "nf-fa-check_square_o" } else { icon_name };
                let label = if status.shuffle { "On" } else { "Off" };
                ViewData::new(icon.to_string(), label.to_string(), "Shuffle".to_string())
            }
            Self::Repeat => {
                let (icon, label) = match status.loop_status {
                    smearor_mpris_model::MprisLoopStatus::None => (icon_name, "Off"),
                    smearor_mpris_model::MprisLoopStatus::Track => ("nf-fa-share_square_o", "Track"),
                    smearor_mpris_model::MprisLoopStatus::Playlist => ("nf-fa-pencil_square_o", "All"),
                };
                ViewData::new(icon.to_string(), label.to_string(), "Repeat".to_string())
            }
        }
    }
}

/// Atomic MPRIS widget that renders a single media view.
///
/// Subscribes to `service.mpris.status` and renders only the view specified
/// at construction time.
pub struct MprisAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: MprisAtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<MprisStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl MprisAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = MprisAtomicView::from_str(widget_name).unwrap_or(MprisAtomicView::PlayPause);

        let widget = MprisAtomicWidget {
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
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn update_ui(&self, status: &MprisStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data).with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f001}');
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, view_data.main_text_color);
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, view_data.info_text_color);
        }
    }

    /// Extract graphic rendering data from the latest status.
    ///
    /// Returns `(icon_char, main_text, info_text, is_error, icon_color)` for the
    /// centralised rendering pipeline.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            return AtomicGraphicData::error('\u{f001}', "Loading...".to_string());
        };

        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data).with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f001}');
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.main_text_color = view_data.main_text_color.map(|c| c.to_rgba());
        data.info_text_color = view_data.info_text_color.map(|c| c.to_rgba());
        data
    }
}

atomic_widget_impl! {
    widget: MprisAtomicWidget,
    status: MprisStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "mpris-atomic",
    mcp_description: "MPRIS atomic widget",
    css_prefix: "mpris",
    default_icon: '\u{f001}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: MprisCommandMessage::refresh(),
    graphic_renderer: true,
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for MprisAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("mpris atomic widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let time_format = status.time_format;
        let override_data = PersonalizationOverride { locale, time_format };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}
