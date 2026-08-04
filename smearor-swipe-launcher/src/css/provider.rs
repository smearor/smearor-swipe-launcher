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
