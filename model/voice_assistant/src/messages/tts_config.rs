use serde::Deserialize;
use serde::Serialize;

use crate::xdg_models_dir;

/// Supported TTS model types.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TtsModelType {
    /// Piper TTS (VITS architecture, lightweight, CPU-only).
    Piper,
    /// Kokoro-82M (high-quality, GPU-accelerated).
    Kokoro,
}

/// TTS phonemizer configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TtsPhonemizerConfig {
    /// BCP-47 language tag for phonemization (e.g., "de", "en").
    pub language: String,
}

impl Default for TtsPhonemizerConfig {
    fn default() -> Self {
        Self { language: "de".to_string() }
    }
}

/// TTS engine configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TtsConfig {
    /// Whether TTS is enabled at all.
    pub enabled: bool,
    /// Whether TTS is enabled for MCP text input path.
    pub tts_enabled_mcp: bool,
    /// Path to the ONNX model file.
    pub model_path: String,
    /// HuggingFace repo ID for auto-download of the ONNX model (optional).
    /// If empty, a hardcoded fallback mapping is used.
    #[serde(default)]
    pub model_repo: String,
    /// Path to the model config JSON file (phoneme map, etc.).
    pub config_path: String,
    /// TTS model type (Piper or Kokoro).
    pub model_type: TtsModelType,
    /// Native sample rate of the TTS model (e.g., 22050 for Piper, 24000 for Kokoro).
    pub model_sample_rate: u32,
    /// Phonemizer configuration.
    pub phonemizer_config: TtsPhonemizerConfig,
    /// Voice name for Kokoro (ignored for Piper).
    #[serde(default = "default_voice")]
    pub voice: String,
    /// Whether to use the LLM-based text_to_speech_answer conversion step.
    /// When true, the ReAct loop sends a [SYSTEM_ACTION] to the LLM after
    /// final_answer to convert numbers/symbols to spoken words.
    /// When false, the final_answer is passed directly to the TTS engine,
    /// which relies on espeak-ng's native number/time pronunciation.
    #[serde(default = "default_tts_conversion_step")]
    pub conversion_step: bool,
    /// Whether to use espeak-ng phonemization before ONNX inference.
    /// When true, text is converted to IPA phonemes via espeak-ng before
    /// being passed to the ONNX model.
    /// When false, the normalized text is passed directly as character codes
    /// to the ONNX model (useful for models with their own phonemizer).
    #[serde(default = "default_phonemize_enabled")]
    pub phonemize_enabled: bool,
    /// Whether to skip inserting pad_id between phoneme IDs.
    /// Some models (e.g., Kokoro) do not expect padding between tokens.
    #[serde(default = "default_disable_pad_id")]
    pub disable_pad_id: bool,
}

fn default_voice() -> String {
    "af_heart".to_string()
}

fn default_tts_conversion_step() -> bool {
    false
}

fn default_phonemize_enabled() -> bool {
    true
}

fn default_disable_pad_id() -> bool {
    false
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tts_enabled_mcp: false,
            model_path: format!("{}/de_DE-thorsten-medium.onnx", xdg_models_dir()),
            model_repo: String::new(),
            config_path: format!("{}/de_DE-thorsten-medium.onnx.json", xdg_models_dir()),
            model_type: TtsModelType::Piper,
            model_sample_rate: 22050,
            phonemizer_config: TtsPhonemizerConfig::default(),
            voice: default_voice(),
            conversion_step: default_tts_conversion_step(),
            phonemize_enabled: default_phonemize_enabled(),
            disable_pad_id: default_disable_pad_id(),
        }
    }
}
