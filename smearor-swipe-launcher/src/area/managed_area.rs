use crate::AreaConfig;
use gtk4::Overlay;
use gtk4::Widget;

/// Represents a dynamically managed area in the layout
#[derive(Debug, Clone)]
pub struct ManagedArea {
    /// Unique identifier for the area
    pub id: String,

    /// Configuration for this area
    pub config: AreaConfig,

    /// The GTK widget representing this area
    pub widget: Widget,

    /// The overlay for this area (for transient sub-areas)
    pub overlay: Option<Overlay>,

    /// The source area widget (for transient areas only)
    pub source_area_widget: Option<Widget>,

    /// Whether this area is transient (auto-closing)
    pub is_transient: bool,
}
