use hf_hub::api::sync::ApiBuilder;
use serde::Deserialize;
use std::path::Path;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// A single fallback model mapping entry loaded from `data/fallback_models.toml`.
///
/// Matching is case-insensitive: the local filename is checked whether it
/// *contains* the `pattern` string.  The first matching entry wins.
#[derive(Debug, Clone, Deserialize)]
pub struct FallbackModelEntry {
    /// Substring pattern to match against the local filename (case-insensitive).
    pub pattern: String,
    /// HuggingFace repository ID (e.g. `ggerganov/whisper.cpp`).
    pub repo: String,
    /// Optional subdirectory within the repo where the file lives.
    /// When set, the file is fetched from `<repo>/resolve/main/<remote_path>/<remote_filename>`.
    /// When absent, the file is fetched from the repository root.
    pub remote_path: Option<String>,
    /// Optional remote filename when it differs from the local filename.
    /// When set, the file is fetched as `<remote_filename>` (or
    /// `<remote_path>/<remote_filename>` when `remote_path` is also set).
    /// When absent, the local filename is used on the remote side as well.
    pub remote_filename: Option<String>,
}

/// Container struct for deserializing the TOML file.
#[derive(Debug, Clone, Deserialize)]
struct FallbackModelTable {
    models: Vec<FallbackModelEntry>,
}

/// Load the fallback model mappings from the embedded TOML file.
///
/// The file `data/fallback_models.toml` is compiled into the binary via
/// `include_str!` so no runtime file access is needed.
fn load_fallback_table() -> Vec<FallbackModelEntry> {
    const TOML_CONTENT: &str = include_str!("../data/fallback_models.toml");
    match toml::from_str::<FallbackModelTable>(TOML_CONTENT) {
        Ok(table) => table.models,
        Err(error) => {
            warn!("Model downloader: failed to parse fallback_models.toml: {error}");
            Vec::new()
        }
    }
}

/// Resolve the fallback mapping for a given filename.
///
/// Returns the first matching entry from the TOML table (case-insensitive
/// substring match on `pattern`).
fn fallback_entry(filename: &str) -> Option<FallbackModelEntry> {
    let lower = filename.to_lowercase();
    load_fallback_table().into_iter().find(|entry| lower.contains(&entry.pattern.to_lowercase()))
}

/// Resolves the full fallback entry (repo + optional remote_path) for a model file.
///
/// If `explicit_repo` is set in the config, it takes precedence and
/// `remote_path` is set to `None` (explicit repos always fetch from root).
fn resolve_fallback(filename: &str, explicit_repo: &str) -> Option<FallbackModelEntry> {
    if !explicit_repo.is_empty() {
        return Some(FallbackModelEntry {
            pattern: String::new(),
            repo: explicit_repo.to_string(),
            remote_path: None,
            remote_filename: None,
        });
    }
    fallback_entry(filename)
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

    let entry = match resolve_fallback(filename, explicit_repo) {
        Some(e) => e,
        None => {
            warn!("Model downloader: no repo configured for {filename}, skipping download");
            return;
        }
    };

    info!("Model downloader: {filename} not found, downloading from HuggingFace repo {}", entry.repo);

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

    let repo_api = api.model(entry.repo.clone());

    // Determine the remote filename: use `remote_filename` if set, otherwise the local filename.
    let remote_name = entry.remote_filename.as_deref().unwrap_or(filename);

    // Fetch from the repo root or from a subdirectory when `remote_path` is set.
    let remote_file = match &entry.remote_path {
        Some(subdir) => format!("{subdir}/{remote_name}"),
        None => remote_name.to_string(),
    };

    match repo_api.get(&remote_file) {
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
            warn!("Model downloader: failed to download {remote_file} from {}: {error}", entry.repo);
        }
    }
}
