use gtk4::Widget;
use gtk4::ffi::GtkWidget;
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::Cast;
use gtk4::prelude::WidgetExt;

use crate::sanitize_css_class_name;

/// An FFI-safe wrapper around a GTK widget pointer.
#[repr(C)]
pub struct FfiWidget {
    pub raw_widget: *mut GtkWidget,
}

impl FfiWidget {
    pub fn new(widget: Widget) -> Self {
        let stable_pointer: *mut GtkWidget = widget.to_glib_full();
        Self { raw_widget: stable_pointer }
    }

    pub fn null() -> Self {
        Self {
            raw_widget: std::ptr::null_mut(),
        }
    }
}

/// Applies the automatic `widget-{plugin_id}` CSS class to a widget.
///
/// The `plugin_id` is sanitized to ensure it only contains valid CSS class
/// name characters (`[a-zA-Z0-9_-]`).
pub fn apply_widget_css_class(widget: &impl WidgetExt, plugin_id: &str) {
    widget.add_css_class(&format!("widget-{}", sanitize_css_class_name(plugin_id)));
}

/// Applies the automatic `widget-{plugin_id}` CSS class and user-configured
/// `css_classes` from `WidgetLayout` to a widget.
///
/// The `plugin_id` is sanitized. User-configured `css_classes` are applied
/// verbatim — invalid CSS class names are silently ignored by GTK4.
pub fn apply_widget_css_classes(widget: &impl WidgetExt, plugin_id: &str, user_css_classes: &[String]) {
    apply_widget_css_class(widget, plugin_id);
    for class in user_css_classes {
        widget.add_css_class(class);
    }
}

/// Trait for types that can build a GTK widget.
pub trait WidgetBuilder {
    fn build_widget(&mut self) -> Widget;
}

/// Trait for building an FfiWidget from a raw plugin pointer.
pub trait FfiWidgetBuilder {
    fn build_ffi_widget(plugin: *mut ()) -> FfiWidget;
}

impl<T: WidgetBuilder> FfiWidgetBuilder for T {
    fn build_ffi_widget(plugin: *mut ()) -> FfiWidget {
        if plugin.is_null() {
            return FfiWidget::null();
        }

        let result = std::panic::catch_unwind(|| {
            let widget = unsafe { &mut *(plugin as *mut Self) };
            let status_page = Self::build_widget(widget);
            FfiWidget::new(status_page.upcast::<Widget>())
        });

        result.unwrap_or(FfiWidget::null())
    }
}
