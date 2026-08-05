use serde::Deserialize;
use serde_json::Value;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::DEFAULT_ICON_SIZE;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetLayout;

/// Configuration for the voice assistant widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VoiceAssistantWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
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
    /// Icon for the speaking state.
    pub icon_speaking: String,
    /// Icon for the error state.
    pub icon_error: String,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
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
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_size: DEFAULT_ICON_SIZE,
            show_icon: true,
            show_transcript: true,
            show_final_answer: true,
            icon_idle: "nf-md-microphone_off".to_string(),
            icon_listening: "nf-md-microphone".to_string(),
            icon_processing: "nf-md-waveform".to_string(),
            icon_thinking: "nf-md-brain".to_string(),
            icon_executing: "nf-md-cog_play".to_string(),
            icon_speaking: "nf-md-volume_high".to_string(),
            icon_error: "nf-md-alert_circle".to_string(),
            actions: ActionBindings::default(),
        }
    }
}
