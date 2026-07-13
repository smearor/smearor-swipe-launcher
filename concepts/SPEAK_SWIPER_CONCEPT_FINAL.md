# Concept: Voice Assistant Service & Widget

This document describes the concept for a **Voice Assistant Agent Service** and an associated **Voice Assistant Widget** in the *Smearor Swipe Launcher*. The
system enables completely local, private, and low-latency voice control using state-of-the-art AI on the CPU. It records audio, converts speech to text via
Whisper, orchestrates an autonomous ReAct loop with a 1B–3B parameter Large Language Model (LLM), and interfaces with the in-process MCP tool registry to
dynamically discover and invoke tools from any active service or plugin (e.g., Weather, Wallpaper, Network, App Launcher, Audio, MPRIS, Power).

The system follows the launcher's decoupled SOA architecture:

1. **Model Crate (`model/voice_assistant`):** Shared structs, enums, topics, and stable ABI message formats.
2. **Service Crate (`services/voice_assistant`):** Background service implementing the local AI pipeline (audio capture, speech-to-text, LLM ReAct loop) and
   acting as an in-process MCP client via the message broker.
3. **Widget Crate (`plugins/voice_assistant`):** A pure GTK4 touch-optimized UI tile displaying status, transcriptions, and activation states.

---

## 1. System Architecture & Data Flow

The entire pipeline runs inside the launcher process space. The voice assistant service acts as an **in-process MCP client**: instead of spawning the MCP server
as a subprocess, it subscribes to `mcp.register.tool` to build a dynamic tool catalog, and invokes tools via `mcp.invoke.tool` / `mcp.tool.response` on the
central message broker. This eliminates stdio overhead and provides near-zero-latency tool execution.

```
+---------------------------+                 +----------------------------------+
| Voice Assistant Widget    |                 | Voice Assistant Service          |
| (GTK4 UI Tile / Status)   |                 | (Singleton Background Executor)  |
+---------------------------+                 +----------------------------------+
              |                                                 |
              |  1. Toggle Recording (Click/Long-Press)         |
              +------------------------------------------------>|
              |                                                 | [Audio Capture: cpal]
              |                                                 |         | (PCM Data)
              |                                                 |         v
              |                                                 | [STT: whisper-rs]
              |                                                 |         | (User Text Intent)
              |                                                 |         v
              |                                                 | [LLM Loop: llama-cpp-2]
              |                                                 |         |
              |                                                 |   === ReAct Loop Begins ===
              |                                                 |   2. Query Tool Catalog
              |                                                 |      (subscribed to mcp.register.tool)
              |                                                 |   3. LLM Reasoning + Tool Selection
              |                                                 |   4. Invoke Tool via mcp.invoke.tool
              |                                                 |   5. Receive Result via mcp.tool.response
              |                                                 |   6. Feed Result back to LLM
              |                                                 |   === ReAct Loop Ends ===
              |                                                 |
              |  7. Broadcast Status Update                    |
              |<------------------------------------------------+
              |     Topic: service.voice_assistant.status      |
```

### The Autonomous Dynamic Execution Loop (ReAct Pattern)

When the user submits an intent, the service acts as an orchestration client. Instead of mapping plugins explicitly, it uses the in-process MCP tool registry to
inspect active tools and their JSON schemas dynamically.

1. **Discovery**: The service subscribes to `mcp.register.tool` and maintains a live catalog of all registered tools (name, description, input schema). This
   catalog is updated automatically as plugins register or unregister tools.
2. **Reasoning (LLM)**: The local LLM processes the user's text input alongside the injected tool catalog. The system prompt instructs the LLM to output
   structured JSON: either a tool call or a final answer.
3. **Execution**: The service parses the LLM output. If it is a tool call, the service broadcasts an `InvokeToolMessage` to `mcp.invoke.tool` with a correlation
   ID and the tool name + arguments. The owning plugin executes the tool and responds with an `InvokeToolResponse` on `mcp.tool.response`.
4. **Observation**: The service matches the response by correlation ID, feeds the tool result back into the LLM context, and continues the loop. This allows
   multi-step tasks (e.g., Check Weather -> Condition met -> Trigger Wallpaper Action).

---

## 2. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path                        | Responsibility                                                                              |
|-------------|-----------------------------|---------------------------------------------------------------------------------------------|
| **Model**   | `model/voice_assistant/`    | Shared structs, enums, topics, and message formats                                          |
| **Service** | `services/voice_assistant/` | Backend logic: audio capture, STT, LLM ReAct loop, in-process MCP tool discovery/invocation |
| **Widget**  | `plugins/voice_assistant/`  | GTK4 user interface: status display, transcription feedback, activation toggle              |

---

## 3. Model Crate (`model/voice_assistant`)

Following the strict code isolation pattern, all public types are split into specialized single-file modules. To ensure FFI and ABI stability across plugins via
`#[stabby::stabby]`, standard heap structures are wrapped.

### 3.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.voice_assistant.command";
pub const TOPIC_STATUS: &str = "service.voice_assistant.status";
```

### 3.2 Assistant State Enum (`state.rs`)

```rust
/// Current state of the voice assistant pipeline.
/// Each variant reflects a distinct phase in the audio-to-action processing chain.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
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
```

### 3.3 Command Message (`command.rs`)

```rust
/// Actions the voice assistant service can perform on request.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum VoiceCommandAction {
    /// Start audio capture and begin the voice pipeline.
    #[default]
    Activate,
    /// Stop audio capture and cancel any in-progress pipeline.
    Deactivate,
    /// Submit a text command directly (bypassing STT, e.g., from a text input).
    SubmitText,
}

/// Command message sent by widgets or external clients to the voice assistant service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct VoiceCommandMessage {
    /// The action to execute.
    pub action: VoiceCommandAction,
    /// Text input when action is SubmitText; empty for Activate/Deactivate.
    pub text: stabby::string::String,
}
```

### 3.4 Status Message (`status.rs`)

```rust
/// Status message broadcast by the voice assistant service.
/// Contains the current pipeline state, partial transcription, and optional error details.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct AssistantStatusMessage {
    /// Current pipeline state.
    pub current_state: AssistantState,
    /// Partial or complete transcription of the user's speech.
    pub partial_transcript: stabby::string::String,
    /// The last final answer produced by the LLM (if any).
    pub final_answer: stabby::option::Option<stabby::string::String>,
    /// The tool currently being executed (if in ExecutingAction state).
    pub active_tool: stabby::option::Option<stabby::string::String>,
    /// Error message when current_state is Error.
    pub error_message: stabby::option::Option<stabby::string::String>,
}
```

### 3.5 Tool Catalog Entry (`tool_catalog.rs`)

```rust
/// A discovered tool from the MCP tool registry.
/// Used internally by the service to build the LLM system prompt.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolCatalogEntry {
    /// Tool name (e.g., "system_power_action").
    pub name: String,
    /// Human-readable description of the tool.
    pub description: String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: String,
}
```

### 3.6 LLM Response Parsing (`llm_response.rs`)

```rust
/// Parsed output from the LLM during a ReAct loop iteration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LlmResponse {
    /// The LLM requests a tool call.
    ToolCall {
        /// Name of the tool to invoke.
        tool: String,
        /// JSON-encoded arguments for the tool.
        arguments: serde_json::Value,
    },
    /// The LLM has reached a final answer.
    FinalAnswer {
        /// The final response text for the user.
        answer: String,
    },
}
```

### 3.7 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::command::VoiceCommandAction;
pub use messages::command::VoiceCommandMessage;
pub use messages::llm_response::LlmResponse;
pub use messages::state::AssistantState;
pub use messages::status::AssistantStatusMessage;
pub use messages::tool_catalog::ToolCatalogEntry;
pub use messages::topics::TOPIC_COMMAND;
pub use messages::topics::TOPIC_STATUS;
```

---

## 4. Service Crate (`services/voice_assistant`)

The service runs as an asynchronous worker utilizing the central runtime environment. It holds the reference states for hardware streaming, Whisper contexts,
and LLM static layers. It subscribes to `mcp.register.tool` to maintain a live tool catalog and invokes tools via `mcp.invoke.tool` / `mcp.tool.response`.

### 4.1 File Structure

- `service.rs` - `VoiceAssistantService` struct and trait implementations
- `config.rs` - `VoiceAssistantServiceConfig` struct and parsing
- `audio.rs` - Audio capture logic using `cpal`
- `transcriber.rs` - Speech-to-text logic using `whisper-rs`
- `llm.rs` - LLM inference engine using `llama-cpp-2`
- `react.rs` - ReAct loop orchestration
- `tool_catalog.rs` - Tool catalog management (subscribe to `mcp.register.tool`)
- `mcp.rs` - MCP tool invocation and resource querying
- `lib.rs` - `service_plugin!` macro invocation

### 4.2 Service Implementation

```rust
pub struct VoiceAssistantService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: VoiceAssistantServiceConfig,
    pub state: Arc<RwLock<AssistantState>>,
    pub tool_catalog: Arc<RwLock<Vec<ToolCatalogEntry>>>,
    pub whisper_context: Option<Arc<whisper_rs::WhisperContext>>,
    pub llm_engine: Option<Arc<LlmInferenceEngine>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<VoiceCommandMessage>>` - Processes activate/deactivate/submit_text commands
- `MessageHandler<FfiEnvelopePayload<RegisterToolMessage>>` - Receives tool registrations from plugins
- `MessageHandler<FfiEnvelopePayload<InvokeToolResponse>>` - Receives tool execution results
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 4.3 Configuration

