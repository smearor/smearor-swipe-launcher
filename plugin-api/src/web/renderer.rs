//! Renderer trait for web-based display surfaces.

/// Trait for widgets that can render to HTML for web instances.
///
/// Used by web instances that serve launcher content via HTTP.
/// See `concepts/WEB_INSTANCE_CONCEPT.md`.
pub trait WebRenderer {
    /// Render the widget as an HTML fragment.
    ///
    /// `instance_id` and `plugin_id` are provided for data-attribute wiring
    /// (e.g. `data-plugin-id`, `data-click-topic`).
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String;
}
