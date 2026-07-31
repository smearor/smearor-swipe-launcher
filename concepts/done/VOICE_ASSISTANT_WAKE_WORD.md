# Concept: Voice Assistant Wake Word Detection (Standby Mode)

This document describes the concept for integrating **openWakeWord**-based wake word detection into the Voice Assistant Service. It adds a new pipeline stage —
**Standby** — that continuously listens for a wake word before activating the full STT → ReAct pipeline. This eliminates the need for manual activation (button
press or MCP tool call) and enables a hands-free "hey assistant" experience.

---

## 1. Goal & Motivation

### Current State

The Voice Assistant requires explicit activation via `VoiceCommandAction::Activate`. The user must press a button, use an MCP tool, or send a `SubmitText`
command before the pipeline starts:

1. `Activate` → `capture_audio` (cpal, 16 kHz mono, silence-detected)
2. `transcribe_async` (Whisper)
3. `run_react` (LLM ReAct loop)

In `Idle` state, the microphone is not capturing audio. There is no way to trigger the assistant by voice alone.

### Problem

- **No hands-free activation**: The user must physically interact with the launcher or invoke an MCP tool.
- **No always-on listening**: The assistant is dormant unless explicitly activated.
- **No wake word detection**: There is no lightweight, low-power detection layer that can run continuously without the cost of full Whisper transcription.

### Required Capabilities

| Capability                     | Example                                    | Solution                                          |
|--------------------------------|--------------------------------------------|---------------------------------------------------|
| Continuous wake word listening | "Alexa" / "Hey Mycroft"                    | `WakeWordDetector` using `oww-rs`                 |
| Hands-free activation          | User says wake word → pipeline starts      | `AssistantEvent::WakeWordDetected` internal event |
| Standby state                  | Microphone active, but Whisper/LLM dormant | `AssistantState::Standby`                         |
| Configurable wake word model   | Choose between Alexa, Mycroft, or custom   | `WakeWordConfig` in `VoiceAssistantServiceConfig` |
| Low-latency detection          | < 200 ms from wake word to pipeline start  | 1280-sample (80 ms) frames via ring buffer        |

---

## 2. Library Choice: oww-rs

### Why oww-rs

