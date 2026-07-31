# Speak Swiper Output Concept: TTS Pipeline via ort & cpal

This document describes the concept for a **Text-to-Speech (TTS) output pipeline** for the Voice Assistant Service in the Smearor Swipe Launcher. The pipeline
converts LLM-generated text responses (`final_answer` and `clarify`) into natural-sounding speech using the existing `ort` (ONNX Runtime) and `cpal`
(Cross-Platform Audio Library) dependencies — without introducing any additional heavy frameworks.

---

## Executive Summary

**Goal:** Enable the voice assistant to speak responses back to the user after processing a voice command or text input.

**Approach:** Use the existing `ort` crate for ONNX model inference (Piper or Kokoro-82M) and the existing `cpal` crate for PCM audio output. No new heavy
dependencies are required.

**Key Constraints:**

- No additional ML frameworks (no `sherpa-onnx`, no `rodio`, no `tts` crate)
- Reuse existing `ort = "=2.0.0-rc.12"` for ONNX inference
- Reuse existing `cpal = "0.18.1"` for audio output
- Must fit within the 2 GB VRAM budget on the low-end system (Ryzen 5 8500G)
- Must leverage GPU acceleration on the high-end system (Ryzen 9 9950X3D + RX 7900 XTX)

---

## 1. System Architecture

### 1.1 Data Flow

```
+-----------+     +-------------------+     +-------------------------+     +--------------------+
| Text input| --> | Phonemizer/Tokens | --> |  ort (v2.0.0-rc.12)     | --> | cpal (v0.18.1)     |
|           |     | (Espeak / JSON)   |     |  Inference (CPU/ROCm)   |     | Audio Output Stream|
+-----------+     +-------------------+     +-------------------------+     +--------------------+
```

1. **Text Input**: The `final_answer` or `clarify` string from the ReAct loop
2. **Phonemizer**: Converts text into phoneme tokens (model-specific, see Section 3)
3. **ort Inference**: Runs the ONNX model to generate raw f32 PCM audio samples
4. **cpal Output**: Streams PCM samples directly to the audio output device

### 1.2 Integration Points

The TTS pipeline integrates into the existing voice assistant architecture via two distinct paths:

#### Path 1: Voice Input → Voice Output

The user speaks to the assistant. Audio is captured, transcribed via Whisper, processed through the ReAct loop, and the response is spoken back via TTS.

| Stage              | Component                          | Description                                   |
|--------------------|------------------------------------|-----------------------------------------------|
| **Input**          | `cpal` (microphone)                | Audio capture                                 |
| **STT**            | `whisper-rs`                       | Speech-to-text transcription                  |
| **Processing**     | `execute_react_loop`               | LLM reasoning + tool execution                |
| **Output (text)**  | `AssistantStatusMessage` broadcast | Widget label update via message broker        |
| **Output (audio)** | `TtsEngine::speak()`               | PCM synthesis via `ort` + playback via `cpal` |

**TTS is enabled by default** on this path — the user expects a spoken response after speaking.

#### Path 2: MCP Text Input → MCP Push Response

An MCP client (e.g., another plugin, a remote tool, or a CLI) submits text via `submit_text`. The response is pushed back to the MCP client as a structured JSON
message. TTS is optional on this path — a headless MCP client typically has no audio output device.

| Stage                        | Component                          | Description                                                                    |
|------------------------------|------------------------------------|--------------------------------------------------------------------------------|
| **Input**                    | `VoiceCommandMessage::submit_text` | Text submitted via MCP `InvokeResourceMessage`                                 |
| **Processing**               | `execute_react_loop`               | LLM reasoning + tool execution                                                 |
| **Output (MCP push)**        | `InvokeResourceResponse`           | Structured JSON with `response_type`, `final_answer` or `clarify`, and `state` |
| **Output (audio, optional)** | `TtsEngine::speak()`               | Only if `tts_enabled_mcp` is `true` in config                                  |

**MCP Push Format:**

The MCP status resource (`voice_assistant://status`) is extended with a `response_type` field so the MCP client can distinguish between a final answer and a
clarifying question:

```json
{
  "state": "Idle",
  "transcript": "öffne den Browser",
  "response_type": "final_answer",
  "final_answer": "Firefox wurde gestartet."
}
```

```json
{
  "state": "Idle",
  "transcript": "öffne das",
  "response_type": "clarify",
  "clarify": "Welche Anwendung möchtest du öffnen?"
}
```

The `response_type` field is an enum with two values:

- `"final_answer"` — the user's goal is fully accomplished or unresolvable
- `"clarify"` — the LLM asks the user a clarifying question; the MCP client should present this as a prompt for further input

