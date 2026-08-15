use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use espeak_ng::EspeakNg;
use ort::execution_providers::ExecutionProviderDispatch;
use ort::session::Session;
use regex::Regex;
use smearor_voice_assistant_model::TtsConfig;
use smearor_voice_assistant_model::TtsModelType;
use text_processing_rs::tn_normalize_lang;
use tracing::debug;
use tracing::warn;
use unicode_normalization::UnicodeNormalization;

use crate::compound_trie::CompoundTrie;
use crate::compound_trie::build_german_compound_trie;

fn german_compound_trie() -> &'static CompoundTrie {
    static TRIE: OnceLock<CompoundTrie> = OnceLock::new();
    TRIE.get_or_init(build_german_compound_trie)
}

/// Errors that can occur during TTS synthesis or playback.
#[derive(Clone, Debug, thiserror::Error)]
pub enum TtsError {
    /// Failed to create the ONNX inference session.
    #[error("Failed to create ONNX session: {0}")]
    SessionCreate(String),
    /// Failed to load the ONNX model file.
    #[error("Failed to load ONNX model: {0}")]
    ModelLoad(String),
    /// No audio output device is available.
    #[error("No audio output device available")]
    NoOutputDevice,
    /// Failed to configure the audio output device.
    #[error("Failed to configure audio output: {0}")]
    AudioConfig(String),
    /// Failed to create an ONNX input tensor.
    #[error("Failed to create input tensor: {0}")]
    TensorCreate(String),
    /// ONNX inference failed.
    #[error("ONNX inference failed: {0}")]
    Inference(String),
    /// The expected output tensor was missing from the model output.
    #[error("Model output tensor missing")]
    MissingOutput,
    /// Failed to extract the output tensor as f32 data.
    #[error("Failed to extract output tensor: {0}")]
    TensorExtract(String),
    /// Failed to create the cpal audio output stream.
    #[error("Failed to create audio stream: {0}")]
    StreamCreate(String),
    /// Failed to start the cpal audio stream.
    #[error("Failed to start audio stream: {0}")]
    StreamPlay(String),
    /// The phonemizer failed to convert text to phonemes.
    #[error("Phonemizer error: {0}")]
    Phonemizer(String),
    /// Failed to load or parse the model config JSON.
    #[error("Model config error: {0}")]
    ModelConfig(String),
}

/// Playback state shared between the main thread and the cpal audio callback.
struct PlaybackState {
    /// Mono PCM samples to play.
    samples: Vec<f32>,
    /// Current read position in the samples vector.
    position: usize,
}

/// Text-to-Speech engine using ONNX models (Piper or Kokoro) and cpal for audio output.
pub struct TtsEngine {
    /// ONNX inference session for the TTS model.
    onnx_session: std::sync::Mutex<Session>,
    /// cpal audio output device.
    cpal_device: cpal::Device,
    /// cpal supported stream configuration.
    cpal_config: cpal::SupportedStreamConfig,
    /// TTS model type (Piper or Kokoro).
    model_type: TtsModelType,
    /// Native sample rate of the TTS model (e.g., 22050 for Piper, 24000 for Kokoro).
    model_sample_rate: u32,
    /// BCP-47 language tag for text normalization language detection.
    language: String,
    /// Persistent espeak-ng engine instance for phonemization and direct synthesis.
    espeak_engine: EspeakNg,
    /// Phoneme ID map loaded from the model's config JSON.
    phoneme_id_map: std::collections::HashMap<String, i64>,
    /// Whether to use espeak-ng phonemization before ONNX inference.
    phonemize_enabled: bool,
    /// Whether to skip inserting pad_id between phoneme IDs.
    disable_pad_id: bool,
    /// Cancellation flag — when set, ongoing playback aborts as soon as possible.
    cancel_requested: Arc<AtomicBool>,
}

