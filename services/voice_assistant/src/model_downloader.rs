use hf_hub::api::sync::ApiBuilder;
use std::path::Path;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Hardcoded fallback mapping: filename pattern -> HuggingFace repo ID.
///
/// Used when no explicit `*_repo` is set in the config. The mapping is
/// matched by checking if the filename **contains** the pattern key.
fn fallback_repo(filename: &str) -> Option<&'static str> {
    let lower = filename.to_lowercase();
    let mappings: &[(&str, &str)] = &[
        // LLM — Gemma 4 GGUF quantizations
        ("gemma-4-12b", "unsloth/gemma-4-12b-it-GGUF"),
        ("gemma-4-e4b", "unsloth/gemma-4-E4B-it-GGUF"),
        ("gemma-4-e2b", "unsloth/gemma-4-E2B-it-GGUF"),
        // LLM — Qwen 2.5
        ("qwen2.5-1.5b", "Qwen/Qwen2.5-1.5B-Instruct-GGUF"),
        ("qwen2.5-3b", "Qwen/Qwen2.5-3B-Instruct-GGUF"),
        // Whisper GGML
        ("ggml-tiny", "openai/whisper-tiny"),
        ("ggml-base", "openai/whisper-base"),
        ("ggml-small", "openai/whisper-small"),
        ("ggml-large-v3-turbo", "openai/whisper-large-v3-turbo"),
        // Silero VAD
        ("silero_vad", "snakers4/silero-vad"),
        // Piper TTS — German thorsten
        ("de_de-thorsten", "rhasspy/piper-voices"),
    ];

    for (pattern, repo) in mappings {
        if lower.contains(pattern) {
            return Some(repo);
        }
    }
    None
}

/// Resolves the HuggingFace repo ID for a model file.
///
/// If `explicit_repo` is set in the config, it takes precedence.
/// Otherwise, the hardcoded fallback mapping is used.
pub fn resolve_repo(filename: &str, explicit_repo: &str) -> Option<String> {
    if !explicit_repo.is_empty() {
        return Some(explicit_repo.to_string());
    }
    fallback_repo(filename).map(|r| r.to_string())
}

/// Ensures a model file exists at `local_path`. If it doesn't exist,
/// downloads it from the HuggingFace Hub using the given repo ID.
///
/// The filename is extracted from `local_path` and used as the remote filename.
/// If no repo can be resolved (neither explicit nor fallback), this is a no-op.
pub fn ensure_model(local_path: &str, explicit_repo: &str) {
    let path = Path::new(local_path);

    if path.exists() {
        debug!("Model downloader: file already exists at {local_path}");
        return;
    }

    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => {
            warn!("Model downloader: cannot extract filename from path {local_path}");
            return;
        }
    };

    let repo = match resolve_repo(filename, explicit_repo) {
        Some(r) => r,
        None => {
            warn!("Model downloader: no repo configured for {filename}, skipping download");
            return;
        }
    };

    info!("Model downloader: {filename} not found, downloading from HuggingFace repo {repo}");

    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            warn!("Model downloader: failed to create directory {:?}: {error}", parent);
            return;
        }
    }

    let api = match ApiBuilder::new().with_progress(true).build() {
        Ok(api) => api,
        Err(error) => {
            warn!("Model downloader: failed to create HuggingFace API client: {error}");
            return;
        }
    };

    let repo_api = api.model(repo.clone());
    match repo_api.get(filename) {
        Ok(downloaded_path) => {
            if let Err(error) = std::fs::rename(&downloaded_path, local_path) {
                // rename may fail across filesystems — try copy + remove
                if let Err(copy_error) = std::fs::copy(&downloaded_path, local_path) {
                    warn!("Model downloader: failed to move downloaded file to {local_path}: rename={error}, copy={copy_error}");
                    return;
                }
                let _ = std::fs::remove_file(&downloaded_path);
            }
            info!("Model downloader: successfully downloaded {filename} to {local_path}");
        }
        Err(error) => {
            warn!("Model downloader: failed to download {filename} from {repo}: {error}");
        }
    }
}
