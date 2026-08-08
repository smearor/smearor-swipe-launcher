/// Macro to generate FFI boilerplate for widget plugins using stabby.
///
/// This macro generates a `#[stabby::export]` entry point (`smearor_plugin_create`)
/// that constructs the widget and returns a `PluginContainer` with a manual VTable.
///
/// # Requirements
///
/// The widget type must:
/// - Have a `meta` field with `id`, `display_name`, and `icon_name` attributes
/// - Implement `PluginMetaGetter`
/// - Implement `Plugin` and `WidgetBuilder`
/// - Implement a `new(config: PluginConfig, executor: PluginExecutor, broker: MessageBrokerHandle) -> Result<Self, PluginConstructionErrorWrapper>` constructor
///
/// # Example
///
/// ```rust
/// use smearor_swipe_launcher_plugin_api::widget_plugin;
///
/// widget_plugin!(MyWidget);
/// ```
#[macro_export]
macro_rules! widget_plugin {
    ($widget_type:ty) => {
        paste::paste! {
            unsafe extern "C" fn [<destroy_ $widget_type:snake>](instance: *mut core::ffi::c_void) {
                if !instance.is_null() {
                    unsafe {
                        let _ = Box::from_raw(instance as *mut $widget_type);
                    }
                }
            }

            unsafe extern "C" fn [<build_widget_ $widget_type:snake>](
                instance: *mut core::ffi::c_void,
            ) -> $crate::FfiWidget {
                if instance.is_null() {
                    return $crate::FfiWidget::null();
                }
                let result = std::panic::catch_unwind(|| {
                    let widget = unsafe { &mut *(instance as *mut $widget_type) };
                    let status_page = <$widget_type as $crate::WidgetBuilder>::build_widget(widget);
                    $crate::FfiWidget::new(status_page)
                });
                result.unwrap_or($crate::FfiWidget::null())
            }

            unsafe extern "C" fn [<on_message_ $widget_type:snake>](
                instance: *mut core::ffi::c_void,
                message: *mut core::ffi::c_void,
            ) {
                if instance.is_null() {
                    return;
                }
                let widget = unsafe { &mut *(instance as *mut $widget_type) };
                <$widget_type as $crate::WidgetPlugin>::on_message(widget, message);
            }

            unsafe extern "C" fn [<start_ $widget_type:snake>](instance: *mut core::ffi::c_void) {
                if instance.is_null() {
                    return;
                }
                let widget = unsafe { &mut *(instance as *mut $widget_type) };
                <$widget_type as $crate::WidgetPlugin>::start(widget);
            }

            static [<VTABLE_ $widget_type:snake>]: $crate::WidgetPluginVTable = $crate::WidgetPluginVTable {
                destroy: [<destroy_ $widget_type:snake>],
                build_widget: [<build_widget_ $widget_type:snake>],
                on_message: [<on_message_ $widget_type:snake>],
                start: [<start_ $widget_type:snake>],
                render_graphic: None,
                render_html: None,
            };

            #[stabby::export]
            pub extern "C" fn smearor_plugin_create(
                config_json: *const i8,
                config_len: usize,
                core_context: *mut core::ffi::c_void,
            ) -> stabby::result::Result<
                *mut core::ffi::c_void,
                $crate::PluginConstructionErrorWrapper,
            > {
                let config = match $crate::PluginConfig::new(config_json, config_len) {
                    Ok(config) => config,
                    Err(e) => return stabby::result::Result::Err(e),
                };

                let ffi_context = if core_context.is_null() {
                    None
                } else {
                    Some(unsafe { *(core_context as *mut $crate::FfiCoreContext) })
                };

                $crate::log_forward::init_plugin_tracing(
                    ffi_context.and_then(|ctx| ctx.log_forward),
                );

                match <$widget_type>::new(config, ffi_context) {
                    Ok(widget) => {
                        let container = $crate::WidgetPluginContainer {
                            instance: Box::into_raw(Box::new(widget)) as *mut core::ffi::c_void,
                            vtable: & [<VTABLE_ $widget_type:snake>],
                            vtable_version: $crate::PLUGIN_VTABLE_VERSION,
                        };
                        stabby::result::Result::Ok(
                            Box::into_raw(Box::new(container)) as *mut core::ffi::c_void
                        )
                    }
                    Err(e) => stabby::result::Result::Err(e),
                }
            }
        }
    };
}