```rust
/// Configuration for the voice assistant service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VoiceAssistantServiceConfig {
    /// Path to the Whisper GGML model file (e.g., "models/ggml-tiny.bin").
    pub whisper_model_path: String,
    /// Path to the LLM GGUF model file (e.g., "models/qwen2.5-1.5b-instruct-q4_k_m.gguf").
    pub llm_model_path: String,
    /// Number of CPU threads for LLM inference.
    pub llm_threads: u32,
    /// Maximum context window size in tokens for the LLM.
    pub llm_context_size: u32,
    /// Maximum number of ReAct loop iterations before giving up.
    pub max_react_iterations: u32,
    /// Sampling temperature for the LLM (0.0 = deterministic, 1.0 = creative).
    pub llm_temperature: f32,
    /// Audio sample rate for capture (Hz). Whisper expects 16000 Hz.
    pub audio_sample_rate: u32,
    /// Audio channels (1 = mono).
    pub audio_channels: u16,
    /// Maximum recording duration in seconds before auto-stopping.
    pub max_recording_seconds: u32,
    /// Silence detection threshold in seconds (stop recording after this much silence).
    pub silence_threshold_seconds: f32,
    /// System language for Whisper (e.g., "de" for German, "en" for English).
    pub language: String,
    /// Whether to enable the voice assistant on startup.
    pub auto_enable: bool,
}

impl Default for VoiceAssistantServiceConfig {
    fn default() -> Self {
        Self {
            whisper_model_path: "models/ggml-tiny.bin".to_string(),
            llm_model_path: "models/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            llm_threads: 4,
            llm_context_size: 2048,
            max_react_iterations: 8,
            llm_temperature: 0.1,
            audio_sample_rate: 16000,
            audio_channels: 1,
            max_recording_seconds: 30,
            silence_threshold_seconds: 1.5,
            language: "en".to_string(),
            auto_enable: false,
        }
    }
}
```

### 4.4 Audio Capture (`audio.rs`)

Audio capture uses `cpal` to access the physical microphone and stream PCM audio into RAM buffers. The capture runs on a dedicated thread to avoid blocking the
async runtime. A `tokio::sync::oneshot` channel signals the capture thread to stop gracefully (either from user deactivation, silence detection, or max-duration
auto-stop).

#### 4.4.1 Design Constraints

- **Thread isolation:** `cpal::Stream` is `!Send` on some platforms. The stream is created, played, and dropped on the same dedicated thread. The audio data is
  collected into a `Arc<Mutex<Vec<f32>>>` shared with the callback closure.
- **Sample format:** Whisper expects 32-bit float PCM at 16 kHz mono. The capture stream is configured to deliver `f32` samples directly. If the hardware device
  does not natively support `f32`, `cpal` performs the conversion internally.
- **Silence detection:** A rolling window of RMS (root mean square) amplitude is computed over the incoming samples. If the RMS stays below a fixed threshold (
  e.g., `0.01`) for a continuous duration exceeding `silence_threshold_seconds`, the recording auto-stops.
- **Max duration:** A hard ceiling of `max_recording_seconds` prevents unbounded recording. At 16 kHz, this is `max_recording_seconds * 16000` samples.
- **Cancellation:** The caller can send a stop signal via a `tokio::sync::oneshot::Sender<()>` at any time. The capture thread checks this signal between buffer
  writes.

#### 4.4.2 Error Types

```rust
/// Errors that can occur during audio capture.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No default input device was found on the system.
    #[error("No default input device found")]
    NoDefaultInputDevice,
    /// Failed to query the default input configuration from the device.
    #[error("Failed to get default input config: {0}")]
    DefaultInputConfig(String),
    /// The selected device does not support the required sample format (f32).
    #[error("Unsupported sample format: {0}")]
    UnsupportedSampleFormat(String),
    /// Failed to build the input stream from the device.
    #[error("Failed to build input stream: {0}")]
    StreamBuild(String),
    /// Failed to start playback on the input stream.
    #[error("Failed to play stream: {0}")]
    StreamPlay(String),
    /// The capture was cancelled before any audio was recorded.
    #[error("Capture cancelled")]
    Cancelled,
    /// No audio data was captured (zero-length buffer).
    #[error("No audio data captured")]
    EmptyBuffer,
}
```

#### 4.4.3 Audio Capture Implementation

```rust
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use cpal::BufferSize;
use cpal::SampleFormat;
use cpal::SampleRate;
use cpal::StreamConfig;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::trace;

/// The RMS amplitude threshold below which audio is considered "silence".
const SILENCE_RMS_THRESHOLD: f32 = 0.01;

/// Captures audio from the default input device until silence is detected,
/// the max recording duration is reached, or a stop signal is received.
///
/// Returns a buffer of 32-bit float PCM samples at 16 kHz mono, suitable
/// for direct ingestion by `whisper-rs`.
pub async fn capture_audio(
    config: &VoiceAssistantServiceConfig,
    stop_rx: oneshot::Receiver<()>,
) -> Result<Vec<f32>, AudioError> {
    let max_samples = (config.max_recording_seconds as usize) * (config.audio_sample_rate as usize);
    let silence_window_samples = (config.silence_threshold_seconds as usize) * (config.audio_sample_rate as usize);

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(max_samples)));
    let buffer_clone = buffer.clone();

    let (done_tx, done_rx) = oneshot::channel::<Result<Vec<f32>, AudioError>>();

    // Spawn the capture thread. cpal::Stream is !Send on some platforms,
    // so the entire stream lifecycle must live on this thread.
    std::thread::spawn(move || {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(device) => device,
            None => {
                let _ = done_tx.send(Err(AudioError::NoDefaultInputDevice));
                return;
            }
        };
        debug!("Audio capture: selected input device: {:?}", device.name());

        let supported_config = match device.default_input_config() {
            Ok(config) => config,
            Err(error) => {
                let _ = done_tx.send(Err(AudioError::DefaultInputConfig(error.to_string())));
                return;
            }
        };

        // We require f32 samples. If the device does not support f32 natively,
        // we attempt to request it anyway — cpal will convert internally on most platforms.
        let sample_format = if supported_config.sample_format() == SampleFormat::F32 {
            SampleFormat::F32
        } else {
            debug!(
                "Audio capture: device native format is {:?}, requesting F32 with conversion",
                supported_config.sample_format()
            );
            SampleFormat::F32
        };

        let stream_config = StreamConfig {
            channels: config.audio_channels,
            sample_rate: SampleRate(config.audio_sample_rate),
            buffer_size: BufferSize::Default,
        };

        // Track consecutive silence samples for silence detection.
        let consecutive_silence: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let consecutive_silence_clone = consecutive_silence.clone();

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap_or_default();
                let mut silence_count = consecutive_silence_clone.lock().unwrap_or_default();

                for &sample in data {
                    // If the buffer is full, stop capturing.
                    if buf.len() >= max_samples {
                        break;
                    }
                    buf.push(sample);

                    // RMS-based silence detection on individual samples.
                    // We use absolute amplitude as a lightweight proxy for RMS over a window.
                    if sample.abs() < SILENCE_RMS_THRESHOLD {
                        *silence_count += 1;
                    } else {
                        *silence_count = 0;
                    }
                }

                // Check if silence threshold has been exceeded.
                if *silence_count >= silence_window_samples && buf.len() > silence_window_samples {
                    debug!(
                        "Audio capture: silence detected after {} samples ({}s)",
                        buf.len(),
                        buf.len() / (config.audio_sample_rate as usize)
                    );
                    // The stream will be stopped by the outer loop after this callback returns.
                    // We signal completion by truncating the buffer to mark it as "done".
                    // The outer loop checks buffer length against max_samples or silence.
                }
            },
            |error| {
                tracing::error!("Audio capture stream error: {}", error);
            },
            None,
        );

        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                let _ = done_tx.send(Err(AudioError::StreamBuild(error.to_string())));
                return;
            }
        };

        if let Err(error) = stream.play() {
            let _ = done_tx.send(Err(AudioError::StreamPlay(error.to_string())));
            return;
        }
        debug!("Audio capture: stream started");

        // Wait for either: stop signal, silence, or max duration.
        // We poll the buffer length every 50ms to check if silence or max duration was reached.
        let mut stop_rx = stop_rx;
        loop {
            // Check if the stop signal has been sent.
            match stop_rx.try_recv() {
                Ok(()) | Err(oneshot::error::TryRecvError::Closed) => {
                    debug!("Audio capture: stop signal received");
                    break;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
            }

            // Check if max duration reached.
            let current_len = buffer.lock().map(|buf| buf.len()).unwrap_or(0);
            if current_len >= max_samples {
                debug!("Audio capture: max recording duration reached");
                break;
            }

            // Check if silence was detected.
            let silence_count = consecutive_silence.lock().map(|count| *count).unwrap_or(0);
            if silence_count >= silence_window_samples && current_len > silence_window_samples {
                debug!("Audio capture: silence threshold exceeded, stopping");
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Drop the stream to stop capture and flush any pending callbacks.
        drop(stream);
        debug!("Audio capture: stream stopped");

        // Extract the captured samples.
        let samples = buffer.lock().map(|mut buf| {
            let drained = std::mem::take(&mut *buf);
            drained
        }).unwrap_or_default();

        if samples.is_empty() {
            let _ = done_tx.send(Err(AudioError::EmptyBuffer));
        } else {
            debug!("Audio capture: captured {} samples ({}s)", samples.len(), samples.len() / 16000);
            let _ = done_tx.send(Ok(samples));
        }
    });

    let result = done_rx.await
        .map_err(|_| AudioError::Cancelled)?;
    result
}
```

#### 4.4.4 Resampling Consideration

If the hardware default input device does not natively support 16 kHz, `cpal` will use the device's native sample rate and the `StreamConfig` will be adjusted.
In practice, most Linux audio systems (PulseAudio, PipeWire, ALSA) support 16 kHz natively. If the native rate differs, the `StreamConfig.sample_rate` should be
set to the device's native rate and a post-capture resampling step should be applied.

