use crate::area::widget::area::AreaWidget;

/// A no-op widget for headless instances that do not use GTK.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct HeadlessWidget;

impl AreaWidget for HeadlessWidget {
    fn area_set_visible(&self, _visible: bool) {}

    fn area_is_visible(&self) -> bool {
        true
    }

    fn area_grab_focus(&self) {}

    fn area_has_focus(&self) -> bool {
        false
    }

    fn area_add_css_class(&self, _class: &str) {}

    fn area_remove_css_class(&self, _class: &str) {}
}
