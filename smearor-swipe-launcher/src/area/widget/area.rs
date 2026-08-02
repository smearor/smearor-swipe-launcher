/// Trait for area widgets that support visibility and focus operations.
///
/// Implemented for `gtk4::Widget` (GTK mode) and `HeadlessWidget` (headless mode).
/// Method names are prefixed with `area_` to avoid conflicts with gtk4 trait methods.
pub trait AreaWidget: Clone + PartialEq + std::fmt::Debug + 'static {
    /// Set the visibility of the widget.
    fn area_set_visible(&self, visible: bool);

    /// Check if the widget is currently visible.
    fn area_is_visible(&self) -> bool;

    /// Grab keyboard focus to this widget.
    fn area_grab_focus(&self);

    /// Check if this widget currently has keyboard focus.
    fn area_has_focus(&self) -> bool;

    /// Add a CSS class to the widget.
    fn area_add_css_class(&self, class: &str);

    /// Remove a CSS class from the widget.
    fn area_remove_css_class(&self, class: &str);
}