For the MVP, the service assumes the device supports 16 kHz. If `device.default_input_config()` reports a different sample rate, the service logs a warning and
falls back to the native rate, followed by a linear resampling step:

```rust
/// Resamples a PCM buffer from one sample rate to another using linear interpolation.
/// This is a simple, dependency-free resampler suitable for speech audio.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (samples.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let src_index = index as f64 / ratio;
        let src_low = src_index.floor() as usize;
        let src_high = (src_low + 1).min(samples.len() - 1);
        let fraction = src_index - src_low as f64;
        let interpolated = samples[src_low] as f64 * (1.0 - fraction) + samples[src_high] as f64 * fraction;
        output.push(interpolated as f32);
    }
    output
}
```

#### 4.4.5 Mono Downmix

If the hardware device provides stereo or multi-channel audio, the channels must be downmixed to mono before passing to Whisper. The `cpal` callback delivers
interleaved samples for multi-channel streams. The downmix averages all channels for each frame:

```rust
/// Downmixes interleaved multi-channel PCM to mono by averaging all channels per frame.
fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    let channels = channels as usize;
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
```

#### 4.4.6 Silence Detection Algorithm

The silence detection uses a simple absolute-amplitude threshold on each incoming sample. A sample is considered "silent" if its absolute value is below
`SILENCE_RMS_THRESHOLD` (0.01 by default, approximately -40 dBFS). The counter of consecutive silent samples is reset to zero whenever a non-silent sample is
encountered. When the consecutive silent sample count exceeds `silence_threshold_seconds * audio_sample_rate`, the recording auto-stops.

This approach is intentionally lightweight — it runs inside the real-time audio callback and must not introduce latency. More sophisticated algorithms (e.g.,
rolling RMS over a window, spectral analysis) can be added in future phases if the simple threshold proves insufficient for noisy environments.

#### 4.4.7 Thread Safety

- The `cpal::Stream` is created, played, and dropped on the dedicated capture thread. It is never sent across threads.
- The shared buffer (`Arc<Mutex<Vec<f32>>>`) is accessed from the audio callback (writer) and the outer loop (reader). The `Mutex` is held for very short
  durations (push a few samples / check length), minimizing lock contention.
- The `oneshot` channel for the stop signal is `Send`-safe and can be triggered from the async runtime.
- The `oneshot` channel for the result (`done_tx` / `done_rx`) bridges the capture thread back to the async caller.

### 4.5 Speech-to-Text (`transcriber.rs`)

STT uses `whisper-rs` 0.16 (bindings for `whisper.cpp`) to transcribe the PCM audio buffer into text. The Whisper context is loaded once at service
initialization and reused for all transcriptions. Each transcription creates a fresh `WhisperState` from the shared context — this is the pattern recommended by
`whisper-rs` for thread-safe reuse.

#### 4.5.1 Design Constraints

- **Context reuse:** `WhisperContext` is expensive to create (it loads the model into memory). It is created once at service startup and stored in
  `Arc<WhisperContext>`. Each transcription call creates a new `WhisperState` via `ctx.create_state()`, which is cheap.
- **Thread safety:** `WhisperContext` is `Send` and can be shared across threads via `Arc`. `WhisperState` is not `Sync` — each transcription must create its
  own state and run on the calling thread. The `transcribe` function is synchronous and called from a `tokio::spawn_blocking` task to avoid blocking the async
  runtime.
- **Language:** The `language` parameter is passed to `FullParams::set_language`. Whisper supports auto-detection (`None`) or explicit language codes (`"de"`,
  `"en"`, `"fr"`, etc.). The service uses the configured language by default.
- **Translation:** `FullParams::set_translate(false)` ensures Whisper transcribes in the source language rather than translating to English.
- **Sampling strategy:** `SamplingStrategy::Greedy { n_past: 0 }` is used for deterministic, low-latency transcription. Beam search can be enabled for higher
  accuracy at the cost of latency.
- **Output:** The transcription result is the concatenation of all segment texts, trimmed and joined by spaces.

#### 4.5.2 Error Types

```rust
/// Errors that can occur during speech-to-text transcription.
#[derive(Debug, thiserror::Error)]
pub enum SttError {
    /// Failed to create a Whisper state from the context.
    #[error("Failed to create Whisper state: {0}")]
    StateCreation(String),
    /// Failed to run the Whisper model on the audio data.
    #[error("Failed to run model: {0}")]
    ModelRun(String),
    /// Failed to retrieve a segment from the Whisper state.
    #[error("Failed to get segment: {0}")]
    SegmentRetrieval(String),
    /// The audio buffer is empty or too short for transcription.
    #[error("Audio buffer is too short: {0} samples")]
    BufferTooShort(usize),
    /// The Whisper context has not been initialized.
    #[error("Whisper context not initialized")]
    ContextNotInitialized,
}
```

#### 4.5.3 Whisper Context Loading

The Whisper context is loaded once at service initialization. The model path is validated and the context is wrapped in `Arc` for shared ownership.

```rust
use std::sync::Arc;
use whisper_rs::FullParams;
use whisper_rs::SamplingStrategy;
use whisper_rs::WhisperContext;
use whisper_rs::WhisperContextParameters;
use tracing::debug;

/// Loads the Whisper model from the configured path and returns a shared context.
pub fn load_whisper_context(model_path: &str) -> Result<Arc<WhisperContext>, SttError> {
    debug!("Loading Whisper model from: {}", model_path);
    let context = WhisperContext::new_with_params(
        model_path,
        WhisperContextParameters::default(),
    ).map_err(|error| SttError::StateCreation(error.to_string()))?;
    debug!("Whisper model loaded successfully");
    Ok(Arc::new(context))
}
```

#### 4.5.4 Transcription Implementation

```rust
/// Transcribes a PCM audio buffer into text using the Whisper model.
///
/// The audio buffer must contain 32-bit float samples at 16 kHz mono.
/// The `whisper_context` must have been loaded via `load_whisper_context`.
/// The `language` parameter sets the Whisper language code (e.g., "de", "en").
///
/// This function is synchronous and CPU-bound. It should be called from
/// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
pub fn transcribe(
    whisper_context: &WhisperContext,
    samples: &[f32],
    language: &str,
) -> Result<String, SttError> {
    if samples.len() < 1600 {
        // Less than 0.1 seconds of audio is too short for meaningful transcription.
        return Err(SttError::BufferTooShort(samples.len()));
    }

    // Create a fresh state for this transcription.
    // WhisperState is cheap to create from an existing WhisperContext.
    let mut state = whisper_context
        .create_state()
        .map_err(|error| SttError::StateCreation(error.to_string()))?;

    // Build the transcription parameters.
    let mut params = FullParams::new(SamplingStrategy::Greedy { n_past: 0 });
    params.set_language(Some(language));
    params.set_translate(false);
    // Disable progress callbacks for non-interactive use.
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    debug!(
        "Starting Whisper transcription: {} samples ({}s), language={}",
        samples.len(),
        samples.len() / 16000,
        language
    );

    // Run the model.
    state
        .full(params, samples)
        .map_err(|error| SttError::ModelRun(error.to_string()))?;

    // Extract and concatenate segment texts.
    let mut transcript = String::new();
    for segment in state.as_iter() {
        let segment_text = segment
            .to_str()
            .map_err(|error| SttError::SegmentRetrieval(error.to_string()))?;
        if !transcript.is_empty() {
            transcript.push(' ');
        }
        transcript.push_str(segment_text.trim());
    }

    debug!("Whisper transcription complete: {} characters", transcript.len());
    Ok(transcript)
}
```

#### 4.5.5 Async Wrapper

The `transcribe` function is synchronous and CPU-bound (it runs the full Whisper inference). To avoid blocking the tokio async runtime, it is wrapped in an
async function that uses `spawn_blocking`:

```rust
/// Async wrapper for `transcribe`. Runs the synchronous Whisper inference
/// on a blocking thread pool to avoid stalling the async runtime.
pub async fn transcribe_async(
    whisper_context: Arc<WhisperContext>,
    samples: Vec<f32>,
    language: String,
) -> Result<String, SttError> {
    tokio::task::spawn_blocking(move || {
        transcribe(&whisper_context, &samples, &language)
    })
        .await
        .map_err(|join_error| SttError::ModelRun(format!("Blocking task failed: {join_error}")))?
}
```

#### 4.5.6 Integration with the Service Pipeline

The service calls `transcribe_async` from within the `run_pipeline` method. The Whisper context is loaded once at service startup and stored in the
`VoiceAssistantService` struct:

```rust
impl VoiceAssistantService {
    /// Initializes the Whisper context from the configured model path.
    pub fn init_whisper(&mut self) -> Result<(), SttError> {
        let context = load_whisper_context(&self.config.whisper_model_path)?;
        self.whisper_context = Some(context);
        Ok(())
    }
}
```

During the pipeline, after audio capture completes:

```rust
// In run_pipeline, after capture_audio returns:
let whisper_ctx = self .whisper_context
.as_ref()
.ok_or(SttError::ContextNotInitialized) ?
.clone();

let transcript = transcribe_async(whisper_ctx, samples, self .config.language.clone()).await?;
```

#### 4.5.7 Whisper Model Selection

The choice of Whisper model affects accuracy, latency, and memory usage. The service supports any GGML-format Whisper model. Recommended models:

