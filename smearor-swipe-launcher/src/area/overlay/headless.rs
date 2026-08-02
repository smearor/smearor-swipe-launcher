use crate::area::overlay::area::AreaOverlay;
use crate::area::widget::headless::HeadlessWidget;

/// A no-op overlay for headless instances.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct HeadlessOverlay;

impl AreaOverlay for HeadlessOverlay {
    type Widget = HeadlessWidget;

    fn area_set_child(&self, _child: &HeadlessWidget) {}

    fn area_add_overlay(&self, _overlay: &HeadlessOverlay) {}

    fn area_remove_overlay(&self, _overlay: &HeadlessOverlay) {}

    fn area_add_css_class(&self, _class: &str) {}

    fn area_child(&self) -> Option<HeadlessWidget> {
        None
    }
}
