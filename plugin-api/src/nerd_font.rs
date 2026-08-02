// Nerd Font icon name resolution.
//
// Maps human-readable icon names like `nf-fa-gamepad` to their Unicode
// codepoints using the glyph list compiled into the binary.

use gtk4::prelude::WidgetExt;
use tracing::trace;

use crate::widget::Color;

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

/// Applies a configured icon color to a GTK `Image` via a display-scoped `CssProvider`.
///
/// A unique CSS class is added to the icon widget, and a CSS rule targeting only that class
/// is loaded on the display. This follows the GTK 4.10 recommendation to avoid widget-scoped
/// `StyleContext::add_provider` (deprecated since 4.10).
pub fn apply_icon_color(icon: &gtk4::Image, color: Color) {
    let class_name = format!(
        "icon-color-{:02x}{:02x}{:02x}{:02x}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        (color.a * 255.0).round() as u8
    );
    icon.add_css_class(&class_name);
    let css = format!(
        ".{} {{ color: rgba({}, {}, {}, {}); }}",
        class_name,
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        color.a
    );
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&css);
    let display = icon.display();
    gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_USER);
}

/// Applies a configured text color to a GTK `Label` via a display-scoped `CssProvider`.
///
/// Accepts `Option<Color>` so callers can pass `None` to reset to the default CSS class.
/// On each call, all previously applied `text-color-*` CSS classes are removed from the
/// label before the new class (if any) is added. This prevents CSS class accumulation
/// across repeated `update_ui()` calls (e.g. semantic color changes Normal → Warning → Critical).
/// The dynamic CSS rule includes `opacity: 1;` to neutralise the `opacity: 0.8` from
/// `.widget-info-text`, ensuring the user's configured color is applied exactly as specified.
pub fn apply_text_color(label: &gtk4::Label, color: Option<Color>) {
    let existing_classes: Vec<String> = label
        .css_classes()
        .iter()
        .filter(|c| c.starts_with("text-color-"))
        .map(|c| c.to_string())
        .collect();

    for class_name in existing_classes {
        label.remove_css_class(&class_name);
    }

    if let Some(color) = color {
        let class_name = format!(
            "text-color-{:02x}{:02x}{:02x}{:02x}",
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            (color.a * 255.0).round() as u8
        );
        label.add_css_class(&class_name);
        let css = format!(
            ".{} {{ color: rgba({}, {}, {}, {}); opacity: 1; }}",
            class_name,
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            color.a
        );
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(&css);
        gtk4::style_context_add_provider_for_display(&label.display(), &provider, gtk4::STYLE_PROVIDER_PRIORITY_USER);
    }
}