| Model              | Size    | Relative Speed | Accuracy | Use Case                  |
|--------------------|---------|----------------|----------|---------------------------|
| `ggml-tiny.bin`    | ~75 MB  | Fastest        | Lowest   | MVP, low-resource systems |
| `ggml-tiny.en.bin` | ~75 MB  | Fastest        | Low      | English-only, fastest     |
| `ggml-base.bin`    | ~142 MB | Fast           | Medium   | Balanced default          |
| `ggml-small.bin`   | ~466 MB | Medium         | High     | High-accuracy, more CPU   |

For the MVP, `ggml-tiny.bin` is recommended. The model path is configurable in `services.toml`, so users can upgrade to a larger model without code changes.

#### 4.5.8 Performance Characteristics

- **Latency:** On a modern x86_64 CPU, `ggml-tiny.bin` transcribes 5 seconds of audio in approximately 1-2 seconds. `ggml-base.bin` takes 3-5 seconds for the
  same input.
- **Memory:** The Whisper context holds the model weights in memory. `ggml-tiny.bin` uses ~75 MB RAM. `ggml-base.bin` uses ~142 MB.
- **CPU:** Whisper inference is single-threaded by default. The `FullParams` can be configured to use multiple threads via `params.set_n_threads(n)`, but for
  small models the overhead of thread synchronization often outweighs the benefit.
- **Thread isolation:** The `transcribe_async` wrapper uses `tokio::task::spawn_blocking`, which runs the inference on a dedicated blocking thread from the
  tokio runtime's blocking pool. This ensures the async runtime remains responsive while Whisper runs.

### 4.6 LLM Inference (`llm.rs`)

LLM inference uses `llama-cpp-2` 0.1.151 (Rust bindings for `llama.cpp`) to run a quantized 1B–3B parameter model entirely on the CPU. The model is loaded once
at service initialization with the configured thread count and context window size. The `LlamaBackend` must be initialized before any model loading or context
creation — it is stored for the lifetime of the service.

#### 4.6.1 Design Constraints

- **Backend initialization:** `LlamaBackend::init()` must be called exactly once before any `llama-cpp-2` operations. The backend is stored in the
  `LlmInferenceEngine` and kept alive for the engine's entire lifetime.
- **Model loading:** `LlamaModel::load_from_file(&backend, path, &model_params)` loads the GGUF model. `LlamaModelParams` is `!Send` and `!Sync`, so it must be
  constructed and used on the same thread. The model itself is `Send + Sync` after loading.
- **Context creation:** `LlamaModel::new_context(&backend, ctx_params)` creates a `LlamaContext<'a>` borrowing the model. `LlamaContext` is `!Send` and
  `!Sync` — it must live on the same thread as the model. The context holds the KV cache and is the primary handle for inference.
- **Context parameters:** `LlamaContextParams` is `Send + Sync` and uses a builder pattern: `.with_n_ctx(NonZeroU32::new(2048))`, `.with_n_threads(4)`,
  `.with_n_threads_batch(4)`, `.with_n_batch(2048)`, `.with_n_seq_max(1)`.
- **Thread isolation:** Because `LlamaContext` is `!Send`, the entire inference pipeline (tokenize → batch → decode → sample → detokenize) must run on a single
  dedicated thread. The `generate` function is synchronous and called from `tokio::task::spawn_blocking`.
- **Chat template:** The model's built-in chat template is retrieved via `model.chat_template(None)` and applied with
  `model.apply_chat_template(&tmpl, &messages, true)`. This produces a prompt string formatted according to the model's expected format (ChatML, Llama-3,
  Mistral, etc.).
- **Sampling:** `LlamaSampler` provides composable sampling stages. The engine uses a chain of `LlamaSampler::temp(temperature)`, `LlamaSampler::top_k(40)`,
  `LlamaSampler::top_p(0.95, 1)`, and `LlamaSampler::greedy()` for deterministic, low-latency generation. The sampler is `!Send` and must be created on the
  inference thread.
- **Batching:** `LlamaBatch::new(n_tokens, n_seq_max)` holds tokens for decode. `batch.add(token, pos, &[seq_id], logits)` adds individual tokens.
  `batch.add_sequence(&tokens, seq_id, logits_all)` adds a full sequence. Only the last token needs `logits=true` for generation.
- **Generation loop:** The loop tokenizes the prompt, feeds the tokens as a batch, calls `ctx.decode(&mut batch)`, then repeatedly samples the next token,
  appends it to the batch, and decodes until an end-of-generation token is produced or `max_tokens` is reached.
- **End-of-generation detection:** `model.is_eog_token(token)` checks whether a token is an end-of-generation marker (EOS, EOT, etc.).

#### 4.6.2 Error Types

```rust
/// Errors that can occur during LLM inference.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Failed to initialize the llama.cpp backend.
    #[error("Failed to initialize backend: {0}")]
    BackendInit(String),
    /// Failed to load the model from the GGUF file.
    #[error("Failed to load model: {0}")]
    ModelLoad(String),
    /// Failed to create a context from the model.
    #[error("Failed to create context: {0}")]
    ContextCreate(String),
    /// Failed to retrieve the chat template from the model.
    #[error("Failed to get chat template: {0}")]
    ChatTemplate(String),
    /// Failed to apply the chat template to the messages.
    #[error("Failed to apply chat template: {0}")]
    ApplyChatTemplate(String),
    /// Failed to tokenize the input string.
    #[error("Failed to tokenize: {0}")]
    Tokenize(String),
    /// Failed to detokenize the output tokens.
    #[error("Failed to detokenize: {0}")]
    Detokenize(String),
    /// Failed to decode a batch of tokens.
    #[error("Failed to decode batch: {0}")]
    Decode(String),
    /// Failed to create a chat message.
    #[error("Failed to create chat message: {0}")]
    ChatMessage(String),
    /// The generation exceeded the maximum token limit without producing an EOS.
    #[error("Max tokens ({0}) reached")]
    MaxTokensReached(usize),
    /// The LLM engine has not been initialized.
    #[error("LLM engine not initialized")]
    NotInitialized,
}
```

#### 4.6.3 LLM Configuration

The LLM configuration is part of `VoiceAssistantServiceConfig`:

```rust
/// Configuration for the LLM inference engine.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Path to the GGUF model file (e.g., "models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf").
    pub model_path: String,
    /// Number of CPU threads for inference.
    pub n_threads: i32,
    /// Context window size in tokens.
    pub n_ctx: u32,
    /// Batch size for prompt processing.
    pub n_batch: u32,
    /// Maximum number of tokens to generate per response.
    pub max_tokens: usize,
    /// Sampling temperature (0.0 = greedy, 1.0 = creative).
    pub temperature: f32,
    /// Top-K sampling parameter.
    pub top_k: i32,
    /// Top-P (nucleus) sampling parameter.
    pub top_p: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_path: "models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf".to_string(),
            n_threads: 4,
            n_ctx: 2048,
            n_batch: 2048,
            max_tokens: 256,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.95,
        }
    }
}
```

#### 4.6.4 LLM Inference Engine

```rust
use std::num::NonZeroU32;
use std::sync::Arc;
use encoding_rs::Decoder;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaChatMessage;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::Special;
use llama_cpp_2::sampling::LlamaSampler;
use tracing::debug;

/// LLM inference engine wrapping a llama.cpp model and context.
pub struct LlmInferenceEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    config: LlmConfig,
}

impl LlmInferenceEngine {
    /// Loads the LLM model from the configured GGUF file.
    pub fn load(config: &LlmConfig) -> Result<Self, LlmError> {
        let backend = LlamaBackend::init()
            .map_err(|error| LlmError::BackendInit(error.to_string()))?;
        debug!("LLM: backend initialized");

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(
            &backend,
            &config.model_path,
            &model_params,
        ).map_err(|error| LlmError::ModelLoad(error.to_string()))?;
        debug!("LLM: model loaded from {}", config.model_path);

        Ok(Self {
            backend,
            model,
            config: config.clone(),
        })
    }

    /// Generates a completion from a system prompt and conversation history.
    ///
    /// This function is synchronous and CPU-bound. It should be called from
    /// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
    pub fn generate(
        &self,
        system_prompt: &str,
        conversation: &[LlamaChatMessage],
    ) -> Result<String, LlmError> {
        // 1. Apply the model's chat template to produce a formatted prompt.
        let chat_template = self.model
            .chat_template(None)
            .map_err(|error| LlmError::ChatTemplate(error.to_string()))?;

        let mut all_messages = vec![
            LlamaChatMessage::new("system".to_string(), system_prompt.to_string())
                .map_err(|error| LlmError::ChatMessage(error.to_string()))?,
        ];
        all_messages.extend(conversation.iter().cloned());

        let prompt = self.model
            .apply_chat_template(&chat_template, &all_messages, true)
            .map_err(|error| LlmError::ApplyChatTemplate(error.to_string()))?;
        debug!("LLM: formatted prompt ({} chars)", prompt.len());

        // 2. Tokenize the prompt.
        let prompt_tokens = self.model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|error| LlmError::Tokenize(error.to_string()))?;
        debug!("LLM: tokenized to {} tokens", prompt_tokens.len());

        // 3. Create a context for inference.
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(self.config.n_ctx))
            .with_n_batch(self.config.n_batch)
            .with_n_threads(self.config.n_threads)
            .with_n_threads_batch(self.config.n_threads)
            .with_n_seq_max(1);

        let mut ctx = self.model
            .new_context(&self.backend, ctx_params)
            .map_err(|error| LlmError::ContextCreate(error.to_string()))?;
        debug!("LLM: context created (n_ctx={})", ctx.n_ctx());

        // 4. Feed the prompt tokens as a batch.
        let batch_size = prompt_tokens.len();
        let mut batch = LlamaBatch::new(batch_size, 1);
        batch
            .add_sequence(&prompt_tokens, 0, false)
            .map_err(|error| LlmError::Decode(error.to_string()))?;

        // Decode the prompt batch.
        ctx.decode(&mut batch)
            .map_err(|error| LlmError::Decode(error.to_string()))?;
        debug!("LLM: prompt batch decoded");

        // 5. Build the sampler chain.
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::top_k(self.config.top_k),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::greedy(),
        ]);

        // 6. Autoregressive generation loop.
        let mut generated_tokens: Vec<LlamaToken> = Vec::with_capacity(self.config.max_tokens);
        let mut n_cur = batch_size as i32;

        for _ in 0..self.config.max_tokens {
            // Sample the next token from the last decoded position.
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            // Check for end-of-generation.
            if self.model.is_eog_token(token) {
                debug!("LLM: end-of-generation token produced");
                break;
            }

            generated_tokens.push(token);

            // Feed the new token back into the context.
            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|error| LlmError::Decode(error.to_string()))?;

            ctx.decode(&mut batch)
                .map_err(|error| LlmError::Decode(error.to_string()))?;

            n_cur += 1;
        }

        if generated_tokens.len() >= self.config.max_tokens {
            return Err(LlmError::MaxTokensReached(self.config.max_tokens));
        }

        // 7. Detokenize the generated tokens.
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        for token in &generated_tokens {
            let token_str = self.model
                .token_to_piece(token, &mut decoder, false, None)
                .map_err(|error| LlmError::Detokenize(error.to_string()))?;
            output.push_str(&token_str);
        }

        debug!("LLM: generated {} tokens, {} chars", generated_tokens.len(), output.len());
        Ok(output)
    }
}
```

