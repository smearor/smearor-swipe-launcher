use gtk4::CssProvider;
use gtk4::gdk::Display;

pub fn create_css_provider() {
    if let Some(display) = Display::default() {
        let provider = CssProvider::new();
        provider.load_from_string(include_str!("../../resources/style.css"));
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