[`oww-rs`](https://crates.io/crates/oww-rs) (v0.3.3) is a minimalistic Rust port of [openWakeWord](https://github.com/dscripka/openWakeWord). It reimplements
the ONNX inference path — melspectrogram → speech-embedding → wakeword-classifier — for low-latency wake word detection.

**Key properties:**

- **Inference-only**: No training. Models are trained with the upstream Python project and consumed as `.onnx` files.
- **Embedded models**: Ships with `alexa` and `hey_mycroft` models embedded at compile time via `rust-embed`. No external model files needed for the built-in
  wake words.
- **Fixed chunk size**: `OWW_MODEL_CHUNK_SIZE = 1280` samples (80 ms at 16 kHz mono `f32`).
- **Falling-edge detection**: Fires on the falling edge of the probability curve with a refractory window, more robust than naive threshold crossing.
- **16 kHz mono f32 input**: Matches the existing audio capture format in `audio.rs`.

### Important: oww-rs uses `tract-onnx`, not `ort`

> **Correction to the original assumption**: oww-rs does **not** use the `ort` (ONNX Runtime) crate. It uses [`tract-onnx`](https://crates.io/crates/tract-onnx)
> v0.23 as its inference engine. `tract` is a pure-Rust neural network inference library with no native dependency on `libonnxruntime.so`.

This has the following implications:

- **No `ort` session sharing**: The existing `ort` usage in the service (TTS via `tts.rs`, SemanticMemory via `memory.rs`) is separate from oww-rs's `tract`
  usage. There is no shared `SessionOptions` or ONNX Runtime environment to configure.
- **No GPU EP conflict**: `tract` runs on CPU. It does not compete with `ort`'s ROCm/CUDA execution providers for GPU resources.
- **No `init_ort_environment` interaction**: The `init_ort_environment()` call in `service.rs` that loads a system `libonnxruntime.so` with GPU support has no
  effect on oww-rs.
- **Memory overhead is minimal**: `tract` models for wake word detection are small (~1–2 MB). The memory overhead of a separate inference engine is negligible
  compared to Whisper and the LLM.

If GPU acceleration for wake word detection becomes desirable in the future, an alternative would be to port the openWakeWord models to `ort` directly (they are
standard ONNX files). However, `tract` on CPU is fast enough for 80 ms frames at < 5% CPU usage.

---

## 3. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                       Voice Assistant Service                            │
│                                                                          │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │  Shared Audio     │    │  Wake Word       │    │  Pipeline        │   │
│  │  Capture (cpal)   │───▶│  Detector        │    │  Audio Consumer  │   │
│  │  (single stream)  │    │  (oww-rs/tract)  │    │  (Whisper+LLM)   │   │
│  │                   │    │                  │    │                  │   │
│  │  16 kHz mono f32  │    │  1280-sample     │    │  capture_audio   │   │
│  │  → Split Buffer   │    │  frames (80 ms)  │    │  → transcribe    │   │
│  │    (broadcast)    │    │  → detection()   │    │  → react_loop    │   │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘   │
│          ▲                        │                     │               │
│          │                        │                     │               │
│  ┌───────┴──────────┐    ┌───────▼──────────┐    ┌────▼──────────┐     │
│  │  Standby Loop     │    │  Event:          │    │  State:        │     │
│  │  (continuous)     │    │  WakeWordDetected│    │  Listening →   │     │
│  │  State: Standby   │    │  → switch mode   │    │  ProcessingStt │     │
│  └──────────────────┘    └──────────────────┘    └────────────────┘     │
│                                                                          │
│  Key: cpal stream stays open across Standby ↔ Listening transitions.     │
│  A shared split buffer broadcasts samples to both consumers.             │
│  No stream close/reopen → zero audio latency on wake word trigger.       │
└──────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User enables wake word mode (config or MCP tool)
2. Service enters Standby state, starts shared continuous audio capture
3. cpal callback pushes 16 kHz mono f32 samples into Split Buffer
4. WakeWordDetector pulls 1280-sample frames from its Split Buffer consumer
5. For each frame: model.detection(frame) → Detection { detected, probability }
6. If detected:
   a. Service sends internal event WakeWordDetected
   b. Service transitions Standby → Listening
   c. Pipeline audio consumer switches from "discard" mode to "collect" mode
      (the cpal stream is NOT restarted — it stays open the entire time)
   d. Service collects audio from Split Buffer → transcribe → react_loop
7. After pipeline completes, Service returns to Standby (if wake word mode is still enabled)
   Pipeline consumer switches back to "discard" mode; detector resumes evaluation
```

---

## 4. Pipeline Stages & State Machine

### New State: `Standby`

Add a new variant to `AssistantState` in `model/voice_assistant/src/messages/state.rs`:

```rust
/// The assistant is in standby mode, continuously listening for a wake word.
/// Audio is captured and processed by the wake word detector, but Whisper and
/// the LLM are not running.
Standby,
```

### State Transitions

```
Idle ──(enable_wake_word)──▶ Standby
Standby ──(wake_word_detected)──▶ Listening
Listening ──(pipeline_complete)──▶ Standby (if wake word mode enabled)
                                    └──▶ Idle (if wake word mode disabled)
Standby ──(disable_wake_word)──▶ Idle
Standby ──(error)──▶ Error ──▶ Idle
```

### New Command Action

Add a new variant to `VoiceCommandAction` in `model/voice_assistant/src/messages/command.rs`:

```rust
/// Enable wake word detection mode (continuous standby listening).
EnableWakeWord,
/// Disable wake word detection mode and return to idle.
DisableWakeWord,
```

---

## 5. Ring Buffer & Frame Accumulation

### Why a Split Buffer (Shared Audio Source)

The cpal audio callback fires in irregular chunks (depending on the device buffer size). The wake word detector requires exactly `OWW_MODEL_CHUNK_SIZE = 1280`
samples per frame. A ring buffer decouples the producer (cpal callback) from the consumer (detection loop).

**Critical design decision**: The cpal stream is opened **once** when wake word mode is enabled and stays open across all Standby ↔ Listening transitions.
Closing and reopening a cpal input stream on Linux (ALSA/PipeWire) can introduce 500 ms – 1 s of latency due to device negotiation. To avoid this, a **split
buffer** architecture is used: a single cpal callback writes into a central `SharedAudioSource`, which broadcasts samples to multiple consumers (wake word
detector + pipeline audio collector) via individual ring buffer consumers.

### Implementation with `ringbuf` Crate

Add the [`ringbuf`](https://crates.io/crates/ringbuf) v0.5 crate to `services/voice_assistant/Cargo.toml`:

```toml
ringbuf = "0.5"
```

### Split Buffer Design

```rust
use ringbuf::HeapRb;
use ringbuf::traits::Producer;
use ringbuf::traits::Consumer;
use std::sync::Arc;
use std::sync::Mutex;

/// Capacity: 2 seconds of audio at 16 kHz = 32000 samples.
/// This is generous — the detection loop consumes 1280 samples every 80 ms.
const RING_BUFFER_CAPACITY: usize = 32000;

/// Central audio source that broadcasts cpal samples to multiple consumers.
/// Each consumer gets its own ring buffer, so they can read at their own pace.
pub struct SharedAudioSource {
    /// List of consumer ring buffer producers.
    /// The cpal callback pushes samples into ALL registered producers.
    consumers: Arc<Mutex<Vec<ringbuf::producer::Producer<f32>>>>,
}

impl SharedAudioSource {
    /// Creates a new shared audio source with no consumers.
    pub fn new() -> Self {
        Self {
            consumers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registers a new consumer and returns its ring buffer consumer half.
    pub fn subscribe(&self) -> ringbuf::consumer::Consumer<f32> {
        let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
        let (producer, consumer) = rb.split();
        if let Ok(mut consumers) = self.consumers.lock() {
            consumers.push(producer);
        }
        consumer
    }

    /// Pushes samples to all registered consumers.
    /// Called from the cpal callback.
    pub fn push_samples(&self, samples: &[f32]) {
        if let Ok(mut consumers) = self.consumers.lock() {
            for producer in consumers.iter_mut() {
                for sample in samples {
                    let _ = producer.push_overwrite(*sample);
                }
            }
        }
    }
}
```

### Producer Side (cpal Callback)

The cpal input stream callback pushes mono f32 samples into the shared audio source, which broadcasts to all subscribed consumers:

```rust
move | data: & [f32], _: & cpal::InputCallbackInfo| {
// Mono downmix (same logic as audio.rs)
let mono_samples: Vec < f32 > = downmix_to_mono(data, channels);
// Broadcast to all consumers (wake word detector + pipeline collector)
shared_audio_source.push_samples( &mono_samples);
}
```

### Consumer 1: Wake Word Detection Loop

The detection loop subscribes to the shared audio source and pulls 1280-sample frames:

```rust
let mut consumer = shared_audio_source.subscribe();
let mut frame_buffer = [0.0f32; OWW_MODEL_CHUNK_SIZE];
let mut frame_index = 0;

loop {
// Pop samples from ring buffer into frame
while frame_index < OWW_MODEL_CHUNK_SIZE {
if let Some(sample) = consumer.try_pop() {
frame_buffer[frame_index] = sample;
frame_index += 1;
} else {
// Buffer underrun — wait for more audio
tokio::time::sleep(Duration::from_millis(10)).await;
continue;
}
}
frame_index = 0;

// Skip detection if TTS is speaking (avoid self-trigger)
if * is_speaking.lock().unwrap() {
continue;
}

// Run detection on full frame
let detection = model.detection( & frame_buffer);
if detection.detected {
// Send WakeWordDetected event
wake_event_sender.send(WakeWordEvent::Detected {
probability: detection.probability,
}).await.ok();
break;
}
}
```

### Consumer 2: Pipeline Audio Collector

When the pipeline activates (after wake word detection), it subscribes to the same shared audio source and collects samples for Whisper transcription. This
replaces the standalone `capture_audio` function when wake word mode is active:

```rust
let mut consumer = shared_audio_source.subscribe();
let mut pipeline_samples = Vec::new();

// Collect until silence or max duration
loop {
if let Some(sample) = consumer.try_pop() {
pipeline_samples.push(sample);
// ... silence detection, max duration check ...
} else {
tokio::time::sleep(Duration::from_millis(10)).await;
}
}
```

The cpal stream is **never** closed and reopened. Both consumers read from the same continuous stream via their independent ring buffer subscriptions.

---

## 6. Wake Word Detector Module

### New File: `services/voice_assistant/src/wake_word.rs`

This module encapsulates the oww-rs model, ring buffer, and detection loop.

```rust
/// Errors that can occur during wake word detection.
#[derive(Debug, thiserror::Error)]
pub enum WakeWordError {
    /// Failed to initialize the oww-rs model.
    #[error("Failed to initialize wake word model: {0}")]
    ModelInit(String),
    /// Failed to open the audio input device.
    #[error("Audio device error: {0}")]
    AudioDevice(String),
    /// The wake word detector was stopped by the user.
    #[error("Wake word detector stopped")]
    Stopped,
}

/// Internal event sent from the detection loop to the service.
pub enum WakeWordEvent {
    /// A wake word was detected with the given probability.
    Detected {
        /// Detection confidence (0.0–1.0).
        probability: f32,
    },
    /// An error occurred in the detection loop.
    Error(WakeWordError),
}

/// Configuration for wake word detection.
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// Which wake word model to use.
    pub model_type: WakeWordModelType,
    /// Detection threshold (0.0–1.0). Lower = more sensitive, more false positives.
    pub threshold: f32,
    /// Whether wake word mode is enabled on startup.
    pub auto_enable: bool,
}

/// Available wake word models.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum WakeWordModelType {
    /// Built-in "Alexa" wake word from openWakeWord.
    #[default]
    Alexa,
    /// Built-in "Hey Mycroft" wake word from openWakeWord.
    HeyMycroft,
    /// Custom wake word model loaded from a .onnx file path.
    Custom(String),
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            model_type: WakeWordModelType::default(),
            threshold: 0.1,
            auto_enable: false,
        }
    }
}

