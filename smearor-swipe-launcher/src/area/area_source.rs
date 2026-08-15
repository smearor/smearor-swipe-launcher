use crate::area::backend::AreaBackend;

/// Result of locating the area that contains a specific plugin.
///
/// Returned by `AreaManager::find_area_source_containing_plugin`.
#[derive(Debug, Clone)]
pub struct AreaSource<B: AreaBackend> {
    /// The overlay of the containing area, if found.
    pub overlay: Option<B::Overlay>,
    /// The widget of the containing area, if found.
    pub widget: Option<B::Widget>,
    /// The ID of the containing area, if found.
    pub area_id: Option<String>,
}

impl<B: AreaBackend> AreaSource<B> {
    /// Creates an empty `AreaSource` (all fields `None`).
    pub fn none() -> Self {
        Self {
            overlay: None,
            widget: None,
            area_id: None,
        }
    }

    /// Returns `true` if all fields are `None`.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.overlay.is_none() && self.widget.is_none() && self.area_id.is_none()
    }
}
