use fastembed::EmbeddingModel;
use fastembed::TextEmbedding;
use fastembed::TextInitOptions;
use moka::sync::Cache;
use ort::environment::Environment;
use ort::execution_providers::ExecutionProviderDispatch;
use ort::logging::LogLevel;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;

/// Suppresses verbose ONNX Runtime logging by setting the environment log level to Warning.
/// When the `tracing` feature is enabled (default), ort creates the environment with VERBOSE level,
/// causing all runtime messages to be forwarded to tracing. This overrides it to Warning.
fn suppress_ort_logging() {
    if let Ok(env) = Environment::current() {
        env.set_log_level(LogLevel::Warning);
    }
}

/// Default batch size for embedding generation.
const DEFAULT_EMBED_BATCH_SIZE: usize = 32;

/// Errors that can occur during embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    /// Embedding generation failed.
    #[error("Embedding failed: {0}")]
    Failed(String),
    /// No embedding was returned for the input text.
    #[error("No embedding returned")]
    NoResult,
    /// Model loading failed.
    #[error("Model load failed: {0}")]
    ModelLoad(String),
}

/// Shared embedding engine wrapping a fastembed `TextEmbedding` model.
/// Provides thread-safe embedding generation for tool/resource/prompt selection
/// and semantic memory. Reuses the same model instance across all consumers.
/// If the configured model (e.g. BGEM3) fails to load, falls back to
/// `BGESmallENV15Q` to ensure basic functionality.
pub struct EmbeddingEngine {
    model: Mutex<TextEmbedding>,
    /// Cache: text string -> embedding vector.
    /// Avoids re-embedding identical text across store/recall and selection calls.
    cache: Cache<String, Vec<f32>>,
    /// Name of the loaded embedding model.
    model_name: String,
    /// Whether the engine was loaded as a fallback.
    is_fallback: bool,
}

impl EmbeddingEngine {
    /// Creates a new embedding engine from a fastembed model.
    pub fn new(model: TextEmbedding) -> Self {
        let cache = Cache::builder().max_capacity(512).time_to_live(std::time::Duration::from_secs(3600)).build();
        Self {
            model: Mutex::new(model),
            cache,
            model_name: String::new(),
            is_fallback: false,
        }
    }

    /// Returns the name of the loaded embedding model.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Returns whether the engine was loaded as a fallback.
    pub fn is_fallback(&self) -> bool {
        self.is_fallback
    }

    /// Returns the current cache entry count.
    pub fn cache_entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Loads an embedding model with the configured name and execution providers.
    /// Falls back to `BGESmallENV15Q` if the configured model is unknown or fails to load.
    pub fn load(embedding_model: &str) -> Result<Self, EmbeddingError> {
        let model_name_enum = EmbeddingModel::from_str(embedding_model).unwrap_or_else(|_| {
            debug!("Embedding engine: unknown embedding model '{embedding_model}', falling back to BGESmallENV15Q");
            EmbeddingModel::BGESmallENV15Q
        });

        suppress_ort_logging();
        let execution_providers = build_execution_providers();
        let intra_threads = std::thread::available_parallelism().map(|n| n.get()).ok();
        let mut init_options = TextInitOptions::new(model_name_enum.clone()).with_execution_providers(execution_providers);
        if let Some(threads) = intra_threads {
            init_options = init_options.with_intra_threads(threads);
        }
        let model = TextEmbedding::try_new(init_options).map_err(|e| EmbeddingError::ModelLoad(e.to_string()))?;
        let resolved_name = format!("{:?}", model_name_enum);
        debug!("Embedding engine: model loaded: {resolved_name}");
        let mut engine = Self::new(model);
        engine.model_name = resolved_name;
        Ok(engine)
    }

    /// Creates a fallback engine with `BGESmallENV15Q`.
    /// Used when the configured model fails to load.
    pub fn fallback() -> Result<Self, EmbeddingError> {
        debug!("Embedding engine: loading fallback model BGESmallENV15Q");
        suppress_ort_logging();
        let execution_providers = build_execution_providers();
        let model = TextEmbedding::try_new(TextInitOptions::new(EmbeddingModel::BGESmallENV15Q).with_execution_providers(execution_providers))
            .map_err(|e| EmbeddingError::ModelLoad(e.to_string()))?;
        let mut engine = Self::new(model);
        engine.model_name = "BGESmallENV15Q".to_string();
        engine.is_fallback = true;
        Ok(engine)
    }

    /// Embeds a single text string, using the cache when possible.
    pub fn embed_single(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if let Some(cached) = self.cache.get(text) {
            return Ok(cached);
        }
        let mut model = self.model.lock().map_err(|e| EmbeddingError::Failed(e.to_string()))?;
        let embedding = model
            .embed(vec![text.to_string()], None)
            .map_err(|e| EmbeddingError::Failed(e.to_string()))?
            .into_iter()
            .next()
            .ok_or(EmbeddingError::NoResult)?;
        drop(model);
        self.cache.insert(text.to_string(), embedding.clone());
        Ok(embedding)
    }

    /// Embeds multiple texts in a single batch call for efficiency.
    /// Uses the default batch size for optimal throughput.
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut model = self.model.lock().map_err(|e| EmbeddingError::Failed(e.to_string()))?;
        let embeddings = model
            .embed(texts.to_vec(), Some(DEFAULT_EMBED_BATCH_SIZE))
            .map_err(|e| EmbeddingError::Failed(e.to_string()))?;
        drop(model);
        for (text, embedding) in texts.iter().zip(embeddings.iter()) {
            self.cache.insert(text.clone(), embedding.clone());
        }
        Ok(embeddings)
    }
}

/// Shared embedding engine type used across the service.
pub type SharedEmbeddingEngine = Arc<EmbeddingEngine>;

/// Builds execution providers for the embedding model based on GPU availability.
/// On Linux with CUDA/ROCm, uses GPU acceleration when available.
/// Falls back to CPU otherwise.
pub fn build_execution_providers() -> Vec<ExecutionProviderDispatch> {
    #[cfg(all(feature = "ort-cuda", target_os = "linux"))]
    {
        use ort::execution_providers::CUDA;
        debug!("Embedding engine: using CUDA execution provider");
        return vec![CUDA::default().build().into()];
    }
    #[cfg(all(feature = "ort-rocm", target_os = "linux"))]
    {
        use ort::execution_providers::ROCm;
        debug!("Embedding engine: using ROCm execution provider");
        return vec![ROCm::default().build().into()];
    }
    #[cfg(not(any(all(feature = "ort-cuda", target_os = "linux"), all(feature = "ort-rocm", target_os = "linux"))))]
    {
        Vec::new()
    }
}

/// Computes cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