/// Wake word detector that runs a continuous audio capture + detection loop.
pub struct WakeWordDetector {
    /// The oww-rs model instance.
    model: OwwModel,
    /// Configuration.
    config: WakeWordConfig,
}
```

### Detection Loop

The detection loop is spawned as a dedicated thread (like the existing `capture_audio` in `audio.rs`) because `cpal::Stream` is `!Send` on some platforms. The
loop:

1. Opens a cpal input stream at 16 kHz mono **once** — this stream stays open for the entire lifetime of wake word mode.
2. Pushes samples into the `SharedAudioSource` in the callback, which broadcasts to all subscribed consumers.
3. The detector subscribes to the shared source and consumes 1280-sample frames from its own ring buffer consumer.
4. Calls `model.detection(frame)` for each frame.
5. On detection, sends `WakeWordEvent::Detected` via a `tokio::sync::mpsc` channel.
6. Exits when a stop signal is received via a `tokio::sync::oneshot` channel.

#### Thread Priority

The detection thread should be spawned with elevated scheduling priority to prevent "stuttering" in wake word recognition when the LLM or Whisper consume heavy
CPU. On Linux, this can be achieved via `nice`/`sched_setscheduler`:

```rust
use std::thread::Builder;

// Spawn with a custom name for debugging and elevated priority
let builder = Builder::new()
.name("wake-word-detector".to_string());