### 1.3 Crate Structure

Following the project's decoupled SOA architecture:

| Crate       | Path                        | Responsibility                                        |
|-------------|-----------------------------|-------------------------------------------------------|
| **Model**   | `model/voice_assistant/`    | TTS config structs, message types for TTS status      |
| **Service** | `services/voice_assistant/` | TTS engine, phonemizer, ort inference, cpal output    |
| **Widget**  | `plugins/voice_assistant/`  | UI feedback during TTS playback (speaker icon, state) |

---

## 2. Hardware Profiles

### 2.1 Low-End Profile: Ryzen 5 8500G (iGPU)

| Property               | Value                                                        |
|------------------------|--------------------------------------------------------------|
| **Model**              | Piper TTS (`.onnx` file)                                     |
| **Execution Provider** | CPU-only in `ort`                                            |
| **RAM Usage**          | < 50 MB                                                      |
| **Latency**            | < 100 ms (Zen-4 CPU is fast enough for real-time)            |
| **Quality**            | Good, clear, robotic but intelligible                        |
| **Languages**          | German (`de_DE`), English (`en_US`) via espeak-ng phonemizer |

**Rationale:** Piper is extremely lightweight. The CPU provider in `ort` renders speech in milliseconds without touching the iGPU, leaving the GPU free for LLM
inference. This fits perfectly within the 2 GB VRAM budget.

### 2.2 High-End Profile: Ryzen 9 9950X3D + RX 7900 XTX

| Property               | Value                                                                             |
|------------------------|-----------------------------------------------------------------------------------|
| **Model**              | Kokoro-82M (`.onnx` file)                                                         |
| **Execution Provider** | ROCm (HIP) provider in `ort`                                                      |
| **VRAM Usage**         | ~200 MB                                                                           |
| **Latency**            | < 30 ms (GPU-accelerated)                                                         |
| **Quality**            | Excellent, natural-sounding, expressive                                           |
| **Languages**          | German, English, and others via Kokoro's multilingual phonemizer                  |
| **Voice Styles**       | Configurable via `voices-v1.0.bin` style vectors (e.g., `de_marcus`, `de_giggle`) |

**Rationale:** Kokoro-82M delivers breathtaking audio quality. With ROCm, the model runs entirely on the 7900 XTX, achieving sub-30 ms inference. The style
vectors allow voice customization without retraining.

### 2.3 Feature Flag Mapping

The existing `Cargo.toml` feature flags map to TTS execution providers:

```toml
[features]
default = []
ryzen-5-8500g = ["llm-vulkan", "whisper-vulkan"]
ryzen-9-9950x3d = ["llm-vulkan", "whisper-vulkan", "ort-rocm", "whisper-hipblas"]
ort-rocm = ["ort/rocm"]
```

The TTS engine selects the execution provider based on the active feature flag:

- `ryzen-5-8500g` → CPU provider, Piper model
- `ryzen-9-9950x3d` → ROCm provider, Kokoro-82M model

---

## 3. Phonemizer

ONNX models are pure mathematical engines — they consume numeric arrays (token IDs), not raw strings. A phonemizer converts text to phoneme tokens before the
`ort` inference call.

### 3.1 Pure Rust Phonemizer (`espeak-ng` crate)

