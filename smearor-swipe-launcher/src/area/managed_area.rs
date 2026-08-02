use crate::area::backend::AreaBackend;
use smearor_model_area::AreaConfig;

/// Represents a dynamically managed area in the layout.
///
/// Generic over the area backend `B`, which determines the concrete widget
/// and overlay types. For GTK instances, `B = GtkBackend` uses
/// `gtk4::Widget` and `gtk4::Overlay`. For headless instances,
/// `B = HeadlessBackend` uses no-op types.
#[derive(Debug, Clone)]
pub struct ManagedArea<B: AreaBackend> {
    /// Unique identifier for the area
    pub id: String,

    /// Configuration for this area
    pub config: AreaConfig,

    /// The widget representing this area
    pub widget: B::Widget,

    /// The overlay for this area (for transient sub-areas)
    pub overlay: Option<B::Overlay>,

    /// The source area widget (for transient areas only)
    pub source_area_widget: Option<B::Widget>,

    /// The source area ID (for transient areas only) — the area that
    /// opened this transient area, used to restore visibility on close.
    pub source_area_id: Option<String>,

    /// Whether this area is transient (auto-closing)
    pub is_transient: bool,
}