#### 4.6.5 Async Wrapper

The `generate` function is synchronous and CPU-bound. To avoid blocking the tokio async runtime, it is wrapped in an async function that uses `spawn_blocking`:

```rust
/// Async wrapper for `generate`. Runs the synchronous LLM inference
/// on a blocking thread pool to avoid stalling the async runtime.
pub async fn generate_async(
    engine: Arc<LlmInferenceEngine>,
    system_prompt: String,
    conversation: Vec<LlamaChatMessage>,
) -> Result<String, LlmError> {
    tokio::task::spawn_blocking(move || {
        engine.generate(&system_prompt, &conversation)
    })
        .await
        .map_err(|join_error| LlmError::Decode(format!("Blocking task failed: {join_error}")))?
}
```

#### 4.6.6 Integration with the Service Pipeline

The engine is loaded once at service startup and stored as `Arc<LlmInferenceEngine>` in the `VoiceAssistantService`:

```rust
impl VoiceAssistantService {
    /// Initializes the LLM engine from the configured model path.
    pub fn init_llm(&mut self) -> Result<(), LlmError> {
        let engine = LlmInferenceEngine::load(&self.config.llm)?;
        self.llm_engine = Some(Arc::new(engine));
        Ok(())
    }
}
```

During the ReAct loop, the service calls `generate_async` with the system prompt (including the tool catalog) and the conversation history:

```rust
// In run_pipeline, after transcription:
let engine = self .llm_engine
.as_ref()
.ok_or(LlmError::NotInitialized) ?
.clone();

let response = generate_async(
engine,
self .build_system_prompt(),
conversation.clone(),
).await?;
```

#### 4.6.7 Chat Template Handling

The `llama-cpp-2` crate provides built-in chat template support via `LlamaModel::chat_template(None)` which retrieves the template baked into the model's GGUF
metadata. This is the preferred mechanism — using the wrong chat template can result in unexpected responses.

The `apply_chat_template` method takes a slice of `LlamaChatMessage` structs and produces a formatted prompt string. Setting `add_ass=true` ensures the prompt
ends with the assistant's opening tag, so the model continues directly from there.

```rust
// Building chat messages for the LLM:
let messages = vec![
    LlamaChatMessage::new("system".to_string(), system_prompt)
        .map_err(|error| LlmError::ChatMessage(error.to_string()))?,
    LlamaChatMessage::new("user".to_string(), user_input)
        .map_err(|error| LlmError::ChatMessage(error.to_string()))?,
    // Previous assistant responses and tool results are appended as the conversation grows.
];
```

If a model does not have a baked-in chat template, a named template can be specified explicitly:

```rust
let chat_template = LlamaChatTemplate::new("chatml")
.map_err( | error| LlmError::ChatTemplate(error.to_string())) ?;
```

#### 4.6.8 Sampling Strategy

The engine uses a composable sampler chain via `LlamaSampler::chain_simple`:

1. **`LlamaSampler::temp(temperature)`** — Scales logits by `1/temperature`. Lower values make the distribution sharper (more deterministic). `temperature=0.0`
   is equivalent to greedy.
2. **`LlamaSampler::top_k(40)`** — Keeps only the top 40 most likely tokens, filtering out the long tail.
3. **`LlamaSampler::top_p(0.95, 1)`** — Nucleus sampling: keeps tokens whose cumulative probability exceeds 0.95.
4. **`LlamaSampler::greedy()`** — Selects the token with the highest logit after the above filters.

For tool-calling scenarios where structured JSON output is required, a grammar sampler can be added to the chain:

```rust
let grammar = LlamaSampler::grammar(
& self .model,
r#"root ::= "{" "\"tool\":" "\"" [a-z]+ "\"" "," "\"arguments\":" "{" .* "}" "}""#,
"root",
).map_err( | error| LlmError::ChatTemplate(error.to_string())) ?;

let mut sampler = LlamaSampler::chain_simple([
LlamaSampler::temp( self .config.temperature),
LlamaSampler::top_k( self .config.top_k),
LlamaSampler::top_p( self .config.top_p, 1),
grammar,
LlamaSampler::greedy(),
]);
```

This constrains the model to produce valid JSON matching the grammar, ensuring tool calls can be parsed reliably.

#### 4.6.9 Tokenization and Detokenization

- **Tokenization:** `model.str_to_token(&prompt, AddBos::Always)` converts a string to a `Vec<LlamaToken>`. The `AddBos::Always` flag prepends the
  beginning-of-stream token.
- **Detokenization:** `model.token_to_piece(token, &mut decoder, false, None)` converts a single token to a string. A stateful `encoding_rs::Decoder` is used
  because tokens from language models may not always map to full UTF-8 characters — stateful decoding prevents loss of partial characters.
- **Special tokens:** The `false` parameter in `token_to_piece` disables decoding of special tokens (e.g., `<|im_start|>`) in the output, keeping the generated
  text clean.

#### 4.6.10 Model Selection

The choice of LLM model affects quality, latency, and memory usage. The service supports any GGUF-format model. Recommended models for CPU inference:

| Model                        | Size    | Params | Relative Speed | Quality | Use Case                        |
|------------------------------|---------|--------|----------------|---------|---------------------------------|
| Qwen2.5-0.5B-Instruct-Q4_K_M | ~400 MB | 0.5B   | Fastest        | Low     | MVP, minimal hardware           |
| Qwen2.5-1.5B-Instruct-Q4_K_M | ~1.1 GB | 1.5B   | Fast           | Medium  | Balanced default                |
| Qwen2.5-3B-Instruct-Q4_K_M   | ~2.0 GB | 3B     | Medium         | High    | High-quality, more CPU          |
| Llama-3.2-1B-Instruct-Q4_K_M | ~1.3 GB | 1B     | Fast           | Medium  | Alternative balanced option     |
| Llama-3.2-3B-Instruct-Q4_K_M | ~2.0 GB | 3B     | Medium         | High    | Alternative high-quality option |

For the MVP, `Qwen2.5-1.5B-Instruct-Q4_K_M` is recommended. It offers a good balance of quality, speed, and memory usage on CPU. The model path is configurable
in `services.toml`.

#### 4.6.11 Performance Characteristics

- **Latency:** On a modern 4-core x86_64 CPU, `Qwen2.5-1.5B-Instruct-Q4_K_M` generates approximately 10-20 tokens/second. A 256-token response takes 13-26
  seconds. `Qwen2.5-0.5B` generates 30-50 tokens/second.
- **Memory:** The model weights are loaded into RAM. `Qwen2.5-1.5B-Q4_K_M` uses ~1.1 GB RAM. The KV cache adds ~128 MB for a 2048-token context window.
- **CPU:** Inference uses the configured number of threads (`n_threads`). More threads improve prompt processing throughput but have diminishing returns for
  token generation. Setting `n_threads` to the number of physical cores is recommended.
- **Thread isolation:** The `generate_async` wrapper uses `tokio::task::spawn_blocking`, which runs the inference on a dedicated blocking thread from the tokio
  runtime's blocking pool. This ensures the async runtime remains responsive while the LLM runs.
- **Context window:** The `n_ctx` parameter limits the total number of tokens (prompt + generated) that can be processed. 2048 is sufficient for short voice
  commands and tool calls. Larger contexts increase memory usage and slow down inference.

#### 4.6.12 Thread Safety Summary

| Type                 | Send | Sync | Notes                                                    |
|----------------------|------|------|----------------------------------------------------------|
| `LlamaBackend`       | Yes  | Yes  | Must be initialized before any model/context operations. |
| `LlamaModel`         | Yes  | Yes  | Shared via `Arc` after loading.                          |
| `LlamaContext`       | No   | No   | Must live on the thread that created it.                 |
| `LlamaModelParams`   | No   | No   | Constructed and used on the same thread.                 |
| `LlamaContextParams` | Yes  | Yes  | Builder pattern, can be constructed anywhere.            |
| `LlamaBatch`         | No   | No   | Created and used on the inference thread.                |
| `LlamaSampler`       | No   | No   | Created and used on the inference thread.                |
| `LlamaChatMessage`   | Yes  | Yes  | Can be constructed from the async runtime.               |

