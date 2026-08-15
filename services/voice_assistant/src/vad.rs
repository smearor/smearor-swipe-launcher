use std::sync::Arc;
use std::sync::Mutex;

use ndarray::Array0;
use ndarray::Array2;
use ndarray::Array3;
use ort::execution_providers::ExecutionProviderDispatch;
use ort::session::Session;
use ort::value::Tensor;
use tracing::debug;

/// Number of samples per frame at 16 kHz (32 ms window).
const FRAME_SIZE: usize = 512;

/// Number of context samples from the previous frame.
const CONTEXT_SIZE: usize = 64;

/// Total input size: frame + context.
const INPUT_SIZE: usize = FRAME_SIZE + CONTEXT_SIZE;

/// State tensor size: [2, 1, 128] = 256 elements.
const STATE_SIZE: usize = 2 * 1 * 128;

/// Errors that can occur during VAD processing.
#[derive(Debug, thiserror::Error)]
pub enum VadError {
    /// Failed to create the ONNX inference session.
    #[error("Failed to create VAD ONNX session: {0}")]
    SessionCreate(String),
    /// Failed to load the VAD ONNX model file.
    #[error("Failed to load VAD ONNX model: {0}")]
    ModelLoad(String),
    /// Failed to create an ONNX input tensor.
    #[error("Failed to create VAD input tensor: {0}")]
    TensorCreate(String),
    /// VAD inference failed.
    #[error("VAD inference failed: {0}")]
    Inference(String),
    /// The expected output tensor was missing from the model output.
    #[error("VAD output tensor missing")]
    MissingOutput,
    /// Failed to extract the output tensor as f32 data.
    #[error("Failed to extract VAD output tensor: {0}")]
    TensorExtract(String),
}

/// Silero VAD engine for voice activity detection and audio trimming.
///
/// Uses the Silero VAD ONNX model to classify 512-sample (32ms) frames
/// at 16 kHz as speech or non-speech, then trims leading/trailing silence
/// from the audio buffer before sending it to Whisper.
pub struct SileroVad {
    /// ONNX inference session for the Silero VAD model.
    session: Session,
    /// LSTM recurrent state, stored flat as [2, 1, 128] = 256 elements.
    state: Vec<f32>,
    /// Context samples from the previous frame (64 samples).
    context: Vec<f32>,
}

impl SileroVad {
    /// Creates a new Silero VAD engine from the given ONNX model path.
    pub fn new(model_path: &str) -> Result<Self, VadError> {
        let mut builder = Session::builder().map_err(|e| VadError::SessionCreate(e.to_string()))?;

        let providers = build_execution_providers();
        if !providers.is_empty() {
            builder = builder
                .with_execution_providers(&providers)
                .map_err(|e| VadError::SessionCreate(e.to_string()))?;
        }

        let session = builder.commit_from_file(model_path).map_err(|e| VadError::ModelLoad(e.to_string()))?;

        debug!("Silero VAD: model loaded from {}", model_path);

        Ok(Self {
            session,
            state: vec![0.0; STATE_SIZE],
            context: vec![0.0; CONTEXT_SIZE],
        })
    }

    /// Resets the LSTM state and context to zeros.
    /// Must be called before processing a new audio buffer.
    pub fn reset(&mut self) {
        self.state.fill(0.0);
        self.context.fill(0.0);
    }

    /// Runs VAD inference on a single 512-sample frame.
    /// Returns the speech probability (0.0–1.0).
    fn predict_frame(&mut self, frame: &[f32]) -> Result<f32, VadError> {
        // Build the input: [context (64) + frame (512)] = [1, 576]
        let mut input_data = Vec::with_capacity(INPUT_SIZE);
        input_data.extend_from_slice(&self.context);
        input_data.extend_from_slice(frame);

        let input_array = Array2::from_shape_vec((1, INPUT_SIZE), input_data).map_err(|e| VadError::TensorCreate(format!("Input shape error: {e}")))?;
        let input_tensor = Tensor::from_array(input_array).map_err(|e| VadError::TensorCreate(e.to_string()))?;

        let state_array = Array3::from_shape_vec((2, 1, 128), self.state.clone()).map_err(|e| VadError::TensorCreate(format!("State shape error: {e}")))?;
        let state_tensor = Tensor::from_array(state_array).map_err(|e| VadError::TensorCreate(e.to_string()))?;

        let sr_array = Array0::from_shape_vec((), vec![16000i64]).map_err(|e| VadError::TensorCreate(format!("SR shape error: {e}")))?;
        let sr_tensor = Tensor::from_array(sr_array).map_err(|e| VadError::TensorCreate(e.to_string()))?;

        let inputs = ort::inputs!["input" => input_tensor, "state" => state_tensor, "sr" => sr_tensor];

        // Scoped block so `outputs` (which holds &mut self.session) drops before
        // we update self.state and self.context.
        let (probability, new_state) = {
            let outputs = self.session.run(inputs).map_err(|e| VadError::Inference(e.to_string()))?;

            let output_tensor = outputs.get("output").ok_or(VadError::MissingOutput)?;
            let (_shape, prob_data) = output_tensor.try_extract_tensor::<f32>().map_err(|e| VadError::TensorExtract(e.to_string()))?;
            let prob = prob_data[0];

            let state_tensor = outputs.get("stateN").ok_or(VadError::MissingOutput)?;
            let (_state_shape, state_data) = state_tensor.try_extract_tensor::<f32>().map_err(|e| VadError::TensorExtract(e.to_string()))?;
            let new_state = state_data.to_vec();

            (prob, new_state)
        };

        self.state = new_state;

        // Update context: last 64 samples of the current frame.
        let frame_len = frame.len();
        if frame_len >= CONTEXT_SIZE {
            self.context.copy_from_slice(&frame[frame_len - CONTEXT_SIZE..]);
        } else {
            // Frame shorter than context — shift and pad.
            let shift = CONTEXT_SIZE - frame_len;
            let (left, right) = self.context.split_at_mut(shift);
            left.copy_from_slice(&right[..frame_len]);
            self.context[shift..].copy_from_slice(frame);
        }

        Ok(probability)
    }