let handle = builder.spawn(move | | {
// Attempt to set real-time scheduling priority (SCHED_FIFO, priority 5).
// This is best-effort — if the process lacks CAP_SYS_NICE, it silently falls back
// to the default scheduler. The detection loop still works, just with less priority.
# [cfg(target_os = "linux")]
{
use std::os::raw::c_int;
extern "C" {
fn sched_setscheduler(pid: c_int, policy: c_int, param: * const SchedParam) -> c_int;
}
# [repr(C)]
struct SchedParam { sched_priority: c_int }
const SCHED_FIFO: c_int = 1;
let param = SchedParam { sched_priority: 5 };
let _ = unsafe { sched_setscheduler(0, SCHED_FIFO, & param) };
}

let runtime = tokio::runtime::Builder::new_current_thread()
.enable_all()
.build();
// ... rest of runtime setup ...
});
```

Alternatively, use the [`nix`](https://crates.io/crates/nix) crate for a safer wrapper around `pthread_setschedparam` / `sched_setscheduler`.

The key insight: wake word detection runs on CPU (via `tract`), while the LLM may also use CPU threads for prompt processing. Without elevated priority, the
detection loop's 80 ms frame deadline can be missed under heavy LLM load, causing the detector to miss a wake word. With `SCHED_FIFO` priority 5 (or a `nice`
value of -5), the detection thread preempts LLM worker threads for its short detection bursts.

```rust
impl WakeWordDetector {
    /// Creates a new detector with the given configuration.
    pub fn new(config: WakeWordConfig) -> Result<Self, WakeWordError> {
        let model = match &config.model_type {
            WakeWordModelType::Alexa => {
                OwwModel::new(SpeechUnlockType::OpenWakeWordAlexa, config.threshold)
                    .map_err(|e| WakeWordError::ModelInit(e.to_string()))?
            }
            WakeWordModelType::HeyMycroft => {
                OwwModel::new(SpeechUnlockType::OpenWakeWordHeyMycroft, config.threshold)
                    .map_err(|e| WakeWordError::ModelInit(e.to_string()))?
            }
            WakeWordModelType::Custom(path) => {
                new_model(path, config.threshold)
                    .map_err(|e| WakeWordError::ModelInit(e.to_string()))?
            }
        };
        Ok(Self { model, config })
    }

