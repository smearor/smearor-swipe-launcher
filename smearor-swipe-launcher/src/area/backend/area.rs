use crate::SwipeLauncherConfig;
use crate::area::container::AreaContainer;
use crate::area::error::CreateAreaError;
use crate::area::overlay::AreaOverlay;
use crate::area::widget::AreaWidget;
use crate::plugin::PluginManager;
use smearor_model_area::AreaConfig;
use smearor_model_area::AreaTransition;

/// Backend abstraction that ties together widget, overlay, and container types.
///
/// This trait allows `AreaManager` and `ManagedArea` to be generic over a
/// single type parameter `B` instead of three separate type parameters.
/// `GtkBackend` provides full GTK widget support, while `HeadlessBackend`
/// provides no-op implementations for instances that do not use GTK.
pub trait AreaBackend: Clone + Send + Sync + 'static {
    /// The widget type for areas.
    type Widget: AreaWidget;

    /// The overlay type for areas.
    type Overlay: AreaOverlay<Widget = Self::Widget>;

    /// The container type that holds area overlays.
    type Container: AreaContainer<Overlay = Self::Overlay>;

    /// Create a widget for an area from its configuration.
    fn create_area_widget(
        plugin_manager: &PluginManager,
        config: &SwipeLauncherConfig,
        area_id: &str,
        area_config: &AreaConfig,
    ) -> Result<Self::Widget, CreateAreaError>;

    /// Create an overlay with the given child widget.
    fn create_overlay(child: &Self::Widget) -> Self::Overlay;

    /// Animate widget addition (no-op for headless).
    fn animate_addition(overlay: &Self::Overlay, transition: &AreaTransition);

    /// Animate widget removal, calling the callback when the animation
    /// completes. For headless, the callback is invoked immediately.
    fn animate_removal(widget: &Self::Widget, transition: &AreaTransition, callback: Box<dyn Fn() + 'static>);
}
