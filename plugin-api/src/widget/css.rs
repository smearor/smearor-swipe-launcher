use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;

use gtk4::CssProvider;
use gtk4::gdk;
use gtk4::prelude::IsA;
use gtk4::prelude::WidgetExt;

/// Minimum and maximum allowed scale values.
const SCALE_MIN: f32 = 0.5;
const SCALE_MAX: f32 = 3.0;

/// Clamps the scale to the valid range and guards against NaN/infinity.
///
/// Returns 1.0 for NaN, infinity, or values outside [SCALE_MIN, SCALE_MAX].
pub fn sanitize_scale(scale: f32) -> f32 {
    if scale.is_nan() || scale.is_infinite() {
        return 1.0;
    }
    scale.clamp(SCALE_MIN, SCALE_MAX)
}

static REGISTERED_CSS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Registers a CSS rule on the display exactly once per unique key.
///
/// Subsequent calls with the same key are no-ops. This prevents CssProvider
/// accumulation when widgets are rebuilt (layout changes, config reloads, etc.).
pub fn register_css_once(key: &str, css: &str, priority: u32) {
    let set = REGISTERED_CSS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = set.lock().unwrap();
    if guard.insert(key.to_string()) {
        if let Some(display) = gdk::Display::default() {
            let provider = CssProvider::new();
            provider.load_from_string(css);
            gtk4::style_context_add_provider_for_display(&display, &provider, priority);
        }
    }
}

/// Generates a scoped CSS class for per-widget font scaling and adds it to the widget's root container.
///
/// Returns the CSS class name (e.g. "scale-200") that was added to the container.
pub fn apply_widget_scaled_css(container: &impl IsA<gtk4::Widget>, scale: f32) -> String {
    let class_name = format!("scale-{}", (scale * 100.0).round() as i32);
    let css = format!(
        ".{class_name} .widget-main-text {{ font-size: {}px; }}
         .{class_name} .widget-info-text {{ font-size: {}px; }}
         .{class_name} .nerd-icon {{ font-size: {}em; }}
         .{class_name} .clock-time {{ font-size: {}px; }}
         .{class_name} .sysinfo-icon {{ font-size: {}em; }}",
        (14.0 * scale).round(),
        (10.0 * scale).round(),
        1.5 * scale as f64,
        (32.0 * scale).round(),
        1.5 * scale as f64,
    );
    register_css_once(&class_name, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
    container.add_css_class(&class_name);
    class_name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_scale_default_returns_one() {
        assert_eq!(sanitize_scale(1.0), 1.0);
    }

    #[test]
    fn sanitize_scale_clamps_below_min() {
        assert_eq!(sanitize_scale(0.0), SCALE_MIN);
        assert_eq!(sanitize_scale(-1.0), SCALE_MIN);
        assert_eq!(sanitize_scale(0.49), SCALE_MIN);
    }

    #[test]
    fn sanitize_scale_clamps_above_max() {
        assert_eq!(sanitize_scale(3.5), SCALE_MAX);
        assert_eq!(sanitize_scale(10.0), SCALE_MAX);
    }

    #[test]
    fn sanitize_scale_nan_returns_one() {
        assert_eq!(sanitize_scale(f32::NAN), 1.0);
    }

    #[test]
    fn sanitize_scale_positive_infinity_returns_one() {
        assert_eq!(sanitize_scale(f32::INFINITY), 1.0);
    }

    #[test]
    fn sanitize_scale_negative_infinity_returns_one() {
        assert_eq!(sanitize_scale(f32::NEG_INFINITY), 1.0);
    }

    #[test]
    fn sanitize_scale_min_boundary() {
        assert_eq!(sanitize_scale(SCALE_MIN), SCALE_MIN);
    }

    #[test]
    fn sanitize_scale_max_boundary() {
        assert_eq!(sanitize_scale(SCALE_MAX), SCALE_MAX);
    }

    #[test]
    fn sanitize_scale_mid_range_unchanged() {
        assert_eq!(sanitize_scale(1.5), 1.5);
        assert_eq!(sanitize_scale(2.0), 2.0);
        assert_eq!(sanitize_scale(0.75), 0.75);
    }

    #[test]
    fn register_css_once_does_not_panic_without_display() {
        // In a headless test environment, gdk::Display::default() returns None.
        // register_css_once should silently skip CSS registration without panicking.
        register_css_once("test-key-no-display", ".test { color: red; }", gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    #[test]
    fn register_css_once_deduplicates_by_key() {
        // Calling with the same key twice should not panic.
        // The second call is a no-op (the key is already in the HashSet).
        register_css_once("test-dedup-key", ".test { color: red; }", gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
        register_css_once("test-dedup-key", ".test { color: blue; }", gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