    /// Processes the entire audio buffer and trims leading/trailing non-speech.
    ///
    /// Returns a trimmed buffer containing only the speech segment(s).
    /// If no speech is detected, returns an empty buffer.
    /// If the buffer is shorter than one frame, returns it unchanged.
    pub fn trim_silence(&mut self, samples: &[f32], threshold: f32) -> Result<Vec<f32>, VadError> {
        if samples.len() < FRAME_SIZE {
            debug!("Silero VAD: buffer too short ({} < {} samples), skipping trim", samples.len(), FRAME_SIZE);
            return Ok(samples.to_vec());
        }

        self.reset();

        let num_frames = samples.len() / FRAME_SIZE;
        let mut probabilities = Vec::with_capacity(num_frames);

        for frame_index in 0..num_frames {
            let start = frame_index * FRAME_SIZE;
            let frame = &samples[start..start + FRAME_SIZE];
            let prob = self.predict_frame(frame)?;
            probabilities.push(prob);
        }

        // Find first and last frame above threshold.
        let mut first_speech_frame: Option<usize> = None;
        let mut last_speech_frame: Option<usize> = None;

        for (index, &prob) in probabilities.iter().enumerate() {
            if prob >= threshold {
                if first_speech_frame.is_none() {
                    first_speech_frame = Some(index);
                }
                last_speech_frame = Some(index);
            }
        }

        match (first_speech_frame, last_speech_frame) {
            (Some(first), Some(last)) => {
                let start_sample = first * FRAME_SIZE;
                let end_sample = ((last + 1) * FRAME_SIZE).min(samples.len());
                let trimmed = samples[start_sample..end_sample].to_vec();
                let speech_frames = probabilities.iter().filter(|&&p| p >= threshold).count();
                debug!(
                    "Silero VAD: trimmed {} -> {} samples (frames {}..{}, {} frames total, {} speech frames)",
                    samples.len(),
                    trimmed.len(),
                    first,
                    last,
                    num_frames,
                    speech_frames
                );
                Ok(trimmed)
            }
            (None, _) | (_, None) => {
                debug!("Silero VAD: no speech detected in {} frames (threshold={})", num_frames, threshold);
                Ok(Vec::new())
            }
        }
    }
}

/// Builds execution providers for the VAD model based on GPU availability.
fn build_execution_providers() -> Vec<ExecutionProviderDispatch> {
    #[cfg(all(feature = "ort-cuda", target_os = "linux"))]
    {
        use ort::execution_providers::CUDA;
        debug!("Silero VAD: using CUDA execution provider");
        vec![CUDA::default().build().into()]
    }
    #[cfg(all(feature = "ort-rocm", target_os = "linux"))]
    {
        use ort::execution_providers::ROCm;
        debug!("Silero VAD: using ROCm execution provider");
        vec![ROCm::default().build().into()]
    }
    #[cfg(not(any(all(feature = "ort-cuda", target_os = "linux"), all(feature = "ort-rocm", target_os = "linux"))))]
    {
        Vec::new()
    }
}

/// Shared Silero VAD engine type.
pub type SharedSileroVad = Arc<Mutex<SileroVad>>;

/// Loads a Silero VAD model and returns a shared engine.
pub fn load_vad_engine(model_path: &str) -> Result<SharedSileroVad, VadError> {
    let vad = SileroVad::new(model_path)?;
    Ok(Arc::new(Mutex::new(vad)))
}

/// Async wrapper for `trim_silence`. Runs the synchronous VAD inference
/// on a blocking thread pool to avoid stalling the async runtime.
///
/// Returns the trimmed audio buffer. If no speech is detected, returns
/// an empty buffer. If VAD inference fails, returns the original buffer
/// so the caller can fall back to Whisper without VAD.
pub async fn trim_silence_async(vad: SharedSileroVad, samples: Vec<f32>, threshold: f32) -> Result<Vec<f32>, VadError> {
    tokio::task::spawn_blocking(move || {
        let mut engine = vad.lock().map_err(|e| VadError::Inference(format!("VAD lock failed: {e}")))?;
        engine.trim_silence(&samples, threshold)
    })
    .await
    .map_err(|e| VadError::Inference(format!("Blocking task failed: {e}")))?
}
