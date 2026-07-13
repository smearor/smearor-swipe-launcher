/// Current state of the voice assistant pipeline.
/// Each variant reflects a distinct phase in the audio-to-action processing chain.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AssistantState {
    /// The assistant is idle and waiting for user activation.
    #[default]
    Idle,
    /// Audio capture is active; the microphone is recording.
    Listening,
    /// Speech-to-text transcription is in progress.
    ProcessingStt,
    /// The LLM is reasoning and selecting tools.
    ThinkingLlm,
    /// A tool is being executed via the MCP tool registry.
    ExecutingAction,
    /// An error occurred during the pipeline.
    Error,
}