Because `LlamaContext`, `LlamaBatch`, and `LlamaSampler` are `!Send`, the entire inference pipeline runs inside a single `spawn_blocking` call. The
`LlmInferenceEngine` itself is `Send + Sync` (via `LlamaBackend` and `LlamaModel`), so it can be shared as `Arc<LlmInferenceEngine>` across async tasks.

### 4.7 Tool Catalog Management (`tool_catalog.rs`)

The service subscribes to `mcp.register.tool` to build and maintain a live catalog of all registered tools. As plugins register their tools during
initialization, the catalog grows automatically. The catalog is used to build the LLM system prompt.

```rust
impl VoiceAssistantService {
    /// Handles a new tool registration from the broker.
    pub fn on_tool_registered(&self, message: RegisterToolMessage) {
        let entry = ToolCatalogEntry {
            name: message.name.to_string(),
            description: message.description.to_string(),
            input_schema: message.input_schema.to_string(),
        };
        self.tool_catalog.write().unwrap_or_default().push(entry);
    }

    /// Builds the system prompt for the LLM, injecting the tool catalog.
    pub fn build_system_prompt(&self) -> String {
        let catalog = self.tool_catalog.read().unwrap_or_default();
        let tools_json: Vec<serde_json::Value> = catalog
            .iter()
            .map(|t| serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": serde_json::from_str::<serde_json::Value>(&t.input_schema)
                    .unwrap_or(serde_json::Value::Null),
            }))
            .collect();

        format!(
            "You are a desktop assistant for the Smearor Swipe Launcher. \
            You control the system via tool calls. \
            Available tools: {tools_json}. \
            Respond in JSON format. \
            To call a tool, output: {{\"tool\": \"<name>\", \"arguments\": {{...}}}}. \
            To give a final answer, output: {{\"final_answer\": \"<text>\"}}. \
            Be concise and efficient. Prefer single tool calls when possible."
        )
    }
}
```

### 4.8 ReAct Loop (`react.rs`)

The ReAct loop orchestrates the multi-step reasoning process. Each iteration:

1. Builds the system prompt with the current tool catalog.
2. Feeds the conversation history to the LLM.
3. Parses the LLM output as either a tool call or a final answer.
4. If tool call: invokes the tool via `mcp.invoke.tool`, waits for `mcp.tool.response`, and appends the result to the conversation.
5. If final answer: broadcasts the status and exits the loop.
6. Safety limit: the loop terminates after `max_react_iterations`.

```rust
impl VoiceAssistantService {
    /// Executes the ReAct loop for a given user text input.
    pub async fn execute_react_loop(
        &self,
        user_text: &str,
    ) -> Result<String, AssistantError> {
        let system_prompt = self.build_system_prompt();
        let mut conversation = vec![user_text.to_string()];

        for iteration in 0..self.config.max_react_iterations {
            self.set_state(AssistantState::ThinkingLlm).await;

            let llm_output = self.llm_engine
                .generate(&system_prompt, &conversation)
                .await
                .map_err(AssistantError::LlmInference)?;

            match parse_llm_response(&llm_output) {
                LlmResponse::ToolCall { tool, arguments } => {
                    self.set_state(AssistantState::ExecutingAction).await;
                    let tool_result = self.invoke_tool(&tool, &arguments).await?;
                    conversation.push(format!("Tool {tool} result: {tool_result}"));
                }
                LlmResponse::FinalAnswer { answer } => {
                    return Ok(answer);
                }
            }
        }

        Err(AssistantError::MaxIterationsReached)
    }

    /// Invokes a tool via the MCP tool registry and waits for the response.
    async fn invoke_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, AssistantError> {
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let invoke_message = InvokeToolMessage::new(
            &correlation_id,
            tool_name,
            &arguments.to_string(),
        );

        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(invoke_message);

        // Wait for the response on mcp.tool.response with matching correlation_id.
        // The response is received via MessageHandler<FfiEnvelopePayload<InvokeToolResponse>>.
        // A pending-response tracker matches by correlation_id.
        self.wait_for_tool_response(&correlation_id).await
    }
}
```

### 4.9 MCP Integration (`mcp.rs`)

The service registers its own MCP resources and tools so that external MCP clients can query the assistant state and trigger voice commands programmatically.

**MCP Resources:**

| URI                              | Description                                            | Source type              |
|----------------------------------|--------------------------------------------------------|--------------------------|
| `voice_assistant://status`       | Current assistant state, transcript, and final answer. | `AssistantStatusMessage` |
| `voice_assistant://tool_catalog` | List of all discovered tools in the catalog.           | JSON array               |

**MCP Tools:**

| Tool                          | Description                                         | Parameters     |
|-------------------------------|-----------------------------------------------------|----------------|
| `voice_assistant_activate`    | Starts audio capture and begins the voice pipeline. | -              |
| `voice_assistant_deactivate`  | Stops audio capture and cancels the pipeline.       | -              |
| `voice_assistant_submit_text` | Submits a text command directly (bypassing STT).    | `text: string` |

> **MCP tool naming convention:** Tool names use `snake_case` with underscores, never dots. This is consistent with existing tools like `sysinfo_refresh` and
`weather_refresh`.

### 4.10 Background Pipeline

On activation (via widget click or `voice_assistant_activate` tool), the service executes the following pipeline:

1. **Set state to `Listening`** and broadcast status.
2. **Capture audio** via `cpal` until silence is detected or max duration is reached.
3. **Set state to `ProcessingStt`** and broadcast status.
4. **Transcribe audio** via `whisper-rs` to get the user's text intent.
5. **Execute ReAct loop** with the transcribed text and the live tool catalog.
6. **Broadcast final status** with the LLM's final answer (or error).

```rust
impl VoiceAssistantService {
    /// Runs the complete voice pipeline: capture -> STT -> ReAct -> status.
    pub async fn run_pipeline(&self) {
        // 1. Capture audio
        self.set_state(AssistantState::Listening).await;
        let samples = match capture_audio(&self.config).await {
            Ok(s) => s,
            Err(e) => {
                self.set_error(&e.to_string()).await;
                return;
            }
        };

        // 2. Transcribe
        self.set_state(AssistantState::ProcessingStt).await;
        let whisper_ctx = match &self.whisper_context {
            Some(ctx) => ctx,
            None => {
                self.set_error("Whisper context not initialized").await;
                return;
            }
        };
        let transcript = match transcribe(whisper_ctx, &samples, &self.config.language) {
            Ok(t) => t,
            Err(e) => {
                self.set_error(&e.to_string()).await;
                return;
            }
        };

        // 3. ReAct loop
        match self.execute_react_loop(&transcript).await {
            Ok(answer) => {
                self.set_final_answer(&answer).await;
            }
            Err(e) => {
                self.set_error(&e.to_string()).await;
            }
        }
    }
}
```

### 4.11 Error Handling

All errors are handled gracefully and broadcast as status messages with `AssistantState::Error`. The service never panics. Error types use `thiserror` for
internal errors and `miette` for user-facing diagnostics.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    #[error("Audio capture failed: {0}")]
    Audio(String),
    #[error("Speech-to-text failed: {0}")]
    Stt(String),
    #[error("LLM inference failed: {0}")]
    LlmInference(String),
    #[error("Tool invocation failed: {0}")]
    ToolInvocation(String),
    #[error("Tool response timeout for correlation_id: {0}")]
    ToolTimeout(String),
    #[error("Max ReAct iterations reached without final answer")]
    MaxIterationsReached,
    #[error("LLM output could not be parsed: {0}")]
    Parse(String),
}
```

---

## 5. Widget Crate (`plugins/voice_assistant`)

The user interface component is a standard touch-optimized tile widget built purely using GTK4 bindings. It provides continuous visual feedback on voice
processing phases and maps direct click handlers to activate or cancel the voice pipeline.

### 5.1 File Structure

- `widget.rs` - `VoiceAssistantWidget` struct and trait implementations
- `config.rs` - `VoiceAssistantWidgetConfig` struct and parsing
- `views.rs` - View rendering functions for each state
- `lib.rs` - `widget_plugin!` macro invocation

### 5.2 Widget Configuration

```rust
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
    /// Background color of the widget.
    pub background_color: Option<String>,
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
    /// Message topic for single-click (defaults to service.voice_assistant.command).
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,
    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,
    /// Message payload for long-press.
    #[serde(default)]
    pub longpress_payload: Option<Value>,
}

impl Default for VoiceAssistantWidgetConfig {
    fn default() -> Self {
        Self {
            width: 120,
            height: 80,
            spacing: 4,
            background_color: None,
            show_icon: true,
            show_transcript: true,
            show_final_answer: true,
            icon_idle: "nf-md-microphone_off".to_string(),
            icon_listening: "nf-md-microphone".to_string(),
            icon_processing: "nf-md-waveform".to_string(),
            icon_thinking: "nf-md-brain".to_string(),
            icon_executing: "nf-md-cog_play".to_string(),
            icon_error: "nf-md-alert_circle".to_string(),
            click_topic: Some("service.voice_assistant.command".to_string()),
            click_payload: Some(serde_json::json!({"action": "Activate"})),
            longpress_topic: Some("service.voice_assistant.command".to_string()),
            longpress_payload: Some(serde_json::json!({"action": "Deactivate"})),
        }
    }
}
```

### 5.3 Widget Implementation

```rust
pub struct VoiceAssistantWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: VoiceAssistantWidgetConfig,
    pub current_state: AssistantState,
    pub current_status: Option<AssistantStatusMessage>,
}
```

> **GTK widget references:** GTK4 widgets (`gtk4::Box`, `gtk4::Image`, `gtk4::Label`) are **not** `Send` or `Sync`. They must not be stored in
`Arc<RwLock<...>>` inside the plugin struct. Instead, widget references are captured inside `glib::clone!` closures or passed directly to
`glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state (`config`, `current_state`, `current_status`).

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<AssistantStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 5.4 View Rendering

