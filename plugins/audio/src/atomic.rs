use crate::labels::AudioLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_audio_model::AudioCommandMessage;
use smearor_audio_model::AudioStatusMessage;
use smearor_audio_model::TOPIC_STATUS;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_progress_bar;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer;
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

/// Which audio view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioAtomicView {
    /// Volume percentage with icon.
    Volume,
    /// Volume up button.
    VolumeUp,
    /// Volume down button.
    VolumeDown,
    /// Mute toggle display.
    Mute,
    /// Active device name with rotate action.
    RotateDevice,
    /// Multi-span volume slider spanning two or more buttons horizontally.
    VolumeSpan,
}

impl AudioAtomicView {
    /// Returns the default nerd font icon name for this view.
    ///
    /// Note: `Volume` and `Mute` have dynamic icons that depend on volume level
    /// and mute state, resolved in `render_atomic_view` instead.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Volume => "nf-md-volume_high",
            Self::VolumeUp => "nf-fa-arrow_up",
            Self::VolumeDown => "nf-fa-arrow_down",
            Self::Mute => "nf-md-volume_off",
            Self::RotateDevice => "nf-fa-desktop",
            Self::VolumeSpan => "nf-md-volume_high",
        }
    }
}

impl FromStr for AudioAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "audio_volume" => Ok(Self::Volume),
            "audio_volume_up" => Ok(Self::VolumeUp),
            "audio_volume_down" => Ok(Self::VolumeDown),
            "audio_mute" => Ok(Self::Mute),
            "audio_rotate_device" => Ok(Self::RotateDevice),
            "audio_volume_span" => Ok(Self::VolumeSpan),
            _ => Err(format!("Unknown audio atomic view: {s}")),
        }
    }
}

/// Atomic audio widget that renders a single audio view.
///
/// Subscribes to `service.audio.status` and renders only the view specified
/// at construction time. No view switching — each atomic widget is a
/// single-purpose display.
pub struct AudioAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: AudioAtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<AudioStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl AudioAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = AudioAtomicView::from_str(widget_name).unwrap_or(AudioAtomicView::Volume);

        let widget = AudioAtomicWidget {
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

    fn update_ui(&self, status: &AudioStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data).with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f028}');
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
            return AtomicGraphicData::error('\u{f028}', "Loading...".to_string());
        };

        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data).with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f028}');
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.main_text_color = view_data.main_text_color.map(|c| c.to_rgba());
        data.info_text_color = view_data.info_text_color.map(|c| c.to_rgba());
        data
    }
}

/// Select the nerd font icon name for a given volume and mute state.
pub fn volume_icon_name(volume: f32, is_muted: bool) -> &'static str {
    if is_muted {
        if volume > 0.0 { "nf-md-volume_off" } else { "nf-md-volume_variant_off" }
        // "nf-md-volume_mute"
        // "nf-fa-volume_off"
    } else if volume > 1.0 {
        "nf-md-volume_vibrate"
    } else if volume > 0.66 {
        "nf-md-volume_high"
        // "nf-fa-volume_up"
    } else if volume > 0.33 {
        "nf-md-volume_medium"
        // "nf-fa-volume_low"
    } else if volume > 0.0 {
        "nf-md-volume_low"
    } else {
        "nf-md-volume_off"
    }
}

impl AudioAtomicView {
    /// Renders this view's display data from the current audio status and personalization override.
    pub fn render(&self, status: &AudioStatusMessage, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.effective_locale();
        match self {
            Self::Volume => {
                let icon = volume_icon_name(status.volume, status.is_muted);
                let pct = if status.is_muted {
                    AudioLabel::Muted.localized_label(locale).to_string()
                } else {
                    format!("{:.0}%", status.volume * 100.0)
                };
                let device = status.active_device.as_ref().map(|d| d.name.as_str()).unwrap_or("");
                ViewData::new(icon.to_string(), pct, device.to_string())
            }
            Self::VolumeUp => ViewData::new(self.icon_name().to_string(), "".to_string(), AudioLabel::VolumeUp.localized_label(locale).to_string()),
            Self::VolumeDown => ViewData::new(self.icon_name().to_string(), "".to_string(), AudioLabel::VolumeDown.localized_label(locale).to_string()),
            Self::Mute => {
                if status.is_muted {
                    ViewData::new(self.icon_name().to_string(), AudioLabel::Muted.localized_label(locale).to_string(), "".to_string())
                } else {
                    ViewData::new("nf-md-volume_high".to_string(), AudioLabel::Mute.localized_label(locale).to_string(), "".to_string())
                }
            }
            Self::RotateDevice => {
                let device = status
                    .active_device
                    .as_ref()
                    .map(|d| d.name.as_str())
                    .unwrap_or(AudioLabel::NoDevice.localized_label(locale));
                ViewData::new(self.icon_name().to_string(), device.to_string(), AudioLabel::NextDevice.localized_label(locale).to_string())
            }
            Self::VolumeSpan => {
                let icon = volume_icon_name(status.volume, status.is_muted);
                let pct = if status.is_muted {
                    AudioLabel::Muted.localized_label(locale).to_string()
                } else {
                    format!("{:.0}%", status.volume * 100.0)
                };
                ViewData::new(icon.to_string(), pct, "".to_string())
            }
        }
    }
}

atomic_widget_impl! {
    widget: AudioAtomicWidget,
    status: AudioStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "audio-atomic",
    mcp_description: "Audio atomic widget",
    css_prefix: "audio",
    default_icon: '\u{f028}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: AudioCommandMessage::refresh(),
    graphic_renderer: true,
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl AtomicGraphicRenderer for AudioAtomicWidget {
    fn render_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        if self.view != AudioAtomicView::VolumeSpan {
            return false;
        }

        let text_col = text_color(false);
        fill_background(pixels, width, height, background_color(false));

        let status = self.latest_status.borrow();
        let (volume, is_muted) = match &*status {
            Some(status) => (status.volume, status.is_muted),
            None => (0.0_f32, false),
        };

        let icon_name = volume_icon_name(volume, is_muted);
        let icon_char = resolve_icon_codepoint(icon_name).unwrap_or('\u{f028}');

        let icon_size = (height as f32 * 0.5).min(40.0);
        let icon_x = (height as f32 / 2.0).min(width as f32 * 0.15);
        draw_nerd_font_codepoint(pixels, width, height, icon_char, icon_x, height as f32 * 0.35, icon_size, text_col);

        let locale = self.personalization.borrow().effective_locale();
        let pct = if is_muted {
            AudioLabel::Muted.localized_label(locale).to_string()
        } else {
            format!("{:.0}%", volume * 100.0)
        };
        let font_size = (height as f32 * 0.35).min(28.0).max(14.0);
        draw_text_centered(pixels, width, height, &pct, height as f32 * 0.40, font_size, text_col);

        draw_progress_bar(pixels, width, height, if is_muted { 0.0 } else { volume }, text_col);

        true
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for AudioAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("audio atomic widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}
