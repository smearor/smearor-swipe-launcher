use cpal::BufferSize;
use cpal::SampleFormat;
use cpal::StreamConfig;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tracing::debug;

use crate::config::VoiceAssistantServiceConfig;

/// The RMS amplitude threshold below which audio is considered "silence".
const SILENCE_RMS_THRESHOLD: f32 = 0.01;

/// Errors that can occur during audio capture.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No default input device was found on the system.
    #[error("No default input device found")]
    NoDefaultInputDevice,
    /// Failed to query the default input configuration from the device.
    #[error("Failed to get default input config: {0}")]
    DefaultInputConfig(String),
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

/// Captures audio from the default input device until silence is detected,
/// the max recording duration is reached, or a stop signal is received.
///
/// Returns a buffer of 32-bit float PCM samples at 16 kHz mono, suitable
/// for direct ingestion by `whisper-rs`.
pub async fn capture_audio(config: &VoiceAssistantServiceConfig, stop_rx: oneshot::Receiver<()>) -> Result<Vec<f32>, AudioError> {
    let (done_tx, done_rx) = oneshot::channel::<Result<Vec<f32>, AudioError>>();

    let config = config.clone();

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
        debug!("Audio capture: selected input device: {}", device);

        let supported_config = match device.default_input_config() {
            Ok(config) => config,
            Err(error) => {
                let _ = done_tx.send(Err(AudioError::DefaultInputConfig(error.to_string())));
                return;
            }
        };

        // We require f32 samples. If the device does not support f32 natively,
        // we attempt to request it anyway — cpal will convert internally on most platforms.
        if supported_config.sample_format() != SampleFormat::F32 {
            debug!(
                "Audio capture: device native format is {:?}, requesting F32 with conversion",
                supported_config.sample_format()
            );
        }

        // Determine the actual capture sample rate.
        let native_sample_rate = supported_config.sample_rate();
        let needs_resampling = native_sample_rate != config.audio_sample_rate;
        if needs_resampling {
            debug!(
                "Audio capture: native sample rate is {} Hz, will resample to {} Hz after capture",
                native_sample_rate, config.audio_sample_rate
            );
        }

        let actual_sample_rate = native_sample_rate;
        let max_samples = (config.max_recording_seconds as usize) * (actual_sample_rate as usize);
        let silence_window_samples = (config.silence_threshold_seconds as usize) * (actual_sample_rate as usize);

        let stream_config = StreamConfig {
            channels: config.audio_channels,
            sample_rate: actual_sample_rate,
            buffer_size: BufferSize::Default,
        };

        // Shared buffer for captured mono samples.
        let max_mono_samples = max_samples / (config.audio_channels as usize);
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(max_mono_samples)));
        let buffer_clone = buffer.clone();

        // Track consecutive silence samples for silence detection.
        let consecutive_silence: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let consecutive_silence_clone = consecutive_silence.clone();

        // Stream error flag: set by the cpal error callback to signal
        // the outer loop that the stream has failed.
        let stream_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let stream_error_clone = stream_error.clone();

        let channels = config.audio_channels as usize;

        let stream = device.build_input_stream(
            stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap_or_else(|e| e.into_inner());
                let mut silence_count = consecutive_silence_clone.lock().unwrap_or_else(|e| e.into_inner());

                // Block-wise mono downmix: average all channels per frame.
                let frame_count = data.len() / channels;
                for frame in 0..frame_count {
                    if buf.len() >= max_mono_samples {
                        break;
                    }

                    let mut sum = 0.0f32;
                    for channel in 0..channels {
                        sum += data[frame * channels + channel];
                    }
                    let mono_sample = sum / (channels as f32);
                    buf.push(mono_sample);

                    // Silence detection on individual samples.
                    if mono_sample.abs() < SILENCE_RMS_THRESHOLD {
                        *silence_count += 1;
                    } else {
                        *silence_count = 0;
                    }
                }

                if *silence_count >= silence_window_samples && buf.len() > silence_window_samples {
                    debug!(
                        "Audio capture: silence detected after {} mono samples ({}s)",
                        buf.len(),
                        buf.len() / (actual_sample_rate as usize)
                    );
                }
            },
            move |error| {
                tracing::error!("Audio capture stream error: {}", error);
                if let Ok(mut err) = stream_error_clone.lock() {
                    *err = Some(error.to_string());
                }
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

        // Wait for either: stop signal, silence, max duration, or stream error.
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

            // Check if the stream has errored.
            if let Ok(mut err) = stream_error.lock() {
                if let Some(error_msg) = err.take() {
                    debug!("Audio capture: stream error detected, aborting: {}", error_msg);
                    let _ = done_tx.send(Err(AudioError::StreamBuild(error_msg)));
                    return;
                }
            }

            // Check if max duration reached.
            let current_len = buffer.lock().map(|buf| buf.len()).unwrap_or(0);
            if current_len >= max_mono_samples {
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

        // Extract the captured mono samples.
        let samples = buffer.lock().map(|mut buf| std::mem::take(&mut *buf)).unwrap_or_default();

        if samples.is_empty() {
            let _ = done_tx.send(Err(AudioError::EmptyBuffer));
        } else {
            debug!(
                "Audio capture: captured {} mono samples at {} Hz ({}s)",
                samples.len(),
                actual_sample_rate,
                samples.len() / (actual_sample_rate as usize)
            );

            // Resampling: if the capture rate differs from the target rate,
            // apply linear resampling. Mono downmix was already done in the callback.
            let final_samples = if needs_resampling {
                debug!("Audio capture: resampling from {} Hz to {} Hz", actual_sample_rate, config.audio_sample_rate);
                resample_linear(&samples, actual_sample_rate, config.audio_sample_rate)
            } else {
                samples
            };

            debug!("Audio capture: {} samples after processing ({} Hz mono)", final_samples.len(), config.audio_sample_rate);
            let _ = done_tx.send(Ok(final_samples));
        }
    });

    let result = done_rx.await.map_err(|_| AudioError::Cancelled)?;
    result
}

/// Computes the root-mean-square (RMS) amplitude of a PCM sample buffer.
/// Returns 0.0 for empty buffers. Useful for detecting whether captured
/// audio contains meaningful speech or is merely background noise/silence.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    (sum_squares / samples.len() as f64).sqrt() as f32
}

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
