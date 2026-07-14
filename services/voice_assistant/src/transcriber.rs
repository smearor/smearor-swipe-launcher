use std::sync::Arc;
use tracing::debug;
use whisper_rs::FullParams;
use whisper_rs::SamplingStrategy;
use whisper_rs::WhisperContext;
use whisper_rs::WhisperContextParameters;

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
}

/// Loads the Whisper model from the configured path and returns a shared context.
/// Configures GPU acceleration and flash attention when enabled via cargo features.
pub fn load_whisper_context(model_path: &str) -> Result<Arc<WhisperContext>, SttError> {
    debug!("Loading Whisper model from: {}", model_path);
    let mut params = WhisperContextParameters::new();

    #[cfg(any(feature = "whisper-cuda", feature = "whisper-hipblas", feature = "whisper-vulkan"))]
    {
        params.use_gpu(true);
        params.flash_attn(true);
        debug!("Whisper: GPU acceleration enabled (flash_attn=true)");
    }

    #[cfg(not(any(feature = "whisper-cuda", feature = "whisper-hipblas", feature = "whisper-vulkan")))]
    {
        params.use_gpu(false);
        debug!("Whisper: using CPU mode");
    }

    let context = WhisperContext::new_with_params(model_path, params).map_err(|error| SttError::StateCreation(error.to_string()))?;
    debug!("Whisper model loaded successfully");
    Ok(Arc::new(context))
}

/// Transcribes a PCM audio buffer into text using the Whisper model.
///
/// The audio buffer must contain 32-bit float samples at 16 kHz mono.
/// This function is synchronous and CPU-bound. It should be called from
/// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
pub fn transcribe(whisper_context: &WhisperContext, samples: &[f32], language: &str) -> Result<String, SttError> {
    if samples.len() < 1600 {
        return Err(SttError::BufferTooShort(samples.len()));
    }

    let mut state = whisper_context.create_state().map_err(|error| SttError::StateCreation(error.to_string()))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    let n_threads = std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(4);
    params.set_n_threads(n_threads);

    debug!(
        "Starting Whisper transcription: {} samples ({}s), language={}",
        samples.len(),
        samples.len() / 16000,
        language
    );

    state.full(params, samples).map_err(|error| SttError::ModelRun(error.to_string()))?;

    let mut transcript = String::new();
    let num_segments = state.full_n_segments();
    for index in 0..num_segments {
        let segment = match state.get_segment(index) {
            Some(seg) => seg,
            None => continue,
        };
        let segment_text = segment.to_str().map_err(|error| SttError::SegmentRetrieval(error.to_string()))?;
        if !transcript.is_empty() {
            transcript.push(' ');
        }
        transcript.push_str(segment_text.trim());
    }

    debug!("Whisper transcription complete: {} characters", transcript.len());
    Ok(transcript)
}

/// Async wrapper for `transcribe`. Runs the synchronous Whisper inference
/// on a blocking thread pool to avoid stalling the async runtime.
pub async fn transcribe_async(whisper_context: Arc<WhisperContext>, samples: Vec<f32>, language: String) -> Result<String, SttError> {
    tokio::task::spawn_blocking(move || transcribe(&whisper_context, &samples, &language))
        .await
        .map_err(|join_error| SttError::ModelRun(format!("Blocking task failed: {join_error}")))?
}
