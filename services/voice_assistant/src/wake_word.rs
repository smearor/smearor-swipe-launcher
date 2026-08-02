use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use oww_rs::config::SpeechUnlockType;
use oww_rs::oww::OWW_MODEL_CHUNK_SIZE;
use oww_rs::oww::OwwModel;
use ringbuf::HeapRb;
use ringbuf::traits::Consumer;
use ringbuf::traits::Observer;
use ringbuf::traits::Producer;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::error;

use crate::config::WakeWordModelType;

/// Errors that can occur during wake word detection.
#[derive(Debug, thiserror::Error)]
pub enum WakeWordError {
    /// No default audio input device was found.
    #[error("No default input device")]
    NoInputDevice,
    /// Audio device configuration failed.
    #[error("Audio config error: {0}")]
    ConfigError(String),
    /// Audio stream creation or playback failed.
    #[error("Stream error: {0}")]
    StreamError(String),
    /// Wake word model initialization failed.
    #[error("Wake word model error: {0}")]
    ModelError(String),
    /// Audio thread initialization failed.
    #[error("Audio thread initialization failed")]
    ThreadInitFailed,
    /// Custom wake word model is not supported by the oww-rs API.
    #[error("Custom model not supported: {0}")]
    CustomModelNotSupported(String),
}

/// Event emitted when a wake word is detected.
#[derive(Debug, Clone)]
pub struct WakeWordEvent {
    /// Detection probability (0.0–1.0).
    pub probability: f32,
}

/// Consumer handle for reading samples from a shared audio ring buffer.
/// Each consumer has its own independent ring buffer, so reading from one
/// consumer does not affect others.
pub struct SharedAudioConsumer {
    buffer: Arc<Mutex<HeapRb<f32>>>,
}

impl SharedAudioConsumer {
    /// Pops up to `chunk_size` samples from the ring buffer.
    /// Returns an empty vector if no samples are available.
    pub fn pop_chunk(&self, chunk_size: usize) -> Vec<f32> {
        let mut buf = match self.buffer.lock() {
            Ok(buf) => buf,
            Err(_) => return Vec::new(),
        };
        let mut chunk = Vec::with_capacity(chunk_size);
        for _ in 0..chunk_size {
            match buf.try_pop() {
                Some(sample) => chunk.push(sample),
                None => break,
            }
        }
        chunk
    }

    /// Drains all available samples from the ring buffer, discarding them.
    pub fn drain(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            while buf.try_pop().is_some() {}
        }
    }

    /// Returns the number of samples currently available in the buffer.
    pub fn available(&self) -> usize {
        self.buffer.lock().map(|buf| buf.occupied_len()).unwrap_or(0)
    }
}

/// Handle to the shared audio source. The audio stream runs on a dedicated
/// thread and is kept alive for the lifetime of this handle.
pub struct SharedAudioHandle {
    consumers: Vec<Arc<Mutex<HeapRb<f32>>>>,
    sample_rate: u32,
    stop_tx: Option<std::sync::mpsc::Sender<()>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl SharedAudioHandle {
    /// Returns a consumer handle for the given index.
    pub fn get_consumer(&self, index: usize) -> Option<SharedAudioConsumer> {
        self.consumers.get(index).map(|buffer| SharedAudioConsumer { buffer: buffer.clone() })
    }

    /// Returns the sample rate of the captured audio.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Stops the audio thread and releases the cpal stream.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SharedAudioHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts a shared audio source with continuous cpal capture.
///
/// Creates `num_consumers` independent ring buffers, each of `buffer_capacity`
/// samples. The cpal input stream runs on a dedicated thread and pushes
/// resampled 16 kHz mono f32 samples to all ring buffers simultaneously.
///
/// Returns a handle that manages the audio thread and provides consumer access.
pub fn start_shared_audio(target_sample_rate: u32, channels: u16, buffer_capacity: usize, num_consumers: usize) -> Result<SharedAudioHandle, WakeWordError> {
    let consumers: Vec<Arc<Mutex<HeapRb<f32>>>> = (0..num_consumers).map(|_| Arc::new(Mutex::new(HeapRb::<f32>::new(buffer_capacity)))).collect();

    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), WakeWordError>>();

    let consumers_clone = consumers.clone();
    let join_handle = std::thread::Builder::new()
        .name("shared-audio".to_string())
        .spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(device) => device,
                None => {
                    let _ = init_tx.send(Err(WakeWordError::NoInputDevice));
                    return;
                }
            };

            let supported_config = match device.default_input_config() {
                Ok(config) => config,
                Err(error) => {
                    let _ = init_tx.send(Err(WakeWordError::ConfigError(error.to_string())));
                    return;
                }
            };

            let native_rate = supported_config.sample_rate();
            let stream_config = cpal::StreamConfig {
                channels,
                sample_rate: native_rate,
                buffer_size: cpal::BufferSize::Default,
            };

            let native_rate_val = native_rate;
            let target_rate = target_sample_rate;
            let channels_count = channels as usize;
            let consumers = consumers_clone;

