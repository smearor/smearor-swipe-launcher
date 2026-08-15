use crate::area::widget::AreaWidget;

/// Trait for area overlays that can contain widgets and nested overlays.
///
/// Implemented for `gtk4::Overlay` (GTK mode) and `HeadlessOverlay` (headless mode).
/// Method names are prefixed with `area_` to avoid conflicts with gtk4 trait methods.
pub trait AreaOverlay: Clone + PartialEq + std::fmt::Debug + 'static {
    /// The widget type contained in this overlay.
    type Widget: AreaWidget;

    /// Set the main child widget of this overlay.
    #[allow(dead_code)]
    fn area_set_child(&self, child: &Self::Widget);

    /// Add a nested overlay on top of the child widget.
    fn area_add_overlay(&self, overlay: &Self);

    /// Remove a nested overlay.
    fn area_remove_overlay(&self, overlay: &Self);

    /// Add a CSS class to this overlay.
    #[allow(dead_code)]
    fn area_add_css_class(&self, class: &str);

    /// Retrieve the current child widget, if any.
    fn area_child(&self) -> Option<Self::Widget>;
}
