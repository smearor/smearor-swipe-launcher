// Nerd Font icon name resolution.
//
// Maps human-readable icon names like `nf-fa-gamepad` to their Unicode
// codepoints using the glyph list compiled into the binary.

use tracing::trace;

/// Resolves a CSS class name (e.g. `"nf-fa-gamepad"` or `"fa-gamepad"`)
/// into an icon name string that `gtk4::Image::from_icon_name` understands.
///
/// The `nerd_gtk_icons` crate registers SVG icons as GResource under
/// the path `/io/nerd_fonts/icons/`. Each icon is named following the
/// pattern `nf-{prefix}-{name}-symbolic` (kebab-case, lower-case).
pub fn resolve_gtk_nerd_icon(css_class: &str) -> Option<String> {
    let clean_name = css_class.strip_prefix("nf-").unwrap_or(css_class);
    let normalized = clean_name.replace('-', "_").to_uppercase();

    let mut icon_name = if normalized.starts_with("NF_") {
        normalized
    } else {
        format!("NF_{}", normalized)
    };

    if !icon_name.ends_with("_SYMBOLIC") {
        icon_name.push_str("_SYMBOLIC");
    }

    let gtk_friendly_name = icon_name.to_lowercase().replace('_', "-");

    trace!("resolve_gtk_nerd_icon: input='{}' -> output='{}'", css_class, gtk_friendly_name);

    Some(gtk_friendly_name)
}
