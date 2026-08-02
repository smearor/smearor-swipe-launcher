use std::path::PathBuf;
use tracing::debug;

/// Prefix for all smearor shared libraries.
const LIBRARY_PREFIX: &str = "libsmearor_";

/// File extension for shared libraries on this platform.
#[cfg(target_os = "linux")]
const LIBRARY_EXTENSION: &str = "so";
#[cfg(target_os = "macos")]
const LIBRARY_EXTENSION: &str = "dylib";
#[cfg(target_os = "windows")]
const LIBRARY_EXTENSION: &str = "dll";

/// System-wide plugin directory.
const SYSTEM_LIB_DIR: &str = "/usr/local/lib/smearor";

/// Resolves a plugin library path from either an explicit `path` or a `name`.
///
/// When `path` is provided, it is returned as-is (after tilde expansion).
/// When `name` is provided, the host searches for
/// `libsmearor_<name>.<ext>` in the following directories (first match wins):
/// - `~/.local/lib/smearor/` (user-local)
/// - `/usr/local/lib/smearor/` (system-wide)
///
/// Returns an error when neither `path` nor `name` is provided, or when
/// a `name`-based lookup finds no matching file in any search directory.
pub fn resolve_library_path(path: &Option<String>, name: &Option<String>) -> Result<PathBuf, LibraryPathError> {
    if let Some(path) = path {
        return Ok(expand_tilde(path));
    }

    let name = name.as_ref().ok_or(LibraryPathError::NeitherPathNorName)?;

    let filename = format!("{LIBRARY_PREFIX}{name}.{LIBRARY_EXTENSION}");

    let user_dir = dirs::home_dir().map(|home| home.join(".local/lib/smearor"));
    let search_dirs: Vec<PathBuf> = [user_dir, Some(PathBuf::from(SYSTEM_LIB_DIR))].into_iter().flatten().collect();

    for dir in &search_dirs {
        let candidate = dir.join(&filename);
        if candidate.is_file() {
            debug!("Resolved plugin library '{}' to {}", name, candidate.display());
            return Ok(candidate);
        }
    }

    Err(LibraryPathError::NotFound {
        name: name.clone(),
        filename,
        searched_dirs: search_dirs.iter().map(|d| d.display().to_string()).collect(),
    })
}

/// Expands a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Errors that can occur during library path resolution.
#[derive(Debug, thiserror::Error)]
pub enum LibraryPathError {
    #[error("Plugin entry has neither 'path' nor 'name' specified")]
    NeitherPathNorName,

    #[error("Plugin library '{name}' not found: searched for '{filename}' in {searched_dirs:?}")]
    NotFound {
        name: String,
        filename: String,
        searched_dirs: Vec<String>,
    },
}
