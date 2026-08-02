use crate::labels::VoiceAssistantLabel;
use gtk4::Image;
use gtk4::Label;
use gtk4::Spinner;
use gtk4::prelude::WidgetExt;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use smearor_voice_assistant_model::AssistantState;

/// Sets the icon on a GTK Image, resolving Nerd Font names to GTK icon names.
pub fn set_icon_image(icon: &Image, icon_name: &str) {
    if icon_name.starts_with("nf-") {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            icon.set_icon_name(Some(&gtk_icon_name));
        }
    } else {
        icon.set_icon_name(Some(icon_name));
    }
}

/// Snapshot of the latest assistant status fields needed by headless and web renderers.
///
/// Extracted from `AssistantStatusMessage` so that `GraphicRenderer` and `WebRenderer`
/// can work with a single named struct instead of positional tuples.
#[derive(Clone, Debug, Default)]
pub struct StatusSnapshot {
    /// Current pipeline state.
    pub state: AssistantState,
    /// Partial or complete transcription text.
    pub transcript: String,
    /// The last final LLM answer, if any.
    pub final_answer: Option<String>,
    /// Error message when state is `Error`.
    pub error_message: Option<String>,
}

impl StatusSnapshot {
    /// Creates a snapshot from an optional `AssistantStatusMessage`.
    pub fn from_status(status: Option<&smearor_voice_assistant_model::AssistantStatusMessage>) -> Self {
        match status {
            Some(s) => Self {
                state: s.current_state.clone(),
                transcript: s.partial_transcript.clone(),
                final_answer: s.final_answer.clone(),
                error_message: s.error_message.clone(),
            },
            None => Self::default(),
        }
    }
}

/// Rendering data for a single assistant state.
pub struct ViewState {
    /// The icon name to display.
    pub icon_name: String,
    /// The primary label text.
    pub primary_label: String,
    /// The secondary label text (transcript or error).
    pub secondary_label: Option<String>,
    /// Whether the spinner should be spinning.
    pub spinning: bool,
}

/// Builds a `ViewState` from an assistant state and optional status fields.
/// Uses the given locale for localized labels.
pub fn render_state(
    state: &AssistantState,
    config: &crate::config::VoiceAssistantWidgetConfig,
    transcript: &str,
    final_answer: Option<&str>,
    active_tool: Option<&str>,
    error_message: Option<&str>,
    locale: Locale,
) -> ViewState {
    match state {
        AssistantState::Idle => {
            let primary = final_answer.unwrap_or("").to_string();
            ViewState {
                icon_name: config.icon_idle.clone(),
                primary_label: primary,
                secondary_label: None,
                spinning: false,
            }
        }
        AssistantState::Standby => ViewState {
            icon_name: config.icon_idle.clone(),
            primary_label: VoiceAssistantLabel::Standby.localized_label(locale).to_string(),
            secondary_label: None,
            spinning: false,
        },
        AssistantState::Listening => ViewState {
            icon_name: config.icon_listening.clone(),
            primary_label: VoiceAssistantLabel::Listening.localized_label(locale).to_string(),
            secondary_label: if transcript.is_empty() { None } else { Some(transcript.to_string()) },
            spinning: false,
        },
        AssistantState::ProcessingStt => ViewState {
            icon_name: config.icon_processing.clone(),
            primary_label: VoiceAssistantLabel::Transcribing.localized_label(locale).to_string(),
            secondary_label: None,
            spinning: true,
        },
        AssistantState::ThinkingLlm => ViewState {
            icon_name: config.icon_thinking.clone(),
            primary_label: VoiceAssistantLabel::Thinking.localized_label(locale).to_string(),
            secondary_label: None,
            spinning: true,
        },
        AssistantState::ExecutingAction => {
            let label = active_tool
                .map(|tool| format!("{}: {tool}", VoiceAssistantLabel::Executing.localized_label(locale)))
                .unwrap_or_else(|| VoiceAssistantLabel::Executing.localized_label(locale).to_string());
            ViewState {
                icon_name: config.icon_executing.clone(),
                primary_label: label,
                secondary_label: None,
                spinning: true,
            }
        }
        AssistantState::Speaking => ViewState {
            icon_name: config.icon_speaking.clone(),
            primary_label: VoiceAssistantLabel::Speaking.localized_label(locale).to_string(),
            secondary_label: final_answer.map(|a| a.to_string()),
            spinning: false,
        },
        AssistantState::Error => ViewState {
            icon_name: config.icon_error.clone(),
            primary_label: VoiceAssistantLabel::Error.localized_label(locale).to_string(),
            secondary_label: error_message.map(|msg| msg.to_string()),
            spinning: false,
        },
    }
}

/// Applies a `ViewState` to GTK widgets.
pub fn apply_state(
    view_state: &ViewState,
    icon: &Image,
    primary_label: &Label,
    secondary_label: &Label,
    spinner: &Spinner,
    show_icon: bool,
    show_transcript: bool,
) {
    if show_icon {
        set_icon_image(icon, &view_state.icon_name);
        icon.set_visible(true);
    } else {
        icon.set_visible(false);
    }

    primary_label.set_text(&view_state.primary_label);

    if let Some(ref secondary) = view_state.secondary_label {
        if show_transcript {
            secondary_label.set_text(secondary);
            secondary_label.set_visible(true);
        } else {
            secondary_label.set_visible(false);
        }
    } else {
        secondary_label.set_visible(false);
    }

    spinner.set_spinning(view_state.spinning);
    spinner.set_visible(view_state.spinning);
}
