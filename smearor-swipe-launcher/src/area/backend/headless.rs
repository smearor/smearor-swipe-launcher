use crate::SwipeLauncherConfig;
use crate::area::backend::area::AreaBackend;
use crate::area::container::headless::HeadlessContainer;
use crate::area::error::CreateAreaError;
use crate::area::overlay::HeadlessOverlay;
use crate::area::widget::HeadlessWidget;
use crate::plugin::PluginManager;
use smearor_model_area::AreaConfig;
use smearor_model_area::AreaTransition;

/// Headless backend using no-op types for instances without GTK.
#[derive(Clone, Default)]
pub struct HeadlessBackend;

impl AreaBackend for HeadlessBackend {
    type Widget = HeadlessWidget;
    type Overlay = HeadlessOverlay;
    type Container = HeadlessContainer;

    fn create_area_widget(
        _plugin_manager: &PluginManager,
        _config: &SwipeLauncherConfig,
        _area_id: &str,
        _area_config: &AreaConfig,
    ) -> Result<HeadlessWidget, CreateAreaError> {
        Ok(HeadlessWidget)
    }

    fn create_overlay(_child: &HeadlessWidget) -> HeadlessOverlay {
        HeadlessOverlay
    }

    fn animate_addition(_overlay: &HeadlessOverlay, _transition: &AreaTransition) {}

    fn animate_removal(_widget: &HeadlessWidget, _transition: &AreaTransition, callback: Box<dyn Fn() + 'static>) {
        callback();
    }
}
