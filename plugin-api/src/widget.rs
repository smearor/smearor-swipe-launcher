use gtk4::Widget;
use gtk4::ffi::GtkWidget;
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::Cast;

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
