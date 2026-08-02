/// Topic for widget update notifications.
///
/// Widgets broadcast this message when their visual state changes
/// and the host needs to re-render them (headless or web).
pub const TOPIC_WIDGET_UPDATE: &str = "widget.update";