impl TtsEngine {
    /// Creates a new TTS engine from the given configuration.
    pub fn new(config: &TtsConfig) -> Result<Self, TtsError> {
        // 0. Install bundled espeak-ng data so the phonemizer can find dictionaries.
        Self::ensure_espeak_data();

        // 0b. Create a persistent espeak-ng engine instance with SSML markup enabled.
        let mut espeak_engine =
            EspeakNg::new(&config.phonemizer_config.language).map_err(|e| TtsError::Phonemizer(format!("Failed to initialize espeak-ng engine: {e}")))?;
        espeak_engine.set_markup(true);

        // 1. Load the phoneme ID map from the model config JSON.
        let config_json = std::fs::read_to_string(&config.config_path)
            .map_err(|e| TtsError::ModelConfig(format!("Failed to read config file {}: {e}", config.config_path)))?;
        let config_value: serde_json::Value =
            serde_json::from_str(&config_json).map_err(|e| TtsError::ModelConfig(format!("Failed to parse config JSON: {e}")))?;
        let phoneme_id_map = Self::extract_phoneme_id_map(&config_value)?;

        // 2. Load the ONNX model via ort.
        //    In ort = "=2.0.0-rc.12", the session is created via commit_from_file.
        let mut builder = Session::builder().map_err(|e| TtsError::SessionCreate(e.to_string()))?;

        // Configure execution providers based on hardware features.
        let providers = Self::build_execution_providers();
        if !providers.is_empty() {
            builder = builder
                .with_execution_providers(&providers)
                .map_err(|e| TtsError::SessionCreate(e.to_string()))?;
        }

        let session = builder.commit_from_file(&config.model_path).map_err(|e| TtsError::ModelLoad(e.to_string()))?;

        // 3. Initialize cpal audio output.
        //    Try to find a supported config matching the model's native sample
        //    rate to avoid resampling overhead. Fall back to the default config
        //    if the device does not support the model's sample rate.
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(TtsError::NoOutputDevice)?;

        let cpal_config = {
            let model_rate: cpal::SampleRate = config.model_sample_rate;
            let mut matched = None;
            if let Ok(supported_configs) = device.supported_output_configs() {
                for sc in supported_configs {
                    if sc.min_sample_rate() <= model_rate && model_rate <= sc.max_sample_rate() {
                        matched = Some(sc.with_sample_rate(model_rate));
                        break;
                    }
                }
            }
            match matched {
                Some(config) => config,
                None => {
                    let default = device.default_output_config().map_err(|e| TtsError::AudioConfig(e.to_string()))?;
                    debug!(
                        "Voice Assistant TTS: device does not support model sample rate {} Hz, falling back to {} Hz (resampling will occur)",
                        config.model_sample_rate,
                        default.sample_rate()
                    );
                    default
                }
            }
        };

        debug!(
            "Voice Assistant TTS: initialized with model {:?}, model_sample_rate={}, cpal_sample_rate={}, cpal_channels={}",
            config.model_type,
            config.model_sample_rate,
            cpal_config.sample_rate(),
            cpal_config.channels()
        );

        Ok(Self {
            onnx_session: std::sync::Mutex::new(session),
            cpal_device: device,
            cpal_config,
            model_type: config.model_type.clone(),
            model_sample_rate: config.model_sample_rate,
            language: config.phonemizer_config.language.clone(),
            espeak_engine,
            phoneme_id_map,
            phonemize_enabled: config.phonemize_enabled,
            disable_pad_id: config.disable_pad_id,
            cancel_requested: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Ensures espeak-ng data is available for phonemization.
    ///
    /// Prefers the system-installed espeak-ng-data package (which contains
    /// the full German dictionary with correct inflected forms) over the
    /// bundled subset from the `espeak-ng` crate. The bundled data only
    /// includes a limited dictionary and produces truncated phonemes for
    /// some inflected words (e.g. "verfügbaren" → "fˈɛrfɛr").
    ///
    /// This limitation persists with espeak-ng-rs 0.1.3 — the 0.1.3 release
    /// improved the engine (rules, compiler, text normalization) but did not
    /// expand the bundled dictionary data. System espeak-ng-data remains
    /// required for correct German inflected-form pronunciation.
    ///
    /// Falls back to installing bundled data to a temp directory if the
    /// system data is not found.
    fn ensure_espeak_data() {
        static INSTALLED: std::sync::Once = std::sync::Once::new();

        INSTALLED.call_once(|| {
            // Check for system espeak-ng-data first.
            let system_paths = [
                "/usr/lib/x86_64-linux-gnu/espeak-ng-data",
                "/usr/share/espeak-ng-data",
                "/usr/lib/espeak-ng-data",
            ];
            for path in &system_paths {
                if std::path::Path::new(path).join("de_dict").exists() {
                    debug!("Voice Assistant TTS: using system espeak-ng data at {}", path);
                    // SAFETY: Setting an environment variable inside a `Once`
                    // before any `text_to_ipa` call is safe.
                    unsafe {
                        std::env::set_var("ESPEAK_DATA_PATH", path);
                    }
                    return;
                }
            }

            // Fall back to bundled data.
            debug!("Voice Assistant TTS: system espeak-ng-data not found, installing bundled data");
            let data_dir = std::env::temp_dir().join("espeak-ng-data");

            if let Err(error) = std::fs::create_dir_all(&data_dir) {
                warn!("Voice Assistant TTS: failed to create espeak-ng data dir {:?}: {error}", data_dir);
                return;
            }

            let languages: &[&str] = &["de", "en"];
            if let Err(error) = espeak_ng::install_bundled_languages(&data_dir, languages) {
                warn!("Voice Assistant TTS: failed to install bundled espeak-ng data: {error}");
                return;
            }

            debug!("Voice Assistant TTS: installed bundled espeak-ng data to {:?}", data_dir);
            // SAFETY: See above.
            unsafe {
                std::env::set_var("ESPEAK_DATA_PATH", &data_dir);
            }
        });
    }

    /// Builds the execution provider list based on compile-time features.
    fn build_execution_providers() -> Vec<ExecutionProviderDispatch> {
        #[cfg(all(feature = "ort-rocm", target_os = "linux"))]
        {
            use ort::execution_providers::ROCm;
            debug!("Voice Assistant TTS: using ROCm execution provider");
            vec![ROCm::default().build().into()]
        }
        #[cfg(not(all(feature = "ort-rocm", target_os = "linux")))]
        {
            Vec::new()
        }
    }

    /// Extracts the phoneme ID map from a Piper-style config JSON.
    ///
    /// Piper configs place the map at the root level (`phoneme_id_map`),
    /// while some other formats nest it under `espeak`. Values may be either
    /// a single integer or an array of integers, in which case the first
    /// element is used as the primary phoneme ID.
    fn extract_phoneme_id_map(config: &serde_json::Value) -> Result<std::collections::HashMap<String, i64>, TtsError> {
        let mut map = std::collections::HashMap::new();

        for source in [config.get("phoneme_id_map"), config.get("espeak").and_then(|v| v.get("phoneme_id_map"))] {
            if let Some(value) = source {
                if let Some(obj) = value.as_object() {
                    for (key, id_value) in obj {
                        let id = if let Some(id) = id_value.as_i64() {
                            Some(id)
                        } else if let Some(array) = id_value.as_array() {
                            array.first().and_then(|first| first.as_i64())
                        } else {
                            None
                        };

                        if let Some(id) = id {
                            if (-256..=255).contains(&id) {
                                map.insert(key.clone(), id);
                            } else {
                                debug!("Voice Assistant TTS: ignoring phoneme_id for '{}' with out-of-bounds id {}", key, id);
                            }
                        }
                    }
                }
            }
            if !map.is_empty() {
                break;
            }
        }

        if map.is_empty() {
            debug!("Voice Assistant TTS: phoneme_id_map is empty or missing from config, will use positional IDs");
        }

        Ok(map)
    }

    /// Request cancellation of any ongoing playback.
    /// The audio stream is stopped at the next poll cycle (within ~10 ms).
    pub fn cancel(&self) {
        self.cancel_requested.store(true, Ordering::SeqCst);
        debug!("Voice Assistant TTS: cancellation requested");
    }

    /// Resets the cancellation flag. Called internally before starting new playback.
    fn reset_cancel(&self) {
        self.cancel_requested.store(false, Ordering::SeqCst);
    }

    /// Returns `true` if cancellation has been requested.
    fn is_cancelled(&self) -> bool {
        self.cancel_requested.load(Ordering::SeqCst)
    }

    /// Synthesizes speech from text and plays it through the audio output device.
    ///
    /// This is an async function — the blocking wait for playback completion
    /// uses `tokio::time::sleep` so the Tokio runtime can process other tasks
    /// during audio playback. Call [`cancel`](Self::cancel) to abort playback.
    pub async fn speak(&self, text: &str) -> Result<(), TtsError> {
        debug!("Voice Assistant TTS: speaking \"{}\"", text);
        self.reset_cancel();

        if !self.phonemize_enabled {
            // Direct espeak-ng synthesis path: text -> PCM (no ONNX).
            let normalized_text = self.preprocess_text_for_tts(text);
            debug!("Voice Assistant TTS: normalized text: \"{}\"", normalized_text);
            let (pcm_i16, rate) = self.espeak_engine.synth(&normalized_text).map_err(|e| TtsError::Phonemizer(e.to_string()))?;
            debug!("Voice Assistant TTS: espeak-ng PCM: {} samples at {} Hz", pcm_i16.len(), rate);
            let pcm_f32: Vec<f32> = pcm_i16.iter().map(|&s| s as f32 / 32768.0).collect();
            let cpal_sample_rate = self.cpal_config.sample_rate();
            let resampled = if rate != cpal_sample_rate {
                Self::resample(pcm_f32, rate, cpal_sample_rate)
            } else {
                pcm_f32
            };
            let padded = Self::pad_leading_silence(resampled, cpal_sample_rate as u32);
            self.play_audio(padded).await?;
            return Ok(());
        }

        // 1. Convert text to phoneme token IDs.
        let phoneme_ids = self.phonemize(text)?;

        // 2. Run ONNX inference to generate PCM audio samples at the model's native sample rate.
        let pcm_samples = self.run_inference(&phoneme_ids)?;

        debug!("Voice Assistant TTS: inference produced {} samples at {} Hz", pcm_samples.len(), self.model_sample_rate);

        // 3. Resample from model sample rate to cpal output sample rate if needed.
        let cpal_sample_rate = self.cpal_config.sample_rate();
        let resampled = if self.model_sample_rate != cpal_sample_rate {
            Self::resample(pcm_samples, self.model_sample_rate, cpal_sample_rate)
        } else {
            pcm_samples
        };

        // 4. Pad leading silence to let cpal stream ramp up, then play.
        let padded = Self::pad_leading_silence(resampled, cpal_sample_rate as u32);
        debug!("Voice Assistant TTS: playing {} samples on cpal output device", padded.len());
        self.play_audio(padded).await?;
        debug!("Voice Assistant TTS: playback completed");

        Ok(())
    }

    /// Normalizes written-form text to spoken form before phonemization.
    ///
    /// Uses text-processing-rs (TN) to convert numbers, dates,
    /// measurements, currency, and symbols into their spoken equivalents.
    /// A regex finds normalizable spans in the text; each span is passed
    /// to `tn_normalize_lang` with the appropriate language. Non-matching
    /// text is left untouched.
    ///
    /// German compound words that espeak-ng mispronounces are decomposed
    /// via a Trie-based stem lookup and wrapped in SSML `<sub alias="...">`
    /// tags so the engine speaks the decomposition.
    ///
    /// Sentence boundaries (`.` `!` `?`) are followed by an SSML `<break>`
    /// tag for more natural speech rhythm.
    ///
    /// Times (`10:30`), hyphens between letters (`well-known`), and
    /// dotted version numbers (`1.2.3`) are handled natively by espeak-ng
    /// 0.1.3 and are intentionally not pre-normalized here.
    fn preprocess_text_for_tts(&self, text: &str) -> String {
        let lang = if self.language.starts_with("de") { "de" } else { "en" };

        let re = match Regex::new(r"\d{1,2}\.\d{1,2}\.\d{4}|\d+(?:[.,]\d+)?\s*(?:km/h|m/s|°C|°F|€|%)|\d+(?:[.,]\d+)?") {
            Ok(r) => r,
            Err(_) => return text.to_string(),
        };

        let mut result = String::new();
        let mut last_end = 0;
        let date_re = match Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{4}$") {
            Ok(r) => r,
            Err(_) => return text.to_string(),
        };
        for mat in re.find_iter(text) {
            result.push_str(&text[last_end..mat.start()]);
            let span = mat.as_str();
            // German TN expects ',' as decimal separator, but the LLM outputs
            // English format with '.'. Convert '.' to ',' for non-date numeric
            // spans so "22.9" becomes "22,9" → "zweiundzwanzig komma neun"
            // instead of "229" → "zweihundertneunundzwanzig".
            let tn_input = if lang == "de" && !date_re.is_match(span) {
                span.replace('.', ",")
            } else {
                span.to_string()
            };
            let tn = tn_normalize_lang(&tn_input, lang);
            if tn != tn_input {
                // text-processing-rs transliterates German umlauts to ASCII
                // (ä→ae, ö→oe, ü→ue, ß→ss). espeak-ng mispronounces these.
                // Restore them on the TN output only — the surrounding text
                // already has correct umlauts from the LLM.
                if lang == "de" {
                    result.push_str(&tn.replace("ae", "ä").replace("oe", "ö").replace("ue", "ü").replace("ssz", "ß"));
                } else {
                    result.push_str(&tn);
                }
            } else {
                result.push_str(span);
            }
            last_end = mat.end();
        }
        result.push_str(&text[last_end..]);

        // Sanitize text to work around espeak-ng prefix duplication bugs:
        // - Hyphens between words (Audio-bezogene) cause prefix stuttering
        // - Colons/semicolons directly after words (gefunden:) cause prefix doubling
        result = Self::sanitize_espeak_text(&result);

        // Decompose German compound words via Trie and wrap in SSML <sub alias>.
        if lang == "de" {
            result = Self::wrap_german_compounds(&result);
        }

        // Insert SSML <break> after sentence boundaries for natural rhythm.
        result = Self::insert_sentence_breaks(&result);

        result
    }

    /// Sanitizes text to work around espeak-ng prefix duplication bugs.
    ///
    /// espeak-ng's G2P state machine has two known triggers that cause
    /// prefix syllable doubling (ge→gege, be→bebe, an→anan):
    ///
    /// 1. **Hyphens between words** ("Audio-bezogene") confuse the lookahead
    ///    parser, causing the following word's prefix to be evaluated twice.
    ///    Fix: replace hyphens between word characters with spaces.
    ///
    /// 2. **Colons/semicolons directly after a word** ("gefunden:") cause
    ///    the prefix to be doubled during phonemization.
    ///    Fix: replace with a period followed by a space.
    fn sanitize_espeak_text(text: &str) -> String {
        static HYPHEN_RE: OnceLock<Option<Regex>> = OnceLock::new();
        static COLON_RE: OnceLock<Option<Regex>> = OnceLock::new();
        let hyphen_re = HYPHEN_RE.get_or_init(|| Regex::new(r"(\p{L})-(\p{L})").ok());
        let colon_re = COLON_RE.get_or_init(|| Regex::new(r"(\p{L})[:;](\s|$)").ok());

        let result = match hyphen_re {
            Some(re) => re.replace_all(text, "$1 $2").to_string(),
            None => text.to_string(),
        };
        match colon_re {
            Some(re) => re.replace_all(&result, "$1.$2").to_string(),
            None => result,
        }
    }

    /// Find German compound words in the text and wrap them in SSML
    /// `<sub alias="...">` tags using Trie-based decomposition.
    /// Only words of 6+ characters that are not already inside SSML tags
    /// are considered.
    fn wrap_german_compounds(text: &str) -> String {
        let word_re = match Regex::new(r"\b[A-ZÄÖÜ][a-zäöüß]{5,}\b") {
            Ok(r) => r,
            Err(_) => return text.to_string(),
        };

        let mut result = String::with_capacity(text.len());
        let mut last_end = 0;

        for mat in word_re.find_iter(text) {
            // Check if we're inside an SSML tag — skip if so.
            let before = &text[last_end..mat.start()];
            let open_tags = before.matches('<').count();
            let close_tags = before.matches('>').count();
            if open_tags > close_tags {
                // Inside an SSML tag attribute or content — skip.
                result.push_str(&text[last_end..mat.end()]);
                last_end = mat.end();
                continue;
            }

            result.push_str(&text[last_end..mat.start()]);
            let word = mat.as_str();

            if let Some(parts) = german_compound_trie().decompose(word) {
                let alias = parts
                    .iter()
                    .map(|p| {
                        let mut chars = p.chars();
                        match chars.next() {
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                result.push_str(&format!("<sub alias=\"{}\">{}</sub>", alias, word));
            } else {
                result.push_str(word);
            }
            last_end = mat.end();
        }
        result.push_str(&text[last_end..]);
        result
    }

    /// Insert SSML `<break time="300ms"/>` after sentence-ending punctuation
    /// (`. `, `! `, `? `) for more natural speech rhythm.
    fn insert_sentence_breaks(text: &str) -> String {
        let sentence_re = match Regex::new(r"([.!?])\s+") {
            Ok(r) => r,
            Err(_) => return text.to_string(),
        };
        sentence_re.replace_all(text, "$1<break time=\"300ms\"/> ").to_string()
    }

    /// Converts text to phoneme token IDs using espeak-ng.
    fn phonemize(&self, text: &str) -> Result<Vec<i64>, TtsError> {
        let normalized_text = self.preprocess_text_for_tts(text);
        debug!("Voice Assistant TTS: normalized text: \"{}\"", normalized_text);

        if !self.phonemize_enabled {
            debug!("Voice Assistant TTS: phonemization disabled, using raw character codes");
            let ids: Vec<i64> = if self.phoneme_id_map.is_empty() {
                normalized_text.chars().map(|c| c as i64).collect()
            } else {
                let pad_id = self.phoneme_id_map.get("_").copied().unwrap_or(0);
                let bos_id = self.phoneme_id_map.get("^").copied().unwrap_or(1);
                let eos_id = self.phoneme_id_map.get("$").copied().unwrap_or(2);
                let mut ids = Vec::new();
                ids.push(bos_id);
                ids.push(pad_id);
                for ch in normalized_text.chars() {
                    let key = ch.to_string();
                    if let Some(id) = self.phoneme_id_map.get(&key).copied() {
                        ids.push(id);
                        if !self.disable_pad_id {
                            ids.push(pad_id);
                        }
                    }
                }
                ids.push(eos_id);
                ids
            };
            if ids.is_empty() {
                return Err(TtsError::Phonemizer("No phoneme IDs generated from text".to_string()));
            }
            return Ok(ids);
        }

        let ipa = self
            .espeak_engine
            .text_to_phonemes_with_options(
                &normalized_text,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .map_err(|e| TtsError::Phonemizer(e.to_string()))?;
        // Decompose precomposed characters (e.g. "ç" -> "c" + combining cedilla)
        // to match the normalization used by piper-phonemize.
        let ipa: String = ipa.nfd().collect();

        debug!("Voice Assistant TTS: IPA phonemes: {}", ipa);

        // Convert IPA phoneme symbols to integer IDs using the phoneme ID map.
        // If the map is empty (e.g., Kokoro), use positional encoding as fallback.
        let ids: Vec<i64> = if self.phoneme_id_map.is_empty() {
            // Fallback: use character codes as IDs (Kokoro has its own phoneme map).
            ipa.chars().map(|c| c as i64).collect()
        } else {
            let pad_id = self.phoneme_id_map.get("_").copied().unwrap_or(0);
            let bos_id = self.phoneme_id_map.get("^").copied().unwrap_or(1);
            let eos_id = self.phoneme_id_map.get("$").copied().unwrap_or(2);

            let mut ids = Vec::new();
            ids.push(bos_id);
            ids.push(pad_id);

            for phoneme in ipa.chars() {
                // Newlines from espeak-ng mark sentence/clause boundaries.
                // Insert a pad_id to create a pause between sentences so the
                // Piper model produces natural inter-sentence pauses.
                if phoneme == '\n' {
                    ids.push(pad_id);
                    if !self.disable_pad_id {
                        ids.push(pad_id);
                    }
                    continue;
                }

                let phoneme_key = phoneme.to_string();
                if let Some(id) = self.phoneme_id_map.get(&phoneme_key).copied() {
                    ids.push(id);
                    if !self.disable_pad_id {
                        ids.push(pad_id);
                    }
                } else {
                    warn!("Voice Assistant TTS: ignoring unknown phoneme '{}' (U+{:04X})", phoneme, phoneme as u32);
                    ids.push(pad_id);
                }
            }

            ids.push(eos_id);
            ids
        };

        if ids.is_empty() {
            return Err(TtsError::Phonemizer("No phoneme IDs generated from text".to_string()));
        }

        Ok(ids)
    }

    /// Runs ONNX inference to generate raw f32 PCM samples from phoneme IDs.
    fn run_inference(&self, phoneme_ids: &[i64]) -> Result<Vec<f32>, TtsError> {
        use ndarray::Array1;
        use ndarray::Array2;
        use ort::value::Tensor;

        let seq_len = phoneme_ids.len();

        // Piper expects 3 inputs:
        //   input:         int64 [batch_size, phonemes]
        //   input_lengths: int64 [batch_size]
        //   scales:        float32 [3] (noise_scale, length_scale, noise_w)
        let input_array = Array2::from_shape_vec((1, seq_len), phoneme_ids.to_vec()).map_err(|e| TtsError::TensorCreate(format!("Input shape error: {e}")))?;
        let input_tensor = Tensor::from_array(input_array).map_err(|e| TtsError::TensorCreate(e.to_string()))?;

        let input_lengths_array = Array1::from_vec(vec![seq_len as i64]);
        let input_lengths_tensor = Tensor::from_array(input_lengths_array).map_err(|e| TtsError::TensorCreate(e.to_string()))?;

        let scales_array = Array1::from_vec(vec![0.667_f32, 1.0, 0.8]);
        let scales_tensor = Tensor::from_array(scales_array).map_err(|e| TtsError::TensorCreate(e.to_string()))?;

        let inputs = ort::inputs![input_tensor, input_lengths_tensor, scales_tensor];

        let mut session = self.onnx_session.lock().map_err(|e| TtsError::Inference(format!("Session lock failed: {e}")))?;

        let outputs = session.run(inputs).map_err(|e| TtsError::Inference(e.to_string()))?;

        // Piper output tensor is named "output" with shape [batch, time, 1, dim].
        let output_tensor = outputs
            .get("output")
            .or_else(|| outputs.get("audio"))
            .or_else(|| outputs.get("wav"))
            .ok_or(TtsError::MissingOutput)?;

        let (_shape, data) = output_tensor.try_extract_tensor::<f32>().map_err(|e| TtsError::TensorExtract(e.to_string()))?;

        // The output is [batch=1, time, 1, dim] — flatten to mono PCM samples.
        Ok(data.to_vec())
    }

    /// Prepends silence to the PCM buffer to prevent truncation of the first words.
    ///
    /// cpal needs time to start the audio stream after `build_output_stream` + `play()`.
    /// Without leading silence, the first samples are consumed before audio output
    /// actually begins, causing the first few words to be inaudible.
    fn pad_leading_silence(samples: Vec<f32>, sample_rate: u32) -> Vec<f32> {
        let silence_samples = (sample_rate as f64 * 0.2) as usize;
        let mut padded = Vec::with_capacity(silence_samples + samples.len());
        padded.extend(std::iter::repeat_n(0.0f32, silence_samples));
        padded.extend(samples);
        padded
    }

    /// Resamples PCM samples from one sample rate to another using linear interpolation.
    fn resample(samples: Vec<f32>, from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate || samples.is_empty() {
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
    async fn play_audio(&self, pcm_samples: Vec<f32>) -> Result<(), TtsError> {
        let state = Arc::new(Mutex::new(PlaybackState {
            samples: pcm_samples,
            position: 0,
        }));
        let state_clone = Arc::clone(&state);
        let cancel_clone = Arc::clone(&self.cancel_requested);

        let err_fn = |err| {
            warn!("Voice Assistant TTS: audio stream error: {err}");
        };

        let channels = self.cpal_config.channels();

        let stream = self
            .cpal_device
            .build_output_stream(
                self.cpal_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Check cancellation flag — if set, output silence and advance position to end.
                    if cancel_clone.load(Ordering::SeqCst) {
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                        if let Ok(mut guard) = state_clone.try_lock() {
                            guard.position = guard.samples.len();
                        }
                        return;
                    }
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
        stream.play().map_err(|e| TtsError::StreamPlay(e.to_string()))?;
        debug!("Voice Assistant TTS: cpal output stream started, waiting for playback to complete");

        // Async wait until all samples have been played or cancellation is requested.
        // Uses tokio::time::sleep so the Tokio runtime can process other tasks.
        let max_wait = std::time::Duration::from_secs(120);
        let start = std::time::Instant::now();
        loop {
            if self.is_cancelled() {
                debug!("Voice Assistant TTS: playback cancelled by request");
                break;
            }
            let done = state.lock().map(|guard| guard.position >= guard.samples.len()).unwrap_or(true);
            if done {
                break;
            }
            if start.elapsed() > max_wait {
                warn!("Voice Assistant TTS: playback timed out after {max_wait:?}, aborting");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Wait for cpal to finish playing the remaining samples in the hardware buffer.
        // Without this delay, dropping the stream immediately truncates the last word.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Drop the stream to stop audio output.
        drop(stream);

        Ok(())
    }
}

/// Loads a TTS engine from configuration if TTS is enabled.
/// Returns `None` if TTS is disabled or if initialization fails (with a warning logged).
pub fn try_init_tts(config: &TtsConfig) -> Option<TtsEngine> {
    if !config.enabled {
        debug!("Voice Assistant TTS: disabled in config, skipping initialization");
        return None;
    }

    if !Path::new(&config.model_path).exists() {
        warn!("Voice Assistant TTS: model file not found at {}, skipping TTS initialization", config.model_path);
        return None;
    }

    match TtsEngine::new(config) {
        Ok(engine) => {
            debug!("Voice Assistant TTS: engine initialized successfully");
            Some(engine)
        }
        Err(error) => {
            warn!("Voice Assistant TTS: failed to initialize engine: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that espeak-ng produces non-empty IPA output for a German
    /// weather text containing numbers, times, temperatures, and compound words.
    /// This tests the system espeak-ng-data path (if available) or the
    /// bundled data fallback.
    #[test]
    fn test_espeak_ipa_german_weather_text() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let text = "Die Temperatur beträgt 21 Grad und die Luftfeuchtigkeit ist 65 Prozent.";
        let ipa = engine.text_to_phonemes(text);
        assert!(ipa.is_ok(), "text_to_phonemes should succeed for German text");
        let ipa = ipa.unwrap();
        assert!(!ipa.is_empty(), "IPA output should not be empty");
    }

    /// Verifies that espeak-ng produces non-empty PCM audio for German text.
    #[test]
    fn test_espeak_pcm_german_text() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let text = "Hallo, wie ist das Wetter heute?";
        let pcm = engine.synth(text);
        assert!(pcm.is_ok(), "synth should succeed for German text");
        let (samples, rate) = pcm.unwrap();
        assert!(!samples.is_empty(), "PCM output should not be empty");
        assert!(rate > 0, "PCM sample rate should be positive");
    }

    /// Verifies that espeak-ng handles English text correctly.
    #[test]
    fn test_espeak_ipa_english_text() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("en").expect("should create espeak-ng engine");
        let text = "The temperature is 72 degrees Fahrenheit.";
        let ipa = engine.text_to_phonemes(text);
        assert!(ipa.is_ok(), "text_to_phonemes should succeed for English text");
        let ipa = ipa.unwrap();
        assert!(!ipa.is_empty(), "IPA output should not be empty");
    }

    /// Verifies that espeak-ng 0.1.3 reads German compound numbers correctly
    /// (e.g. "21" should produce IPA containing the compound form, not
    /// "eins und zwanzig").
    #[test]
    fn test_espeak_german_compound_number() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ipa = engine.text_to_phonemes("21").unwrap();
        assert!(!ipa.is_empty(), "IPA for '21' should not be empty");
    }

    /// Verifies that espeak-ng 0.1.3 reads times correctly (e.g. "10:30").
    #[test]
    fn test_espeak_german_time() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ipa = engine.text_to_phonemes("10:30").unwrap();
        assert!(!ipa.is_empty(), "IPA for '10:30' should not be empty");
    }

    /// Verifies that espeak-ng 0.1.3 reads negative numbers correctly.
    #[test]
    fn test_espeak_german_negative_number() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ipa = engine.text_to_phonemes("-5 Grad").unwrap();
        assert!(!ipa.is_empty(), "IPA for '-5 Grad' should not be empty");
    }

    /// Verifies that espeak-ng 0.1.3 handles degree symbols in German.
    #[test]
    fn test_espeak_german_degree_symbol() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ipa = engine.text_to_phonemes("20°C").unwrap();
        assert!(!ipa.is_empty(), "IPA for '20°C' should not be empty");
    }

    /// Verifies that espeak-ng 0.1.3 handles hyphenated words natively
    /// (e.g. "Wallpaper-Themen" should not be merged into one phoneme sequence).
    #[test]
    fn test_espeak_german_hyphenated_word() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ipa = engine.text_to_phonemes("Wallpaper-Themen").unwrap();
        assert!(!ipa.is_empty(), "IPA for 'Wallpaper-Themen' should not be empty");
    }

    /// Verifies that SSML `<sub alias="...">` replaces compound word pronunciation.
    /// "Luftqualität" wrapped in `<sub alias="Luft Qualität">` should produce
    /// IPA containing both "Luft" and "Qualität" as separate words.
    #[test]
    fn test_ssml_sub_alias_compound_word() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ssml = r#"<sub alias="Luft Qualität">Luftqualität</sub>"#;
        let ipa = engine
            .text_to_phonemes_with_options(
                ssml,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .unwrap();
        assert!(!ipa.is_empty(), "IPA for SSML <sub> compound should not be empty");
    }

    /// Verifies that SSML markup mode processes German weather text with
    /// embedded `<sub>` tags correctly — the IPA should contain the alias
    /// pronunciation, not the raw compound word.
    #[test]
    fn test_ssml_german_weather_with_compound_subs() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ssml = r#"Die <sub alias="Luft Qualität">Luftqualität</sub> ist gut."#;
        let ipa = engine
            .text_to_phonemes_with_options(
                ssml,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .unwrap();
        assert!(!ipa.is_empty(), "IPA for SSML weather text should not be empty");
    }

    /// Verifies that SSML `<break>` inserts a pause in the IPA output.
    #[test]
    fn test_ssml_break_tag() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ssml = "Hallo<break/>Welt";
        let ipa = engine
            .text_to_phonemes_with_options(
                ssml,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .unwrap();
        assert!(!ipa.is_empty(), "IPA for SSML <break> should not be empty");
    }

    /// Verifies that SSML `<say-as interpret-as="characters">` spells out
    /// each character instead of reading the word.
    #[test]
    fn test_ssml_say_as_characters() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("en").expect("should create espeak-ng engine");
        let ssml = r#"<say-as interpret-as="characters">ABC</say-as>"#;
        let ipa = engine
            .text_to_phonemes_with_options(
                ssml,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .unwrap();
        assert!(!ipa.is_empty(), "IPA for SSML <say-as characters> should not be empty");
    }

    /// Verifies that the Trie-based compound decomposition correctly
    /// wraps "Luftqualität" in an SSML <sub alias> tag.
    #[test]
    fn test_trie_decompose_luftqualitaet_ssml() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ssml = "<sub alias=\"Luft Qualität\">Luftqualität</sub>";
        let ipa = engine
            .text_to_phonemes_with_options(
                ssml,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .unwrap();
        assert!(!ipa.is_empty(), "IPA for Trie-decomposed compound should not be empty");
    }

    /// Verifies that sentence breaks insert <break> tags correctly.
    #[test]
    fn test_insert_sentence_breaks() {
        let input = "Hallo. Wie geht es dir?";
        let result = TtsEngine::insert_sentence_breaks(input);
        assert!(result.contains("<break time=\"300ms\"/>"), "Should contain break tag after sentence");
    }

    /// Verifies that wrap_german_compounds produces SSML for known compounds.
    #[test]
    fn test_wrap_german_compounds_luftfeuchtigkeit() {
        let input = "Die Luftfeuchtigkeit ist 65 Prozent.";
        let result = TtsEngine::wrap_german_compounds(input);
        assert!(
            result.contains("<sub alias=\"Luft Feuchtigkeit\">Luftfeuchtigkeit</sub>"),
            "Should wrap Luftfeuchtigkeit in SSML <sub alias>"
        );
    }

    /// Verifies that wrap_german_compounds does not wrap non-compound words.
    #[test]
    fn test_wrap_german_compounds_non_compound() {
        let input = "Das Wetter ist schön.";
        let result = TtsEngine::wrap_german_compounds(input);
        assert!(!result.contains("<sub"), "Should not wrap non-compound words");
    }

    /// Verifies that SSML <break> with time attribute works in synthesis.
    #[test]
    fn test_ssml_break_with_time() {
        TtsEngine::ensure_espeak_data();
        let engine = EspeakNg::new("de").expect("should create espeak-ng engine");
        let ssml = "Hallo.<break time=\"300ms\"/> Wie geht es dir?";
        let ipa = engine
            .text_to_phonemes_with_options(
                ssml,
                espeak_ng::TextToPhonemesOptions {
                    preserve_punctuation: true,
                    flatten_clauses: true,
                    markup: true,
                },
            )
            .unwrap();
        assert!(!ipa.is_empty(), "IPA for SSML <break time> should not be empty");
    }
}
