use crate::area::widget::area::AreaWidget;
use gtk4::Widget;
use gtk4::prelude::*;

impl AreaWidget for Widget {
    fn area_set_visible(&self, visible: bool) {
        self.set_visible(visible);
    }

    fn area_is_visible(&self) -> bool {
        self.is_visible()
    }

    fn area_grab_focus(&self) {
        self.grab_focus();
    }

    fn area_has_focus(&self) -> bool {
        self.has_focus()
    }

    fn area_add_css_class(&self, class: &str) {
        self.add_css_class(class);
    }

    fn area_remove_css_class(&self, class: &str) {
        self.remove_css_class(class);
    }
}