Each state renders an icon and a text label into the widget container. Rendering is done via `glib::MainContext::spawn_local` to ensure GTK thread safety.

| State             | Icon                     | Primary Label       | Secondary Label    |
|-------------------|--------------------------|---------------------|--------------------|
| `Idle`            | `icon_idle`              | -                   | -                  |
| `Listening`       | `icon_listening` (pulse) | "Listening..."      | Partial transcript |
| `ProcessingStt`   | `icon_processing`        | "Transcribing..."   | -                  |
| `ThinkingLlm`     | `icon_thinking` (spin)   | "Thinking..."       | -                  |
| `ExecutingAction` | `icon_executing` (spin)  | "Executing: {tool}" | -                  |
| `Error`           | `icon_error`             | "Error"             | Error message      |

### 5.5 Click and Long-Press Actions

- **Single click**: Publishes the configured `click_topic` / `click_payload` to toggle the voice pipeline. By default, this sends `VoiceCommandAction::Activate`
  to `service.voice_assistant.command`.
- **Long-press**: Publishes the configured `longpress_topic` / `longpress_payload`. By default, this sends `VoiceCommandAction::Deactivate` to cancel the
  pipeline.

### 5.6 State Synchronization

The widget subscribes to `service.voice_assistant.status`. When a new `AssistantStatusMessage` arrives:

1. The message is deserialized and stored in `current_status`.
2. The current state is updated and the view is re-rendered.
3. All GTK updates happen via `glib::MainContext::spawn_local`.

```rust
impl MessageHandler<FfiEnvelopePayload<AssistantStatusMessage>> for VoiceAssistantWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<AssistantStatusMessage>, _sender_id: &str) {
        let status = message.0;
        let state = status.current_state.clone();
        let transcript = status.partial_transcript.to_string();
        let final_answer = status.final_answer.as_ref().map(|s| s.to_string());
        let error = status.error_message.as_ref().map(|s| s.to_string());

        glib::MainContext::default().spawn_local(glib::clone!(
            @weak self.icon_widget as icon,
            @weak self.label_primary as primary,
            @weak self.label_secondary as secondary,
            @weak self.spinner as spinner => move || {
                match state {
                    AssistantState::Listening => {
                        icon.set_icon_name(Some("nf-md-microphone"));
                        primary.set_text("Listening...");
                        secondary.set_text(&transcript);
                        spinner.set_spinning(true);
                    }
                    AssistantState::ThinkingLlm => {
                        icon.set_icon_name(Some("nf-md-brain"));
                        primary.set_text("Thinking...");
                        spinner.set_spinning(true);
                    }
                    AssistantState::ExecutingAction => {
                        icon.set_icon_name(Some("nf-md-cog_play"));
                        primary.set_text("Executing...");
                        spinner.set_spinning(true);
                    }
                    AssistantState::Error => {
                        icon.set_icon_name(Some("nf-md-alert_circle"));
                        primary.set_text("Error");
                        secondary.set_text(&error.unwrap_or_default());
                        spinner.set_spinning(false);
                    }
                    AssistantState::Idle => {
                        icon.set_icon_name(Some("nf-md-microphone_off"));
                        if let Some(answer) = final_answer {
                            primary.set_text(&answer);
                        } else {
                            primary.set_text("");
                        }
                        spinner.set_spinning(false);
                    }
                    _ => {
                        spinner.set_spinning(false);
                    }
                }
            }
        ));
    }
}
```

---

## 6. Message Flow

```
+-----------------------+         +-----------------------+         +-----------------------+
| Voice Assistant       |<--------|                       |-------->| Voice Assistant       |
| Widget                |  Status |   Event Broker        | Command | Service (Singleton)   |
| (tile in scroll band) | Broadcast                     Broadcast +-----------------------+
+---------+-------------+         +-----------------------+         |                       |
          |                                                       | [cpal] Audio Capture  |
          | Click: Activate                                        | [whisper-rs] STT      |
          | Longpress: Deactivate                                  | [llama-cpp-2] LLM     |
          v                                                       v                       |
+-----------------------+                               +-----------------------+   |
| View update           |                               | mcp.register.tool     |   |
| (local state)         |                               | (tool catalog)       |   |
+-----------------------+                               +-----------------------+   |
                                                        | mcp.invoke.tool      |   |
                                                        | mcp.tool.response    |   |
                                                        +-----------------------+   |
                                                                                    |
                                                                                    v
                                                                        +-----------------------+
                                                                        | Target Plugin         |
                                                                        | (e.g., weather,       |
                                                                        |  wallpaper, power)    |
                                                                        +-----------------------+
```

---

## 7. Configuration Example

### 7.1 Service Registration in `services.toml`

```toml
[[services]]
id = "voice_assistant"
path = "target/release/libsmearor_voice_assistant_service.so"

[voice_assistant]
whisper_model_path = "models/ggml-tiny.bin"
llm_model_path = "models/qwen2.5-1.5b-instruct-q4_k_m.gguf"
llm_threads = 4
llm_context_size = 2048
max_react_iterations = 8
llm_temperature = 0.1
audio_sample_rate = 16000
audio_channels = 1
max_recording_seconds = 30
silence_threshold_seconds = 1.5
language = "en"
auto_enable = false
```

### 7.2 Widget Configuration in `config.toml`

```toml
[[scroll_band.plugins]]
id = "voice_assistant_widget"
path = "target/release/libsmearor_voice_assistant_widget.so"

[voice_assistant_widget]
width = 120
height = 80
show_icon = true
show_transcript = true
show_final_answer = true
icon_idle = "nf-md-microphone_off"
icon_listening = "nf-md-microphone"
icon_processing = "nf-md-waveform"
icon_thinking = "nf-md-brain"
icon_executing = "nf-md-cog_play"
icon_error = "nf-md-alert_circle"

# Single click activates the voice pipeline
click_topic = "service.voice_assistant.command"
click_payload = { action = "Activate" }

# Long-press deactivates the voice pipeline
longpress_topic = "service.voice_assistant.command"
longpress_payload = { action = "Deactivate" }
```

### 7.3 German Language Configuration

```toml
[voice_assistant]
whisper_model_path = "models/ggml-tiny.bin"
llm_model_path = "models/qwen2.5-1.5b-instruct-q4_k_m.gguf"
language = "de"
llm_temperature = 0.1
max_react_iterations = 8
```

---

## 8. Dependencies

### 8.1 Model Crate

| Dependency | Purpose                       |
|------------|-------------------------------|
| `serde`    | Serialization/deserialization |
| `stabby`   | ABI-stable FFI types          |

### 8.2 Service Crate

| Dependency              | Purpose                              |
|-------------------------|--------------------------------------|
| `model/voice_assistant` | Shared types                         |
| `plugin-api`            | Plugin traits and FFI                |
| `model/mcp`             | MCP tool/resource messages           |
| `cpal`                  | Cross-platform audio capture         |
| `whisper-rs`            | Whisper speech-to-text bindings      |
| `llama-cpp-2`           | LLM inference on CPU (GGUF models)   |
| `tokio`                 | Async runtime                        |
| `tracing`               | Logging                              |
| `thiserror`             | Internal error types                 |
| `serde_json`            | JSON parsing for LLM I/O             |
| `uuid`                  | Correlation IDs for tool invocations |

### 8.3 Widget Crate

| Dependency              | Purpose               |
|-------------------------|-----------------------|
| `model/voice_assistant` | Shared types          |
| `plugin-api`            | Plugin traits and FFI |
| `gtk4`                  | GTK4 framework        |
| `glib`                  | GLib utilities        |

### 8.4 Model Files

Model files are **not** bundled in the crate. They must be downloaded separately and placed in the configured path:

| Model                         | Size    | Download URL                                                                                                  |
|-------------------------------|---------|---------------------------------------------------------------------------------------------------------------|
| `ggml-tiny.bin` (Whisper)     | ~75 MB  | `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin`                                     |
| `ggml-tiny.en.bin` (English)  | ~75 MB  | `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin`                                  |
| Qwen-2.5-1.5B-Instruct Q4_K_M | ~1 GB   | `https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf`       |
| Llama-3.2-1B-Instruct Q4_K_M  | ~800 MB | `https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct-GGUF/resolve/main/llama-3.2-1b-instruct-q4_k_m.gguf` |

> **Resource cap:** Models are quantized to 4-bit (`Q4_K_M`) to limit total memory footprint to < 1.5 GB RAM, protecting general launcher reactivity during
> inference tasks.

---

## 9. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Voice Assistant feature. The order is chosen so that each layer
is built on top of already-tested foundations.

### Phase 1: Foundation — Model Crate (`model/voice_assistant`)

**Goal:** Define all shared messages, topics, states, and configuration types.

**Order:**

1. Create the crate `model/voice_assistant` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_COMMAND` and `TOPIC_STATUS`.
3. Create one file per message struct:
    - `src/messages/state.rs` -> `AssistantState` enum
    - `src/messages/command.rs` -> `VoiceCommandAction` and `VoiceCommandMessage`
    - `src/messages/status.rs` -> `AssistantStatusMessage`
    - `src/messages/tool_catalog.rs` -> `ToolCatalogEntry`
    - `src/messages/llm_response.rs` -> `LlmResponse` enum
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.

---

### Phase 2: Backend — Service Crate (`services/voice_assistant`)

**Goal:** Implement the local AI pipeline (audio capture, STT, LLM ReAct loop) and in-process MCP tool discovery/invocation.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/voice_assistant` with a `Cargo.toml` that depends on the `model/voice_assistant` crate, the project plugin API, `model/mcp`,
   `cpal`, `whisper-rs`, `llama-cpp-2`, `tokio`, `tracing`, `thiserror`, `serde_json`, and `uuid`.