            let stream = match device.build_input_stream(
                stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let frame_count = data.len() / channels_count.max(1);
                    let mut mono = Vec::with_capacity(frame_count);
                    for frame in 0..frame_count {
                        let mut sum = 0.0f32;
                        for channel in 0..channels_count {
                            sum += data[frame * channels_count + channel];
                        }
                        mono.push(sum / channels_count as f32);
                    }

                    let samples = if native_rate_val != target_rate {
                        resample_linear(&mono, native_rate_val, target_rate)
                    } else {
                        mono
                    };

                    for consumer in &consumers {
                        if let Ok(mut buf) = consumer.lock() {
                            for &sample in &samples {
                                let _ = buf.try_push(sample);
                            }
                        }
                    }
                },
                move |stream_error| {
                    error!("Shared audio stream error: {stream_error}");
                },
                None,
            ) {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = init_tx.send(Err(WakeWordError::StreamError(error.to_string())));
                    return;
                }
            };

            if let Err(error) = stream.play() {
                let _ = init_tx.send(Err(WakeWordError::StreamError(error.to_string())));
                return;
            }
            debug!("Shared audio source: stream started at {} Hz", target_sample_rate);

            let _ = init_tx.send(Ok(()));

            let _ = stop_rx.recv();
            debug!("Shared audio source: stream stopping");
        })
        .map_err(|_| WakeWordError::ThreadInitFailed)?;

    match init_rx.recv() {
        Ok(Ok(())) => Ok(SharedAudioHandle {
            consumers,
            sample_rate: target_sample_rate,
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }),
        Ok(Err(error)) => {
            let _ = join_handle.join();
            Err(error)
        }
        Err(_) => {
            let _ = join_handle.join();
            Err(WakeWordError::ThreadInitFailed)
        }
    }
}

/// Handle to a running wake word detection thread.
pub struct WakeWordDetectorHandle {
    stop_tx: Option<tokio::sync::mpsc::UnboundedSender<()>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl WakeWordDetectorHandle {
    /// Stops the detection thread.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WakeWordDetectorHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the wake word detection loop on a dedicated thread.
///
/// The detector reads 1280-sample (80 ms) chunks from the shared audio
/// consumer and runs oww-rs inference on each chunk. When a wake word is
/// detected, a `WakeWordEvent` is sent via `event_tx`.
///
/// The `is_speaking` flag is checked before each detection cycle to avoid
/// TTS self-triggering.
pub fn start_wake_word_detection(
    consumer: SharedAudioConsumer,
    model_type: WakeWordModelType,
    model_path: String,
    threshold: f32,
    is_speaking: Arc<Mutex<bool>>,
    event_tx: tokio::sync::mpsc::UnboundedSender<WakeWordEvent>,
) -> Result<WakeWordDetectorHandle, WakeWordError> {
    let (stop_tx, mut stop_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), WakeWordError>>();

    let join_handle = std::thread::Builder::new()
        .name("wake-word".to_string())
        .spawn(move || {
            let mut model = match create_wake_word_model(&model_type, &model_path, threshold) {
                Ok(model) => model,
                Err(error) => {
                    let _ = init_tx.send(Err(error));
                    return;
                }
            };
            let _ = init_tx.send(Ok(()));
            debug!("Wake word detection loop started (model: {:?}, threshold: {})", model_type, threshold);

            let mut frame_buffer: Vec<f32> = Vec::with_capacity(OWW_MODEL_CHUNK_SIZE);

            loop {
                if stop_rx.try_recv().is_ok() {
                    debug!("Wake word detection loop stopping");
                    break;
                }

                if let Ok(speaking) = is_speaking.lock() {
                    if *speaking {
                        consumer.drain();
                        std::thread::sleep(std::time::Duration::from_millis(80));
                        continue;
                    }
                }

                let chunk = consumer.pop_chunk(OWW_MODEL_CHUNK_SIZE);
                if chunk.is_empty() {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    continue;
                }

                frame_buffer.extend_from_slice(&chunk);

                while frame_buffer.len() >= OWW_MODEL_CHUNK_SIZE {
                    let frame: Vec<f32> = frame_buffer.drain(..OWW_MODEL_CHUNK_SIZE).collect();
                    let detection = model.detection(frame);
                    if detection.detected {
                        debug!("Wake word detected! probability: {:.3}", detection.probability);
                        let _ = event_tx.send(WakeWordEvent {
                            probability: detection.probability,
                        });
                    }
                }
            }
            debug!("Wake word detection loop ended");
        })
        .map_err(|_| WakeWordError::ThreadInitFailed)?;

    match init_rx.recv() {
        Ok(Ok(())) => Ok(WakeWordDetectorHandle {
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }),
        Ok(Err(error)) => {
            let _ = join_handle.join();
            Err(error)
        }
        Err(_) => {
            let _ = join_handle.join();
            Err(WakeWordError::ThreadInitFailed)
        }
    }
}

/// Creates an oww-rs model for the given model type and threshold.
fn create_wake_word_model(model_type: &WakeWordModelType, _model_path: &str, threshold: f32) -> Result<OwwModel, WakeWordError> {
    match model_type {
        WakeWordModelType::Alexa => OwwModel::new(SpeechUnlockType::OpenWakeWordAlexa, threshold).map_err(|error| WakeWordError::ModelError(error.to_string())),
        WakeWordModelType::HeyMycroft => {
            OwwModel::new(SpeechUnlockType::OpenWakeWordHeyMycroft, threshold).map_err(|error| WakeWordError::ModelError(error.to_string()))
        }
        WakeWordModelType::Custom => Err(WakeWordError::CustomModelNotSupported(
            "oww-rs does not expose a public API for loading custom ONNX models. Use Alexa or HeyMycroft.".to_string(),
        )),
    }
}

/// Resamples a PCM buffer from one sample rate to another using linear interpolation.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
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
