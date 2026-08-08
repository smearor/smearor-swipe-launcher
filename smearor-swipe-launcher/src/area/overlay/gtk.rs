use crate::area::overlay::area::AreaOverlay;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::prelude::*;

impl AreaOverlay for Overlay {
    type Widget = Widget;

    fn area_set_child(&self, child: &Widget) {
        self.set_child(Some(child));
    }

    fn area_add_overlay(&self, overlay: &Overlay) {
        self.add_overlay(overlay);
    }

    fn area_remove_overlay(&self, overlay: &Overlay) {
        self.remove_overlay(overlay);
    }

    fn area_add_css_class(&self, class: &str) {
        self.add_css_class(class);
    }

    fn area_child(&self) -> Option<Widget> {
        self.child()
    }
}
