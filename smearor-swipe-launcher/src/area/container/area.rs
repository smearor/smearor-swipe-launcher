use crate::area::overlay::AreaOverlay;

/// Trait for the main container that holds area overlays.
///
/// Implemented for `gtk4::Box` (GTK mode) and `HeadlessContainer` (headless mode).
/// Method names are prefixed to avoid conflicts with gtk4 trait methods.
pub trait AreaContainer: Clone + 'static {
    /// The overlay type that this container holds.
    type Overlay: AreaOverlay;

    /// Append an overlay to this container.
    fn append_overlay(&self, overlay: &Self::Overlay);

    /// Remove an overlay from this container.
    fn remove_overlay(&self, overlay: &Self::Overlay);
}
