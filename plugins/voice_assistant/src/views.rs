use gtk4::Image;
use gtk4::Label;
use gtk4::Spinner;
use gtk4::prelude::WidgetExt;
use smearor_voice_assistant_model::AssistantState;

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
pub fn render_state(
    state: &AssistantState,
    config: &crate::config::VoiceAssistantWidgetConfig,
    transcript: &str,
    final_answer: Option<&str>,
    active_tool: Option<&str>,
    error_message: Option<&str>,
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
        AssistantState::Listening => ViewState {
            icon_name: config.icon_listening.clone(),
            primary_label: "Listening...".to_string(),
            secondary_label: if transcript.is_empty() { None } else { Some(transcript.to_string()) },
            spinning: false,
        },
        AssistantState::ProcessingStt => ViewState {
            icon_name: config.icon_processing.clone(),
            primary_label: "Transcribing...".to_string(),
            secondary_label: None,
            spinning: true,
        },
        AssistantState::ThinkingLlm => ViewState {
            icon_name: config.icon_thinking.clone(),
            primary_label: "Thinking...".to_string(),
            secondary_label: None,
            spinning: true,
        },
        AssistantState::ExecutingAction => {
            let label = active_tool
                .map(|tool| format!("Executing: {tool}"))
                .unwrap_or_else(|| "Executing...".to_string());
            ViewState {
                icon_name: config.icon_executing.clone(),
                primary_label: label,
                secondary_label: None,
                spinning: true,
            }
        }
        AssistantState::Error => ViewState {
            icon_name: config.icon_error.clone(),
            primary_label: "Error".to_string(),
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
        icon.set_icon_name(Some(&view_state.icon_name));
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