2. Create `src/config.rs` with `VoiceAssistantServiceConfig` and its default values.
3. Create `src/audio.rs` and implement `capture_audio`:
    - Enumerate input devices via `cpal`.
    - Build a 16kHz mono f32 stream.
    - Record into a `Vec<f32>` buffer.
    - Apply silence detection and max duration auto-stop.
4. Create `src/transcriber.rs` and implement `transcribe`:
    - Load the Whisper context from the configured model path.
    - Transcribe PCM samples to text.
    - Support configurable language.
5. Create `src/llm.rs` and implement `LlmInferenceEngine`:
    - Load the GGUF model via `llama-cpp-2`.
    - Implement `generate` with chat template formatting.
    - Configure thread count, context size, and temperature.
6. Create `src/tool_catalog.rs` and implement tool catalog management:
    - Subscribe to `mcp.register.tool` via `MessageHandler<FfiEnvelopePayload<RegisterToolMessage>>`.
    - Maintain a `Vec<ToolCatalogEntry>` in `Arc<RwLock<...>>`.
    - Build the system prompt with the tool catalog injected as JSON.
7. Create `src/react.rs` and implement the ReAct loop:
    - Parse LLM output as `LlmResponse` (ToolCall or FinalAnswer).
    - Invoke tools via `mcp.invoke.tool` with correlation IDs.
    - Wait for responses via `MessageHandler<FfiEnvelopePayload<InvokeToolResponse>>`.
    - Implement a pending-response tracker matching by correlation ID.
    - Safety limit: `max_react_iterations`.
8. Create `src/mcp.rs` and register MCP resources and tools:
    - `voice_assistant://status`, `voice_assistant://tool_catalog`.
    - `voice_assistant_activate`, `voice_assistant_deactivate`, `voice_assistant_submit_text`.
9. Create `src/service.rs` with `VoiceAssistantService` and all required trait implementations.
10. Implement `run_pipeline` to orchestrate: capture -> STT -> ReAct -> status.
11. Wire `service_plugin!` in `src/lib.rs`.
12. Add unit tests for LLM response parsing, tool catalog management, and silence detection.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Audio capture produces a valid PCM buffer from the microphone.
- Whisper transcription produces text from recorded audio.
- The LLM generates text from a system prompt and conversation.
- The ReAct loop can invoke a registered tool and process the response.
- Tool catalog is populated from `mcp.register.tool` messages.
- Status messages are broadcast on `TOPIC_STATUS` at each pipeline stage.
- No `unwrap`, `expect`, or `panic` in the implementation.

---

### Phase 3: Display — Widget Crate (`plugins/voice_assistant`)

**Goal:** Provide a compact voice assistant tile with state-based visual feedback and click/long-press actions.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/voice_assistant` with a `Cargo.toml` that depends on `model/voice_assistant`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with `VoiceAssistantWidgetConfig` including icons, dimensions, and click/longpress topics.
3. Create `src/views.rs` and implement a render function for each `AssistantState` variant:
    - `Idle`: muted microphone icon, no animation.
    - `Listening`: pulsing microphone icon, partial transcript.
    - `ProcessingStt`: waveform icon, "Transcribing..." label.
    - `ThinkingLlm`: brain icon, spinner, "Thinking..." label.
    - `ExecutingAction`: cog icon, spinner, "Executing: {tool}" label.
    - `Error`: alert icon, error message.
4. Create `src/widget.rs` with `VoiceAssistantWidget` and all required trait implementations.
5. Implement click handling: publish `VoiceCommandAction::Activate` to `TOPIC_COMMAND`.
6. Implement long-press handling: publish `VoiceCommandAction::Deactivate` to `TOPIC_COMMAND`.
7. Subscribe to `TOPIC_STATUS` and update `current_status` + re-render on every message.
8. Wire `widget_plugin!` in `src/lib.rs`.
9. Add an integration test that verifies the widget accepts `TOPIC_STATUS` and renders the correct state.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The widget displays the correct icon and label for each `AssistantState`.
- The widget updates its display when a new `AssistantStatusMessage` arrives.
- Click publishes `Activate` command to the broker.
- Long-press publishes `Deactivate` command to the broker.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/voice_assistant` and `services/voice_assistant` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `voice_assistant` in `config.toml`.
4. Add a sample widget configuration for the voice assistant widget.
5. Document model file download instructions in the README.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The voice assistant widget receives messages and renders correctly.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that `TOPIC_STATUS` appears on the message broker when the widget is clicked.
2. Verify the widget displays the correct state transitions: Idle -> Listening -> ProcessingStt -> ThinkingLlm -> ExecutingAction -> Idle.
3. Verify the ReAct loop can invoke at least one registered tool (e.g., `get_current_time`).
4. Verify multi-step tool chains work (e.g., "What time is it?" -> `get_current_time` -> final answer).
5. Verify error handling: simulate a missing model file and confirm the service broadcasts an error status without crashing.
6. Verify the `voice_assistant_activate` MCP tool triggers the pipeline.
7. Verify the `voice_assistant_submit_text` MCP tool bypasses STT and runs the ReAct loop directly.
8. Run `cargo test` for all three crates.
9. Run `cargo clippy` and `cargo fmt` and fix any issues.
10. Measure memory usage during inference to confirm the < 1.5 GB RAM target.

**Exit criteria:**

- All tests pass.
- The widget renders correctly for all states.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.
- Memory usage during inference stays below 1.5 GB.
- The ReAct loop successfully invokes at least one tool and produces a final answer.

---

### Summary of Order

```
Phase 1: model/voice_assistant
    |
    v
Phase 2: services/voice_assistant
    |
    v
Phase 3: plugins/voice_assistant
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

### Rationale

- **Model first:** Message formats and state definitions must exist before the service or widgets can use them.
- **Service second:** The widget needs a running publisher to test against. The service is the most complex crate (audio, STT, LLM, ReAct loop, MCP
  integration).
- **Widget third:** The display widget depends on the service's status topic.
- **Wiring fourth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 10. Technical Notes

- **In-process MCP client:** The voice assistant service does **not** spawn the MCP server as a subprocess. Instead, it uses the in-process message broker to
  discover tools (via `mcp.register.tool`) and invoke them (via `mcp.invoke.tool` / `mcp.tool.response`). This eliminates stdio overhead and provides
  near-zero-latency tool execution.
- **Tool catalog caching:** The service subscribes to `mcp.register.tool` at startup and maintains a live tool catalog. Tools registered by plugins after the
  voice assistant starts are automatically added to the catalog. The catalog is rebuilt into the system prompt on each ReAct iteration.
- **No polling in the widget:** The widget updates exclusively through incoming status messages. The service drives all state transitions.
- **CPU-only inference:** Both Whisper and the LLM run entirely on the CPU. No GPU or external API is required. The `llama-cpp-2` crate automatically uses CPU
  extensions like AVX2 or AVX-512 when available.
- **Resource cap:** Models are quantized to 4-bit (`Q4_K_M`) to limit total memory footprint to < 1.5 GB RAM, protecting general launcher reactivity during
  inference tasks.
- **Silence detection:** Audio capture auto-stops after `silence_threshold_seconds` of near-zero samples to avoid requiring the user to manually stop recording.
  The threshold is configurable.
- **ReAct safety limit:** The ReAct loop terminates after `max_react_iterations` to prevent infinite loops. The default is 8 iterations, which is sufficient for
  most multi-step tasks.
- **Language support:** Whisper supports multiple languages. The `language` config field sets the Whisper language and can be changed at any time via
  `services.toml`. The LLM model should be chosen to support the target language (Qwen-2.5 supports many languages including German).
- **Model file management:** Model files (`.bin` for Whisper, `.gguf` for LLM) are not bundled in the crate. They must be downloaded separately and placed in
  the configured path. The service validates file existence at startup and broadcasts an error status if files are missing.
- **GTK widget ownership:** GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget
  references are captured in `glib::clone!` closures or `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.
- **MCP tool naming:** Tool names use `snake_case` with underscores, never dots. This is consistent with existing tools (`sysinfo_refresh`, `weather_refresh`,
  `get_current_time`).
- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in `model/notifications`, `model/audio`, and `model/app-launcher`.
- **Correlation IDs:** Tool invocations use UUID-based correlation IDs to match `InvokeToolMessage` with `InvokeToolResponse`. A pending-response tracker stores
  pending IDs and resolves them when the matching response arrives. A timeout (10 seconds, matching the MCP server) prevents hanging.
- **Alternative: text input bypass:** The `voice_assistant_submit_text` MCP tool and `VoiceCommandAction::SubmitText` allow text-based commands without
  microphone input. This is useful for testing, accessibility, and integration with other input methods.

---

## 11. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/voice_assistant`, `services/voice_assistant`, and `plugins/voice_assistant`.
- **One struct per file:** Each message struct, enum, and configuration type lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops. The service's pipeline is triggered by command messages, not by a periodic
  timer.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String` and optional strings use
  `stabby::option::Option<stabby::string::String>` to maintain ABI stability across compiler invocations.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions. No abbreviations.
- **Documentation:** All public structs, enums, and fields are documented in English with rustdoc comments.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `cpal`, `whisper-rs`, `llama-cpp-2`, `tokio`, and `tracing`; the widget uses `gtk4`
  and `glib`.
- **Import organization:** Imports are one per line, alphabetically ordered, with `crate::` first, then external crates, then `std::`.
- **Error handling:** Internal errors use `thiserror`; user-facing errors use `miette`.

---

*End of document.*
