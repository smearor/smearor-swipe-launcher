use ab_glyph::FontVec;
use std::sync::OnceLock;
use tracing::debug;
use tracing::error;
use tracing::trace;
use woff2_patched::decode::convert_woff2_to_ttf;
use woff2_patched::decode::is_woff2;

/// Relative path to the Symbols-only Nerd Font (for icons).
const NERD_FONT_RELATIVE: &str = "resources/NerdFontsSymbolsOnly/SymbolsNerdFont-Regular.ttf";

/// Relative path to the JetBrains Mono Nerd Font (WOFF2, for labels with full character set).
const LABEL_FONT_RELATIVE: &str = "resources/JetBrainsMonoNLNerdFont/JetBrainsMonoNLNerdFont-Regular.woff2";

/// System-wide base directory (Debian package install location).
const SYSTEM_BASE_DIR: &str = "/usr/share/smearor";

/// Search candidate paths for a font file: CWD-relative, system-wide, and executable-relative.
fn candidate_font_paths(relative: &str) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    paths.push(std::path::PathBuf::from(relative));
    paths.push(std::path::PathBuf::from(SYSTEM_BASE_DIR).join(relative));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(relative));
        }
    }
    paths
}

/// Read the first matching font file from candidate paths.
fn read_font_file(relative: &str) -> Result<Vec<u8>, std::io::Error> {
    let candidates = candidate_font_paths(relative);
    for path in &candidates {
        match std::fs::read(path) {
            Ok(data) => {
                trace!("render-utils: loaded font from {}", path.display());
                return Ok(data);
            }
            Err(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "font file not found in any of: {}",
            candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        ),
    ))
}

/// Cached Symbols-only Nerd Font loaded from disk.
static NERD_FONT: OnceLock<Option<FontVec>> = OnceLock::new();

/// Cached label font (JetBrains Mono Nerd Font) loaded from disk.
static LABEL_FONT: OnceLock<Option<FontVec>> = OnceLock::new();

/// Get the cached Symbols-only Nerd Font, loading from disk on first access.
pub fn nerd_font() -> Option<&'static FontVec> {
    NERD_FONT
        .get_or_init(|| match read_font_file(NERD_FONT_RELATIVE) {
            Ok(data) => match FontVec::try_from_vec(data) {
                Ok(font) => Some(font),
                Err(e) => {
                    trace!("render-utils: failed to parse Nerd Font: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("render-utils: failed to read Nerd Font: {}", e);
                None
            }
        })
        .as_ref()
}

/// Get the cached label font (JetBrains Mono Nerd Font), loading from WOFF2 on first access.
pub fn label_font() -> Option<&'static FontVec> {
    LABEL_FONT
        .get_or_init(|| match read_font_file(LABEL_FONT_RELATIVE) {
            Ok(data) => {
                if !is_woff2(&data) {
                    debug!("render-utils: label font file is not WOFF2, trying as TTF");
                    return FontVec::try_from_vec(data).ok();
                }
                match convert_woff2_to_ttf(&mut std::io::Cursor::new(data)) {
                    Ok(ttf_data) => match FontVec::try_from_vec(ttf_data) {
                        Ok(font) => Some(font),
                        Err(e) => {
                            trace!("render-utils: failed to parse decompressed label font: {}", e);
                            None
                        }
                    },
                    Err(e) => {
                        error!("render-utils: failed to decompress WOFF2 label font: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("render-utils: failed to read label font: {}", e);
                None
            }
        })
        .as_ref()
}
