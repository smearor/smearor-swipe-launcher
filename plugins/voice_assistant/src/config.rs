use serde::Deserialize;
use serde_json::Value;

/// Configuration for the voice assistant widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VoiceAssistantWidgetConfig {
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Spacing between child widgets inside the voice assistant widget.
    pub spacing: i32,
    /// Size of the icon in pixels.
    pub icon_size: i32,
    /// Whether to show the assistant icon.
    pub show_icon: bool,
    /// Whether to show the transcription text.
    pub show_transcript: bool,
    /// Whether to show the final answer.
    pub show_final_answer: bool,
    /// Icon for the idle state.
    pub icon_idle: String,
    /// Icon for the listening state.
    pub icon_listening: String,
    /// Icon for the processing state.
    pub icon_processing: String,
    /// Icon for the thinking state.
    pub icon_thinking: String,
    /// Icon for the executing state.
    pub icon_executing: String,
    /// Icon for the error state.
    pub icon_error: String,
}

impl VoiceAssistantWidgetConfig {
    /// Parses the widget configuration from a JSON value.
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

impl Default for VoiceAssistantWidgetConfig {
    fn default() -> Self {
        Self {
            width: 120,
            height: 80,
            spacing: 4,
            icon_size: 32,
            show_icon: true,
            show_transcript: true,
            show_final_answer: true,
            icon_idle: "nf-md-microphone_off".to_string(),
            icon_listening: "nf-md-microphone".to_string(),
            icon_processing: "nf-md-waveform".to_string(),
            icon_thinking: "nf-md-brain".to_string(),
            icon_executing: "nf-md-cog_play".to_string(),
            icon_error: "nf-md-alert_circle".to_string(),
        }
    }
}