Both Piper and Kokoro use the same phonemizer: the pure Rust [`espeak-ng`](https://crates.io/crates/espeak-ng) crate (v0.1.2). This is a from-scratch Rust
reimplementation of eSpeak NG — no C library, no FFI, no system package required.

**Key API functions:**

One-shot helper (no state, simplest for our use case):

- `espeak_ng::text_to_ipa(lang: &str, text: &str) -> Result<String, EspeakError>` — converts text to IPA phoneme symbols for the given BCP-47 language tag

Example: `espeak_ng::text_to_ipa("de", "Hallo")` → `"haloː"`

For full control (custom data directory, voice selection), use the `EspeakNg` builder:

```rust
let engine = espeak_ng::EspeakNg::new("de") ?;
let ipa = engine.text_to_phonemes("Hallo") ?;
```

If a custom data directory is needed (instead of bundled data), set the `ESPEAK_DATA_PATH` environment variable before initialization.

**Bundled data:** The crate ships phoneme data and 114 language dictionaries as optional feature flags. For the voice assistant, only German and English are
needed:

```toml
espeak-ng = { version = "0.1", features = ["bundled-data-de", "bundled-data-en"] }
```

This embeds the phoneme tables and dictionary data at compile time, making the service fully self-contained.

**Performance:** The pure Rust implementation achieves ~606 ns first-phoneme latency (9,000× faster than C subprocess) and 380× real-time synthesizer
throughput, eliminating process-spawn and shared-library initialization overhead.

### 3.2 Piper Phonemizer Pipeline

Piper (VITS architecture) requires text-to-phoneme conversion:

1. Text string → `espeak_ng::text_to_ipa("de", text)` → IPA phoneme symbols
2. IPA phoneme symbols → integer IDs via the model's `config.json` phoneme map
3. Integer IDs → `ort` inference → raw f32 PCM samples

### 3.3 Kokoro Phonemizer Pipeline

Kokoro-82M also requires phonemes but additionally needs a style vector:

1. Text string → `espeak_ng::text_to_ipa("de", text)` → IPA phoneme symbols
2. IPA phoneme symbols → integer IDs via Kokoro's phoneme map
3. Load style vector from `voices-v1.0.bin` based on the selected voice name
4. Feed both phoneme IDs and the 256-dimensional style vector as input tensors to the `ort` session

### 3.4 Configuration

```rust
/// TTS phonemizer configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TtsPhonemizerConfig {
    /// BCP-47 language tag for phonemization (e.g., "de", "en").
    pub language: String,
    /// Path to the espeak-ng data directory (optional if using bundled data).
    pub espeak_data_path: Option<String>,
    /// Path to the model's config.json containing the phoneme-to-ID map.
    pub phoneme_map_path: String,
    /// Path to the voices-v1.0.bin file (Kokoro only).
    pub voices_path: Option<String>,
    /// Selected voice name (Kokoro only, e.g., "de_marcus").
    pub voice_name: Option<String>,
}
```

---

## 4. TTS Engine

### 4.1 Core Struct

The TTS engine encapsulates the ONNX session and cpal output device. It follows the same pattern as the existing `LlmInferenceEngine` and `WhisperContext` in
the voice assistant service.

```rust
/// Text-to-Speech engine using ort for ONNX inference and cpal for audio output.
pub struct TtsEngine {
    /// ONNX inference session for the TTS model.
    onnx_session: ort::Session,
    /// cpal audio output device.
    cpal_device: cpal::Device,
    /// cpal supported stream configuration.
    cpal_config: cpal::SupportedStreamConfig,
    /// Phonemizer configuration.
    phonemizer_config: TtsPhonemizerConfig,
    /// TTS model type (Piper or Kokoro).
    model_type: TtsModelType,
    /// Native sample rate of the TTS model (e.g., 22050 for Piper, 24000 for Kokoro).
    model_sample_rate: u32,
}

/// Supported TTS model types.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TtsModelType {
    /// Piper TTS (VITS architecture, lightweight, CPU-only).
    Piper,
    /// Kokoro-82M (high-quality, GPU-accelerated).
    Kokoro,
}
```

### 4.2 Initialization

```rust
impl TtsEngine {
    /// Creates a new TTS engine from the given configuration.
    pub fn new(config: &TtsConfig) -> Result<Self, TtsError> {
        // 1. Load the ONNX model via ort.
        //    For ROCm: use ort::SessionBuilder with ROCm execution provider.
        //    For CPU: use ort::SessionBuilder with CPU execution provider.
        //    In ort = "=2.0.0-rc.12", the session is created via commit_from_file.
        let session = ort::Session::builder()
            .map_err(|e| TtsError::SessionCreate(e.to_string()))?
            .commit_from_file(&config.model_path)
            .map_err(|e| TtsError::ModelLoad(e.to_string()))?;

        // 2. Initialize cpal audio output.
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| TtsError::NoOutputDevice)?;
        let cpal_config = device
            .default_output_config()
            .map_err(|e| TtsError::AudioConfig(e.to_string()))?;

        Ok(Self {
            onnx_session: session,
            cpal_device: device,
            cpal_config,
            phonemizer_config: config.phonemizer_config.clone(),
            model_type: config.model_type.clone(),
            model_sample_rate: config.model_sample_rate(),
        })
    }
}
```

### 4.3 Inference and Playback

```rust
impl TtsEngine {
    /// Synthesizes speech from text and plays it through the audio output device.
    pub fn speak(&self, text: &str) -> Result<(), TtsError> {
        // 1. Convert text to phoneme tokens.
        let phoneme_ids = self.phonemize(text)?;

        // 2. Run ONNX inference to generate PCM audio samples at the model's native sample rate.
        let pcm_samples = self.run_inference(&phoneme_ids)?;

        // 3. Resample from model sample rate to cpal output sample rate if needed.
        let cpal_sample_rate = self.cpal_config.sample_rate().0;
        let resampled = if self.model_sample_rate != cpal_sample_rate {
            self.resample(pcm_samples, self.model_sample_rate, cpal_sample_rate)
        } else {
            pcm_samples
        };

        // 4. Stream PCM samples to cpal output device.
        self.play_audio(resampled)?;

        Ok(())
    }

    /// Converts text to phoneme token IDs using the configured phonemizer.
    fn phonemize(&self, text: &str) -> Result<Vec<i64>, TtsError> {
        match self.model_type {
            TtsModelType::Piper => {
                // espeak-ng → IPA phonemes → config.json ID map
                self.phonemize_piper(text)
            }
            TtsModelType::Kokoro => {
                // espeak-ng → IPA phonemes → Kokoro ID map
                self.phonemize_kokoro(text)
            }
        }
    }

    /// Runs the ONNX inference session to generate raw f32 PCM samples.
    fn run_inference(&self, phoneme_ids: &[i64]) -> Result<Vec<f32>, TtsError> {
        let input_tensor = ort::Tensor::from_array((
            [1, phoneme_ids.len()],
            phoneme_ids.to_vec(),
        ))
            .map_err(|e| TtsError::TensorCreate(e.to_string()))?;

        let inputs = ort::inputs!["input" => input_tensor]
            .map_err(|e| TtsError::InputCreate(e.to_string()))?;

        let outputs = self
            .onnx_session
            .run(inputs)
            .map_err(|e| TtsError::Inference(e.to_string()))?;

        let output_tensor = outputs
            .get("output")
            .ok_or_else(|| TtsError::MissingOutput)?;
        let extracted = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| TtsError::TensorExtract(e.to_string()))?;

        Ok(extracted.view().iter().copied().collect())
    }

    /// Resamples PCM samples from one sample rate to another using linear interpolation.
    /// This is necessary because TTS models output at their native rate (e.g., 22050 Hz
    /// for Piper, 24000 Hz for Kokoro), while cpal's default output device typically
    /// runs at 44100 Hz or 48000 Hz on Ubuntu/Gnome.
    fn resample(&self, samples: Vec<f32>, from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate {
            return samples;
        }
        let ratio = to_rate as f64 / from_rate as f64;
        let output_len = (samples.len() as f64 * ratio) as usize;
        let mut output = Vec::with_capacity(output_len);
        for i in 0..output_len {
            let src_pos = i as f64 / ratio;
            let src_idx = src_pos as usize;
            let frac = src_pos - src_idx as f64;
            let s0 = samples.get(src_idx).copied().unwrap_or(0.0);
            let s1 = samples.get(src_idx + 1).copied().unwrap_or(0.0);
            output.push((s0 as f64 + (s1 - s0) as f64 * frac) as f32);
        }
        output
    }

    /// Streams f32 PCM samples to the cpal audio output device.
    ///
    /// Uses a cursor-based `PlaybackState` struct for O(1) sample access in the
    /// audio callback. This avoids the O(N) `Vec::remove(0)` anti-pattern that
    /// would cause CPU spikes and buffer underruns in the real-time audio thread.
    ///
    /// Handles mono-to-stereo conversion: if cpal reports 2 channels, each mono
    /// sample is duplicated to both left and right channels.
    fn play_audio(&self, pcm_samples: Vec<f32>) -> Result<(), TtsError> {
        /// Playback state shared between the main thread and the cpal audio callback.
        struct PlaybackState {
            /// Mono PCM samples to play.
            samples: Vec<f32>,
            /// Current read position in the samples vector.
            position: usize,
        }

        let state = Arc::new(Mutex::new(PlaybackState {
            samples: pcm_samples,
            position: 0,
        }));
        let state_clone = Arc::clone(&state);

        let err_fn = |err| {
            debug!("Voice Assistant TTS: audio stream error: {}", err);
        };

        let channels = self.cpal_config.channels();

        let stream = self
            .cpal_device
            .build_output_stream(
                &self.cpal_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Use try_lock to avoid blocking the audio thread if the
                    // main thread is holding the lock (e.g., checking completion).
                    if let Ok(mut guard) = state_clone.try_lock() {
                        if channels == 2 {
                            // Stereo: duplicate mono samples to both channels.
                            let mut i = 0;
                            while i + 1 < data.len() {
                                if guard.position < guard.samples.len() {
                                    let sample = guard.samples[guard.position];
                                    guard.position += 1;
                                    data[i] = sample;
                                    data[i + 1] = sample;
                                } else {
                                    data[i] = 0.0;
                                    data[i + 1] = 0.0;
                                }
                                i += 2;
                            }
                        } else {
                            // Mono or unknown channel count: write directly.
                            for sample in data.iter_mut() {
                                *sample = if guard.position < guard.samples.len() {
                                    let s = guard.samples[guard.position];
                                    guard.position += 1;
                                    s
                                } else {
                                    0.0
                                };
                            }
                        }
                    } else {
                        // If lock is unavailable, output silence to avoid glitches.
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| TtsError::StreamCreate(e.to_string()))?;

        // CRITICAL for cpal v0.18.1: explicitly call .play()
        stream
            .play()
            .map_err(|e| TtsError::StreamPlay(e.to_string()))?;

        // Block until all samples have been played.
        loop {
            let done = state
                .lock()
                .map(|guard| guard.position >= guard.samples.len())
                .unwrap_or(true);
            if done {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Ok(())
    }
}
```

### 4.4 Error Handling

```rust
/// Errors that can occur during TTS synthesis or playback.
#[derive(Clone, Debug, thiserror::Error)]
pub enum TtsError {
    #[error("Failed to create ONNX session: {0}")]
    SessionCreate(String),
    #[error("Failed to load ONNX model: {0}")]
    ModelLoad(String),
    #[error("No audio output device available")]
    NoOutputDevice,
    #[error("Failed to configure audio output: {0}")]
    AudioConfig(String),
    #[error("Failed to create input tensor: {0}")]
    TensorCreate(String),
    #[error("Failed to create model inputs: {0}")]
    InputCreate(String),
    #[error("ONNX inference failed: {0}")]
    Inference(String),
    #[error("Model output tensor missing")]
    MissingOutput,
    #[error("Failed to extract output tensor: {0}")]
    TensorExtract(String),
    #[error("Failed to create audio stream: {0}")]
    StreamCreate(String),
    #[error("Failed to start audio stream: {0}")]
    StreamPlay(String),
    #[error("Phonemizer error: {0}")]
    Phonemizer(String),
}
```

---

## 5. Configuration

### 5.1 TTS Config Struct

```rust
/// Configuration for the TTS engine.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TtsConfig {
    /// Whether TTS output is enabled.
    pub enabled: bool,
    /// Path to the ONNX model file.
    pub model_path: String,
    /// TTS model type (Piper or Kokoro).
    pub model_type: TtsModelType,
    /// Phonemizer configuration.
    pub phonemizer_config: TtsPhonemizerConfig,
    /// Audio output sample rate (optional, defaults to model native rate).
    pub sample_rate: Option<u32>,
    /// Volume multiplier (0.0 to 1.0, default 1.0).
    pub volume: f32,
    /// Whether TTS is active for MCP text input path (default: false).
    /// Headless MCP clients typically have no audio output device.
    pub tts_enabled_mcp: bool,
}
```

### 5.2 Default Configuration

```rust
impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: String::new(),
            model_type: TtsModelType::Piper,
            phonemizer_config: TtsPhonemizerConfig::default(),
            sample_rate: None,
            volume: 1.0,
            tts_enabled_mcp: false,
        }
    }
}
```

### 5.3 Integration with VoiceAssistantServiceConfig

The TTS config is embedded in the existing voice assistant service configuration:

```rust
/// Voice assistant service configuration (extended with TTS).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VoiceAssistantServiceConfig {
    // ... existing fields ...

    /// Text-to-Speech configuration.
    #[serde(default)]
    pub tts: TtsConfig,
}
```

---

## 6. Integration into the Voice Assistant Pipeline

### 6.1 Current Pipelines (without TTS)

**Path 1: Voice Input**

```
Audio Capture → Whisper STT → Text → execute_react_loop() → String → current_answer → Widget Label
```

**Path 2: MCP Text Input**

```
MCP submit_text → execute_react_loop() → String → current_answer → voice_assistant://status (pollable resource)
```

### 6.2 Extended Pipeline: Path 1 — Voice Input → Voice Output

```
Audio Capture → Whisper STT → Text → execute_react_loop() → LlmResponse
                                                                    ↓
                                                    ┌───────────────┴───────────────┐
                                                    │                               │
                                              Widget Label                   TTS Engine
                                              (status update)              (speak response)
                                                    │                               │
                                                    │                          cpal Output
                                                    │                          (audio playback)
                                                    ▼                               ▼
                                              User sees text              User hears speech
```

TTS is **always active** on this path (if `tts.enabled` is `true` in config). The user spoke, so they expect a spoken response.

### 6.3 Extended Pipeline: Path 2 — MCP Text Input → MCP Push

```
MCP submit_text → execute_react_loop() → LlmResponse
                                    ↓
                    ┌─────────────────┴─────────────────┐
                    │                                   │
              MCP Push Response                   TTS Engine (optional)
              (structured JSON)                  (only if tts_enabled_mcp = true)
                    │                                   │
                    ▼                              cpal Output
              MCP Client receives               (audio playback, local only)
              response_type + payload
```

The MCP push response is sent via `InvokeResourceResponse` and contains:

- `response_type`: `"final_answer"` or `"clarify"`
- `final_answer` or `clarify`: the response text
- `state`: current assistant state
- `transcript`: the original user input

TTS on this path is controlled by the `tts_enabled_mcp` config flag (default: `false`). A headless MCP client typically has no audio output device, so TTS is
disabled by default for MCP text input.

### 6.4 Service Integration

The TTS engine is initialized alongside the existing LLM and Whisper engines in the service's `init` method:

```rust
pub struct VoiceAssistantService {
    // ... existing fields ...

    /// Text-to-Speech engine (optional, initialized if TTS is enabled).
    pub tts_engine: Option<Arc<TtsEngine>>,
}
```

### 6.5 Pipeline Call Site: Path 1 — Voice Input

After `execute_react_loop` returns successfully in `run_pipeline_inner`:

```rust
match temp_service.execute_react_loop(user_text).await {
Ok(response) => {
let (response_type, text) = match & response {
LlmResponse::FinalAnswer { answer } => ("final_answer", answer.clone()),
LlmResponse::Clarify { question } => ("clarify", question.clone()),
LlmResponse::ToolCall { .. } => unreachable ! ("ReAct loop should not return ToolCall"),
};

// Update the answer for widget display.
if let Ok( mut answer_guard) = answer.write() {
* answer_guard = text.clone();
}

// Broadcast status update with response type and text.
Self::set_state(state, AssistantState::Speaking, status_sender, transcript, answer).await;

// Speak the response via TTS (always active on voice input path if enabled).
if let Some(tts) = & tts_engine {
if let Err(error) = tts.speak( & text) {
warn ! ("Voice Assistant: TTS playback failed: {error}");
}
}

Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
}
Err(error) => {
// ... existing error handling ...
}
}
```

### 6.6 Pipeline Call Site: Path 2 — MCP Text Input

After `execute_react_loop` returns successfully in `run_text_pipeline`:

```rust
match temp_service.execute_react_loop(user_text).await {
Ok(response) => {
let (response_type, text) = match & response {
LlmResponse::FinalAnswer { answer } => ("final_answer", answer.clone()),
LlmResponse::Clarify { question } => ("clarify", question.clone()),
LlmResponse::ToolCall { .. } => unreachable ! ("ReAct loop should not return ToolCall"),
};

// Update the answer for MCP status resource.
if let Ok( mut answer_guard) = answer.write() {
* answer_guard = text.clone();
}

// Broadcast status update for widget (if running locally).
Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;

// TTS on MCP path is optional (default: disabled for headless clients).
if config.tts.enabled & & config.tts.tts_enabled_mcp {
if let Some(tts) = & tts_engine {
if let Err(error) = tts.speak( & text) {
warn ! ("Voice Assistant: TTS playback failed: {error}");
}
}
}

// MCP push response is available via voice_assistant://status resource.
// The response_type field distinguishes final_answer from clarify.
}
Err(error) => {
// ... existing error handling ...
}
}
```

### 6.7 MCP Status Resource Extension

The `voice_assistant://status` resource is extended with `response_type` and `clarify` fields:

```rust
let json = serde_json::json!({
    "state": state,
    "transcript": self.current_transcript.read().map(|t| t.clone()).unwrap_or_default(),
    "response_type": self.current_response_type.read().map(|t| t.clone()).unwrap_or_default(),
    "final_answer": self.current_answer.read().map(|a| a.clone()).unwrap_or_default(),
    "clarify": self.current_clarify.read().map(|c| c.clone()).unwrap_or_default(),
});
```

The MCP client can then distinguish:

- `response_type = "final_answer"` → read `final_answer` field, conversation is complete
- `response_type = "clarify"` → read `clarify` field, prompt the user for additional input

### 6.8 State Machine Extension

The assistant state machine is extended with a `Speaking` state to provide UI feedback during TTS playback:

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize, stabby::stabby)]
pub enum AssistantState {
    #[default]
    Idle,
    Listening,
    Transcribing,
    ThinkingLlm,
    ExecutingAction,
    /// Speaking the response via TTS.
    Speaking,
    Error,
}
```

The pipeline transitions:

1. `ThinkingLlm` → `Speaking` (TTS starts)
2. `Speaking` → `Idle` (TTS finished)

---

## 7. Model Acquisition

### 7.1 Piper TTS Models

Piper models are available from the [Piper Releases](https://github.com/rhasspy/piper/releases) page. Each model consists of:

| File                              | Description                         |
|-----------------------------------|-------------------------------------|
| `de_DE-thorsten-medium.onnx`      | The ONNX model file (~60 MB)        |
| `de_DE-thorsten-medium.onnx.json` | Phoneme map and model configuration |
| `en_US-amy-medium.onnx`           | English model (~60 MB)              |

**Recommended German voice**: `de_DE-thorsten-medium` (good quality, reasonable size)
**Recommended English voice**: `en_US-amy-medium` (natural, clear)

### 7.2 Kokoro-82M Models

Kokoro models are available from the [Kokoro Releases](https://huggingface.co/hexgrad/Kokoro-82M) page:

| File               | Description                          |
|--------------------|--------------------------------------|
| `kokoro-v1_0.onnx` | The ONNX model file (~330 MB)        |
| `voices-v1.0.bin`  | Style vectors for all voices (~2 MB) |
| `config.json`      | Phoneme map and model configuration  |

**Recommended German voices**: `de_marcus`, `de_giggle`
**Recommended English voices**: `af_heart`, `am_adam`

---

## 8. Audio Resampling

TTS models output PCM samples at their native sample rate, which typically differs from the cpal output device's sample rate on Ubuntu/Gnome.

### 8.1 Sample Rate Mismatch

| Source     | Typical Sample Rate    | cpal Default Output (Ubuntu/Gnome) |
|------------|------------------------|------------------------------------|
| Piper TTS  | 22,050 Hz or 16,000 Hz | 44,100 Hz or 48,000 Hz (Stereo)    |
| Kokoro-82M | 24,000 Hz              | 44,100 Hz or 48,000 Hz (Stereo)    |

If 22,050 Hz mono samples are played directly into a 48,000 Hz stereo stream without resampling:

- The voice plays at ~2.2× speed (Mickey Mouse effect) because the audio server consumes more samples per second
- Audio only plays on the left channel (mono-to-stereo mismatch)

### 8.2 Resampling Strategy

The `TtsEngine::resample()` method performs linear interpolation to convert between sample rates. This is a lightweight O (N) operation that runs once after
inference, before audio playback begins.

For higher quality, the `rubato` crate could be used as a drop-in replacement for the linear interpolation, but linear interpolation is sufficient for speech
(music would require higher quality). The resampling step adds < 1 ms latency for typical response lengths.

### 8.3 Mono-to-Stereo Conversion

The cpal audio callback handles mono-to-stereo conversion at playback time. When `cpal_config.channels() == 2`, each mono sample is duplicated to both left and
right channels:

```rust
data[i] = sample;      // left
data[i + 1] = sample;  // right
```

This ensures the voice is centered in the stereo field.

---

## 9. Dependency Strategy

### 9.1 Existing Dependencies (No Changes Required)

| Dependency  | Version        | Role in TTS                                     |
|-------------|----------------|-------------------------------------------------|
| `ort`       | `=2.0.0-rc.12` | ONNX model inference (Piper and Kokoro)         |
| `cpal`      | `0.18.1`       | Audio output stream (PCM playback)              |
| `fastembed` | `5.17`         | Already brings `ort` as a transitive dependency |

### 9.2 New Dependency

| Dependency  | Version | Purpose                                    |
|-------------|---------|--------------------------------------------|
| `espeak-ng` | `0.1`   | Pure Rust phonemizer (text → IPA phonemes) |

**Feature flags:** `bundled-data-de` and `bundled-data-en` embed the German and English dictionaries at compile time, eliminating any runtime system dependency.

```toml
espeak-ng = { version = "0.1", features = ["bundled-data-de", "bundled-data-en"] }
```

### 9.3 Why No Additional Frameworks

| Framework               | Reason to Avoid                                                              |
|-------------------------|------------------------------------------------------------------------------|
| `sherpa-onnx`           | Ships with older `ort` v1.16, causing duplicate ONNX C-library compilation   |
| `rodio`                 | Adds unnecessary abstraction over `cpal`, which we already use for STT input |
| `tts` crate             | Wraps external TTS engines, adding overhead without benefit                  |
| `coqui-tts`             | Python-based, not suitable for a pure Rust project                           |
| `libespeak-ng` (system) | Replaced by pure Rust `espeak-ng` crate — no system package or FFI needed    |

---

## 10. Performance Analysis

### 10.1 Latency Budget

| Stage                               | Piper (CPU)                               | Kokoro (ROCm) |
|-------------------------------------|-------------------------------------------|---------------|
| Phonemizer                          | < 1 ms (pure Rust, ~606 ns first phoneme) | < 1 ms        |
| ONNX Inference                      | 50-100 ms                                 | 10-30 ms      |
| Resampling                          | < 1 ms                                    | < 1 ms        |
| Audio Playback Start                | 5-10 ms                                   | 5-10 ms       |
| **Total TTF (Time-to-First-Audio)** | **56-112 ms**                             | **16-42 ms**  |

### 10.2 Memory Usage

| Component                 | Piper (CPU)  | Kokoro (ROCm)  |
|---------------------------|--------------|----------------|
| ONNX Model                | ~60 MB (RAM) | ~330 MB (VRAM) |
| Style Vectors             | N/A          | ~2 MB (VRAM)   |
| Phonemizer (bundled data) | ~5 MB (RAM)  | ~5 MB (RAM)    |
| Audio Buffer              | ~1 MB (RAM)  | ~1 MB (RAM)    |
| Resampled Buffer          | ~2 MB (RAM)  | ~2 MB (RAM)    |
| **Total**                 | **~68 MB**   | **~340 MB**    |

### 10.3 Comparison: Before and After TTS

| Aspect                      | Before TTS                     | With TTS                        |
|-----------------------------|--------------------------------|---------------------------------|
| **Output Modality**         | Text only (widget label)       | Text + Speech                   |
| **Accessibility**           | Requires visual attention      | Hands-free, eyes-free           |
| **Voice Input Response**    | User must read the screen      | Natural conversational flow     |
| **MCP Text Input Response** | User must poll status resource | Audio confirmation (if enabled) |
| **Latency Overhead**        | N/A                            | 20-120 ms (model-dependent)     |
| **Memory Overhead**         | N/A                            | 66-338 MB (model-dependent)     |

---

## 11. Limitations and Future Extensions

### 11.1 Current Limitations

- **Blocking audio playback**: The `speak` method blocks until playback completes. For long responses, this could be improved with async streaming.
- **No audio device selection**: Currently uses the default output device. Configuration for a specific device would be needed for multi-output setups.
- **No interruption support**: Once TTS playback starts, it cannot be interrupted. A cancel mechanism would allow the user to stop playback by clicking the
  widget.
- **Single voice**: The voice is fixed at initialization. Dynamic voice switching would require reloading the style vector (Kokoro) or the entire model (Piper).

### 11.2 Future Extensions

- **Streaming inference**: Generate audio in chunks and start playback before the full inference completes, reducing time-to-first-audio.
- **Voice activity detection (VAD)**: Allow the user to interrupt TTS playback by speaking, enabling barge-in.
- **Dynamic voice selection**: Switch voices based on the response type (e.g., different voices for `final_answer` vs `clarify`).
- **SSML support**: Parse Speech Synthesis Markup Language for prosody control (pauses, emphasis, speed).
- **Higher-quality resampling**: Replace linear interpolation with `rubato` for broadcast-quality sample rate conversion, useful if TTS is used for long-form
  content.
- **Audio caching**: Cache frequently used phrases (e.g., "I'm sorry, I didn't understand that") to avoid redundant inference.

---

## 12. Summary

| Aspect                | Value                                                                                          |
|-----------------------|------------------------------------------------------------------------------------------------|
| **TTS Engine**        | `ort` (ONNX Runtime) + `cpal` (audio output)                                                   |
| **Low-End Model**     | Piper TTS (CPU, < 50 MB, < 120 ms latency)                                                     |
| **High-End Model**    | Kokoro-82M (ROCm, ~330 MB, < 30 ms latency)                                                    |
| **Phonemizer**        | `espeak-ng` v0.1 (pure Rust, bundled data for de + en)                                         |
| **Path 1 (Voice)**    | Voice input → ReAct → TTS audio + widget label                                                 |
| **Path 2 (MCP)**      | MCP text input → ReAct → MCP push (`response_type` + payload), TTS optional                    |
| **MCP Push Fields**   | `response_type` (`final_answer` / `clarify`), `final_answer`, `clarify`, `state`, `transcript` |
| **New Dependencies**  | `espeak-ng` v0.1 (pure Rust phonemizer, bundled data)                                          |
| **Integration Point** | After `execute_react_loop` in `run_pipeline_inner` (Path 1) and `run_text_pipeline` (Path 2)   |
| **State Extension**   | `AssistantState::Speaking` for UI feedback                                                     |
| **Config**            | `TtsConfig` with `tts_enabled_mcp` flag (default: `false`)                                     |
| **Error Handling**    | `TtsError` with `thiserror`, non-fatal (TTS failures do not block the response)                |