    /// Starts the continuous detection loop.
    /// Returns a receiver for wake word events and a stop sender.
    /// The cpal stream is managed by the caller via `SharedAudioSource`;
    /// the detector only subscribes to it.
    pub fn start(
        mut self,
        shared_audio_source: &SharedAudioSource,
        is_speaking: Arc<Mutex<bool>>,
    ) -> (
        tokio::sync::mpsc::UnboundedReceiver<WakeWordEvent>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let consumer = shared_audio_source.subscribe();

        let builder = std::thread::Builder::new()
            .name("wake-word-detector".to_string());

        builder.spawn(move || {
            // Best-effort elevated scheduling priority on Linux.
            #[cfg(target_os = "linux")]
            {
                use std::os::raw::c_int;
                extern "C" {
                    fn sched_setscheduler(pid: c_int, policy: c_int, param: *const SchedParam) -> c_int;
                }
                #[repr(C)]
                struct SchedParam { sched_priority: c_int }
                const SCHED_FIFO: c_int = 1;
                let param = SchedParam { sched_priority: 5 };
                let _ = unsafe { sched_setscheduler(0, SCHED_FIFO, &param) };
            }

            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = event_tx.send(WakeWordEvent::Error(
                        WakeWordError::AudioDevice(e.to_string())
                    ));
                    return;
                }
            };
            runtime.block_on(async move {
                if let Err(e) = self.run_detection_loop(event_tx.clone(), stop_rx, consumer, is_speaking).await {
                    let _ = event_tx.send(WakeWordEvent::Error(e));
                }
            });
        });

        (event_rx, stop_tx)
    }

    async fn run_detection_loop(
        &mut self,
        event_tx: tokio::sync::mpsc::UnboundedSender<WakeWordEvent>,
        stop_rx: tokio::sync::oneshot::Receiver<()>,
        mut consumer: ringbuf::consumer::Consumer<f32>,
        is_speaking: Arc<Mutex<bool>>,
    ) -> Result<(), WakeWordError> {
        // Detection loop — reads from the shared audio source's ring buffer.
        // The cpal stream is NOT managed here; it stays open in the service.
        let mut frame_buffer = [0.0f32; OWW_MODEL_CHUNK_SIZE];
        let mut frame_index = 0;
        let mut stop_rx = stop_rx;

        loop {
            // Check stop signal
            match stop_rx.try_recv() {
                Ok(()) | Err(oneshot::error::TryRecvError::Closed) => {
                    return Err(WakeWordError::Stopped);
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
            }

            // Fill frame from ring buffer
            while frame_index < OWW_MODEL_CHUNK_SIZE {
                if let Some(sample) = consumer.try_pop() {
                    frame_buffer[frame_index] = sample;
                    frame_index += 1;
                } else {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    break;
                }
            }
            if frame_index < OWW_MODEL_CHUNK_SIZE {
                continue;
            }
            frame_index = 0;

            // Skip detection while TTS is speaking to prevent self-trigger
            // (the assistant's own voice could match the wake word model).
            if *is_speaking.lock().unwrap_or_else(|e| e.into_inner()) {
                continue;
            }

            // Run detection
            let detection = self.model.detection(&frame_buffer);
            if detection.detected {
                let _ = event_tx.send(WakeWordEvent::Detected {
                    probability: detection.probability,
                });
                // Refractory period: oww-rs handles this internally via
                // falling-edge detection, but we add a short sleep to avoid
                // immediate re-trigger from the same audio segment.
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}
```

---

## 7. Integration into `VoiceAssistantService`

### 7.1 State Additions

In `services/voice_assistant/src/service.rs`, add:

```rust
pub struct VoiceAssistantService {
    // ... existing fields ...

    /// Whether wake word mode is enabled.
    wake_word_enabled: Arc<Mutex<bool>>,
    /// The wake word detector, active when in Standby mode.
    wake_word_detector: Arc<Mutex<Option<WakeWordDetectorHandle>>>,
    /// Wake word configuration.
    wake_word_config: WakeWordConfig,
    /// Shared audio source — a single cpal stream that broadcasts to all consumers.
    /// Opened once when wake word mode is enabled; stays open across all transitions.
    shared_audio_source: Arc<Mutex<Option<SharedAudioSource>>>,
    /// Whether TTS is currently speaking. Used to suppress wake word detection
    /// during TTS playback to prevent the assistant from hearing its own voice.
    is_speaking: Arc<Mutex<bool>>,
}

/// Handle for the running wake word detector.
struct WakeWordDetectorHandle {
    /// Stop sender to terminate the detection loop.
    stop_sender: tokio::sync::oneshot::Sender<()>,
    /// Event receiver for wake word detections.
    event_receiver: tokio::sync::mpsc::UnboundedReceiver<WakeWordEvent>,
}
```

The `is_speaking` flag is set to `true` before TTS playback starts in `run_react` (section 7.3) and set to `false` after playback completes. The detection loop
checks this flag before each `model.detection()` call and skips evaluation while TTS is active. This prevents the assistant from triggering on its own
synthesized speech.

The `shared_audio_source` holds the single cpal input stream that stays open for the entire lifetime of wake word mode. Both the wake word detector and the
pipeline audio collector subscribe to it via `subscribe()`. When the pipeline needs audio for Whisper transcription, it subscribes to the same source — no
stream restart is needed.

### 7.2 Command Handling

Add `EnableWakeWord` and `DisableWakeWord` to the command receiver loop in `service.rs`:

```rust
VoiceCommandAction::EnableWakeWord => {
let mut enabled = wake_word_enabled.lock().unwrap_or_else( | e | e.into_inner());
if * enabled {
debug ! ("Voice Assistant: wake word mode already enabled");
continue;
}
* enabled = true;
drop(enabled);

// Enter Standby state and start detection loop
Self::set_state( & service_state, AssistantState::Standby, & service_status_sender, & service_transcript, & service_answer).await;
Self::start_wake_word_detection(
& service_wake_word_config,
& service_wake_word_detector,
& service_state,
& service_status_sender,
&service_transcript,
& service_answer,
& service_active,
& service_config,
& service_whisper,
// ... all other pipeline params ...
).await;
}

VoiceCommandAction::DisableWakeWord => {
let mut enabled = wake_word_enabled.lock().unwrap_or_else( | e | e.into_inner());
* enabled = false;
drop(enabled);

// Stop detection loop
if let Ok( mut guard) = service_wake_word_detector.lock() {
if let Some(handle) = guard.take() {
let _ = handle.stop_sender.send(());
}
}

Self::set_state( & service_state, AssistantState::Idle, & service_status_sender, & service_transcript, & service_answer).await;
}
```

### 7.3 Wake Word Detection → Pipeline Activation

When `WakeWordEvent::Detected` is received, the service:

1. **Does NOT stop the cpal stream** — the stream stays open. The detection loop is paused (it exits its loop after sending the event), but the
   `SharedAudioSource` continues receiving samples.
2. Transitions to `Listening` state.
3. Subscribes a new consumer to the `SharedAudioSource` and collects audio for Whisper transcription (replaces the standalone `capture_audio` function).
4. Runs the existing transcribe → react pipeline.
5. Sets `is_speaking = true` before TTS playback, `is_speaking = false` after.
6. After the pipeline completes, if `wake_word_enabled` is still `true`, re-enters `Standby` and restarts the detection loop (which subscribes to the same
   still-open `SharedAudioSource`).

**Why not stop/restart the cpal stream?** On Linux (ALSA/PipeWire), closing and reopening an input stream can take 500 ms – 1 s due to device negotiation and
buffer reallocation. This delay would make the wake word experience feel sluggish. By keeping the stream open and using a split buffer, the transition from
Standby → Listening is effectively zero-latency.

```rust
async fn start_wake_word_detection(
    // ... parameters ...
) {
    // Open the shared cpal stream ONCE. This stream stays open for the
    // entire lifetime of wake word mode and is never restarted.
    let shared_audio_source = SharedAudioSource::new();
    // ... cpal stream setup, callback calls shared_audio_source.push_samples() ...

    if let Ok(mut guard) = shared_audio_source_state.lock() {
        *guard = Some(shared_audio_source.clone());
    }

    let detector = match WakeWordDetector::new(wake_word_config.clone()) {
        Ok(d) => d,
        Err(e) => {
            error!("Voice Assistant: Failed to start wake word detector: {e}");
            Self::set_error(state, &e.to_string(), answer).await;
            return;
        }
    };

    // Detector subscribes to the shared audio source.
    // The cpal stream is NOT passed to the detector — it reads from the ring buffer.
    let (mut event_rx, stop_tx) = detector.start(&shared_audio_source, is_speaking.clone());

    // Store handle
    if let Ok(mut guard) = wake_word_detector.lock() {
        *guard = Some(WakeWordDetectorHandle {
            stop_sender: stop_tx,
            event_receiver: None, // event_rx is moved into the spawn below
        });
    }

    // Spawn event listener
    let wake_word_enabled = wake_word_enabled.clone();
    let shared_audio_source_clone = shared_audio_source.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                WakeWordEvent::Detected { probability } => {
                    debug!("Voice Assistant: Wake word detected (probability: {probability:.3})");

                    // Do NOT stop the cpal stream. The detection loop has already
                    // exited; the shared audio source continues receiving samples.

                    // Subscribe a new consumer for pipeline audio collection.
                    let pipeline_consumer = shared_audio_source_clone.subscribe();

                    // Run the full pipeline using the shared audio source
                    // instead of the standalone capture_audio function.
                    Self::set_state(state, AssistantState::Listening, status_sender, transcript, answer).await;
                    Self::run_pipeline_from_shared_audio(
                        pipeline_consumer,
                        config, state, whisper, llm, worker,
                        entity_store, semantic_memory, conversation_history,
                        tool_router, resource_router, prompt_router,
                        training_mode, active_trace, training_history,
                        transcript, answer, response_type, active,
                        tts, is_speaking, // passed to set is_speaking during TTS
                        pending, pending_resources, pending_prompts,
                        tool_catalog, resource_catalog, prompt_catalog,
                        core_context, meta, status_sender, performance_monitor,
                    ).await;

                    // After pipeline, re-enter Standby if still enabled.
                    // The cpal stream is still open — we just restart the detector,
                    // which subscribes to the same shared audio source.
                    let still_enabled = wake_word_enabled.lock()
                        .map(|g| *g)
                        .unwrap_or(false);
                    if still_enabled {
                        Self::set_state(state, AssistantState::Standby, status_sender, transcript, answer).await;
                        // Re-create and restart detector (subscribes to same shared source)
                        // ...
                    }
                }
                WakeWordEvent::Error(e) => {
                    error!("Voice Assistant: Wake word detector error: {e}");
                    Self::set_error(state, &e.to_string(), answer).await;
                }
            }
        }
    });
}
```

### 7.4 `run_pipeline_from_shared_audio`

A variant of `run_pipeline_inner` that reads audio from the shared audio source's ring buffer instead of opening a new cpal stream. The silence detection, RMS
check, and transcription logic remain identical:

```rust
/// Runs the pipeline using audio from the shared audio source.
/// This is used when wake word mode is active and the cpal stream is already open.
async fn run_pipeline_from_shared_audio(
    mut consumer: ringbuf::consumer::Consumer<f32>,
    // ... all other pipeline parameters ...
    is_speaking: Arc<Mutex<bool>>,
) {
    // 1. Collect audio from ring buffer (replaces capture_audio)
    let samples = collect_audio_from_consumer(&mut consumer, config).await;

    // 2. RMS check (same as run_pipeline_inner)
    let rms = compute_rms(&samples);
    if rms < MIN_AUDIO_RMS { /* skip, return to standby */ }

    // 3. Transcribe (same as run_pipeline_inner)
    let transcribed = transcribe_async(whisper_ctx, samples, config.language.clone()).await;

    // 4. ReAct loop (same as run_pipeline_inner)
    Self::run_react(/* ... */).await;

    // 5. TTS playback — set is_speaking flag
    if let Some(tts) = tts_engine.as_ref() {
        if tts_allowed {
            *is_speaking.lock().unwrap() = true;
            Self::set_state(state, AssistantState::Speaking, /* ... */).await;
            if let Err(error) = tts.speak(&final_answer) { /* ... */ }
            *is_speaking.lock().unwrap() = false;
        }
    }
}
```

---

## 8. Configuration

### Additions to `VoiceAssistantServiceConfig`

In `services/voice_assistant/src/config.rs`:

```rust
/// Configuration for wake word detection.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct WakeWordServiceConfig {
    /// Which wake word model to use.
    pub model_type: WakeWordModelType,
    /// Detection threshold (0.0–1.0). Lower = more sensitive.
    pub threshold: f32,
    /// Whether wake word mode is enabled on startup.
    pub auto_enable: bool,
}
```

Add to `VoiceAssistantServiceConfig`:

```rust
pub struct VoiceAssistantServiceConfig {
    // ... existing fields ...

