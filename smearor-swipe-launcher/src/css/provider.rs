use gtk4::CssProvider;
use gtk4::gdk::Display;

/// Creates and registers the built-in CSS provider.
///
/// The built-in `resources/style.css` is loaded with
/// `STYLE_PROVIDER_PRIORITY_APPLICATION`. Global user CSS
/// (`~/.config/smearor/style.css`) and per-instance CSS are loaded by
/// `CssWatcher`, which also handles hot-reload.
pub fn create_css_provider() {
    if let Some(display) = Display::default() {
        let provider = CssProvider::new();
        provider.load_from_string(include_str!("../../../resources/style.css"));
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

/// Registers a global CSS provider that scales all widget font sizes.
///
/// Uses `STYLE_PROVIDER_PRIORITY_APPLICATION + 1` so it overrides the
/// built-in `style.css` (which is at `APPLICATION`), but is itself
/// overridden by per-widget scoped CSS at `APPLICATION + 2`.
pub fn apply_global_scaled_css(scale: f32) {
    if scale == 1.0 {
        return;
    }
    let key = format!("global-scale-{}", (scale * 100.0).round() as i32);
    let css = format!(
        ".widget-main-text {{ font-size: {}px; }}
         .widget-info-text {{ font-size: {}px; }}
         .nerd-icon {{ font-size: {}em; }}
         .clock-time {{ font-size: {}px; }}
         .sysinfo-icon {{ font-size: {}em; }}",
        (14.0 * scale).round(),
        (10.0 * scale).round(),
        1.5 * scale as f64,
        (32.0 * scale).round(),
        1.5 * scale as f64,
    );
    smearor_swipe_launcher_plugin_api::register_css_once(&key, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
}
