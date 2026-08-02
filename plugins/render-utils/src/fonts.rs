use ab_glyph::FontVec;
use std::sync::OnceLock;
use tracing::debug;
use tracing::error;
use tracing::trace;
use woff2_patched::decode::convert_woff2_to_ttf;
use woff2_patched::decode::is_woff2;

/// Path to the Symbols-only Nerd Font (for icons).
const NERD_FONT_PATH: &str = "resources/NerdFontsSymbolsOnly/SymbolsNerdFont-Regular.ttf";

/// Path to the JetBrains Mono Nerd Font (WOFF2, for labels with full character set).
const LABEL_FONT_PATH: &str = "resources/JetBrainsMonoNLNerdFont/JetBrainsMonoNLNerdFont-Regular.woff2";

/// Cached Symbols-only Nerd Font loaded from disk.
static NERD_FONT: OnceLock<Option<FontVec>> = OnceLock::new();

/// Cached label font (JetBrains Mono Nerd Font) loaded from disk.
static LABEL_FONT: OnceLock<Option<FontVec>> = OnceLock::new();

/// Get the cached Symbols-only Nerd Font, loading from disk on first access.
pub fn nerd_font() -> Option<&'static FontVec> {
    NERD_FONT
        .get_or_init(|| match std::fs::read(NERD_FONT_PATH) {
            Ok(data) => match FontVec::try_from_vec(data) {
                Ok(font) => {
                    trace!("render-utils: loaded Nerd Font from {}", NERD_FONT_PATH);
                    Some(font)
                }
                Err(e) => {
                    trace!("render-utils: failed to parse Nerd Font: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("render-utils: failed to read Nerd Font file {}: {}", NERD_FONT_PATH, e);
                None
            }
        })
        .as_ref()
}

/// Get the cached label font (JetBrains Mono Nerd Font), loading from WOFF2 on first access.
pub fn label_font() -> Option<&'static FontVec> {
    LABEL_FONT
        .get_or_init(|| match std::fs::read(LABEL_FONT_PATH) {
            Ok(data) => {
                if !is_woff2(&data) {
                    debug!("render-utils: label font file is not WOFF2, trying as TTF");
                    return FontVec::try_from_vec(data).ok();
                }
                match convert_woff2_to_ttf(&mut std::io::Cursor::new(data)) {
                    Ok(ttf_data) => match FontVec::try_from_vec(ttf_data) {
                        Ok(font) => {
                            trace!("render-utils: loaded label font from {}", LABEL_FONT_PATH);
                            Some(font)
                        }
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
                error!("render-utils: failed to read label font file {}: {}", LABEL_FONT_PATH, e);
                None
            }
        })
        .as_ref()
}