    /// Wake word detection configuration.
    pub wake_word: WakeWordServiceConfig,
}
```

### Example `config.toml` Section

```toml
[[services]]
name = "voice-assistant"
type = "voice_assistant"

[wake_word]
model_type = "alexa"        # "alexa", "hey_mycroft", or { custom = "path/to/model.onnx" }
threshold = 0.1             # 0.0–1.0, lower = more sensitive
auto_enable = false         # Set to true for hands-free startup
```

---

## 9. Model Management

### Built-in Models

oww-rs embeds the `alexa` and `hey_mycroft` models at compile time via `rust-embed`. No external `.onnx` files are needed. The models are:

- **melspectrogram** front-end (shared across all wake words)
- **speech-embedding** model (shared across all wake words)
- **wakeword-classifier** (per wake word, ~1–2 MB each)

These are loaded into `tract` inference sessions internally by `OwwModel::new()`.

### Custom Models

For custom wake words trained with the upstream openWakeWord Python project:

1. Export the classifier to `.onnx`.
2. Place the file in the project's `models/` directory.
3. Reference it in config: `model_type = { custom = "models/my_wake_word.onnx" }`.
4. The `WakeWordDetector::new()` function calls `new_model(path, threshold)` for custom models.

### Memory Considerations

| Component                   | Engine              | Memory     | GPU                  |
|-----------------------------|---------------------|------------|----------------------|
| Wake word detector (oww-rs) | tract-onnx (CPU)    | ~5–10 MB   | No                   |
| TTS (Piper/Kokoro)          | ort (GPU possible)  | ~50–200 MB | Yes (ROCm/CUDA)      |
| SemanticMemory (fastembed)  | ort (CPU)           | ~50 MB     | No                   |
| Whisper (whisper-rs)        | GGML (GPU possible) | ~75–500 MB | Yes (Vulkan/HIPBLAS) |
| LLM (llama-cpp-4)           | GGML (GPU possible) | ~1–4 GB    | Yes (Vulkan/HIP)     |

Since oww-rs uses `tract` (pure Rust, CPU-only), there is **no conflict** with the existing `ort` GPU execution providers. The wake word detector runs entirely
on CPU alongside the GPU-accelerated TTS and Whisper. No `SessionOptions` sharing is needed or possible.

---

## 10. Dependencies

### New Crate Dependencies

Add to `services/voice_assistant/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
oww-rs = "0.3"
ringbuf = "0.5"
```

### Feature Flags

No new Cargo features needed. oww-rs uses `tract-onnx` which is a pure-Rust dependency with no native libraries.

The existing `ort-rocm` / `ort-cuda` features remain unaffected since oww-rs does not use `ort`.

---

## 11. File Additions

| File                                            | Purpose                                                                                                                                                          |
|-------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `services/voice_assistant/src/wake_word.rs`     | `WakeWordDetector`, `WakeWordEvent`, `WakeWordConfig`, `WakeWordModelType`, `WakeWordError`                                                                      |
| `services/voice_assistant/src/lib.rs`           | Add `pub(crate) mod wake_word;`                                                                                                                                  |
| `services/voice_assistant/src/service.rs`       | Add `wake_word_enabled`, `wake_word_detector`, `wake_word_config` fields; handle `EnableWakeWord` / `DisableWakeWord` actions; add `start_wake_word_detection()` |
| `services/voice_assistant/src/config.rs`        | Add `WakeWordServiceConfig` struct and field in `VoiceAssistantServiceConfig`                                                                                    |
| `model/voice_assistant/src/messages/state.rs`   | Add `Standby` variant to `AssistantState`                                                                                                                        |
| `model/voice_assistant/src/messages/command.rs` | Add `EnableWakeWord` and `DisableWakeWord` variants to `VoiceCommandAction`                                                                                      |
| `services/voice_assistant/Cargo.toml`           | Add `oww-rs` and `ringbuf` dependencies                                                                                                                          |

---

## 12. MCP Tool Integration

### `voice_assistant_enable_wake_word`

**Description:** Enables wake word detection mode. The assistant enters Standby state and continuously listens for the configured wake word.

**Input schema:**

```json
{
  "type": "object",
  "properties": {}
}
```

**Behavior:**

- Sets `wake_word_enabled` to `true`.
- Transitions to `Standby` state.
- Starts the `WakeWordDetector` detection loop.
- Returns `{"status": "ok"}`.

### `voice_assistant_disable_wake_word`

**Description:** Disables wake word detection mode and returns the assistant to idle.

**Input schema:**

```json
{
  "type": "object",
  "properties": {}
}
```

**Behavior:**

- Sets `wake_word_enabled` to `false`.
- Stops the `WakeWordDetector` detection loop.
- Transitions to `Idle` state.
- Returns `{"status": "ok"}`.

Both tools are registered in `services/voice_assistant/src/mcp.rs` as `RegisterToolMessage` broadcasts and handled in the `InvokeToolMessage` handler.

---

## 13. Security & Privacy

- **Always-on microphone**: Wake word mode keeps the microphone continuously active. This is a privacy-sensitive feature. It must be opt-in via config
  (`wake_word.auto_enable = false` by default) or explicit MCP tool call.
- **No audio storage**: The ring buffer is in-memory only. Audio samples are discarded after detection. No audio is persisted to disk.
- **No network transmission**: All detection runs locally via `tract-onnx`. No audio or detection data leaves the device.
- **Visual indicator**: The widget should display a distinct icon when in `Standby` state (e.g., a pulsing microphone) so the user knows the microphone is
  active.

---

## 14. Edge Cases & Error Handling

| Scenario                                         | Handling                                                                                                                    |
|--------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------|
| No microphone available                          | `WakeWordError::AudioDevice` → set `Error` state, log error, do not retry                                                   |
| Ring buffer underrun                             | Detection loop sleeps 10 ms and retries; no error                                                                           |
| Ring buffer overrun                              | `push_overwrite` discards oldest samples; detection may miss a frame but recovers                                           |
| Detection loop crashes                           | `WakeWordEvent::Error` → set `Error` state, log error                                                                       |
| Pipeline already running when wake word detected | Check `active` flag; if already active, ignore detection (oww-rs refractory window handles most cases)                      |
| TTS self-trigger (assistant hears its own voice) | `is_speaking` flag suppresses detection during TTS playback; detection loop skips frames while flag is `true`               |
| cpal stream open/close latency on Linux          | Shared audio source keeps stream open across all transitions; no restart needed                                             |
| Detection stutter under high LLM load            | Detection thread spawned with `SCHED_FIFO` priority 5 (best-effort, falls back to default scheduler without `CAP_SYS_NICE`) |
| User deactivates during pipeline                 | After pipeline completes, check `wake_word_enabled`; if false, go to `Idle` instead of `Standby`                            |
| Model file not found (custom model)              | `WakeWordError::ModelInit` → set `Error` state, log error                                                                   |

---

## 15. Future Extensions

- **Multiple wake words**: Run multiple `OwwModel` instances in parallel and trigger on any detection.
- **Custom wake word training**: Integrate with the upstream openWakeWord training pipeline to create personalized wake words.
- **VAD pre-filtering**: Add a lightweight Voice Activity Detection (VAD) stage before the wake word detector to reduce CPU usage during silence.
- **Wake word + hotword combination**: Support both wake word activation and follow-up hotword commands (e.g., "Alexa, what's the weather?" in a single
  utterance).
- **GPU-accelerated detection**: Port the openWakeWord models to `ort` directly if GPU acceleration becomes necessary (currently CPU is sufficient).
- **Configurable refractory period**: Allow the user to configure the cooldown between detections to avoid re-triggering.
