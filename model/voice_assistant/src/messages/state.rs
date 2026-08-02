use serde::Deserialize;
use serde::Serialize;

/// Current state of the voice assistant pipeline.
/// Each variant reflects a distinct phase in the audio-to-action processing chain.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantState {
    /// The assistant is idle and waiting for user activation.
    #[default]
    Idle,
    /// The assistant is in wake-word standby mode, continuously listening for
    /// the configured wake word. When detected, it transitions to `Listening`.
    Standby,
    /// Audio capture is active; the microphone is recording.
    Listening,
    /// Speech-to-text transcription is in progress.
    ProcessingStt,
    /// The LLM is reasoning and selecting tools.
    ThinkingLlm,
    /// A tool is being executed via the MCP tool registry.
    ExecutingAction,
    /// The assistant is speaking the response via TTS.
    Speaking,
    /// An error occurred during the pipeline.
    Error,
}
