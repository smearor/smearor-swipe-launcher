/// Macro to generate FFI boilerplate for a widget plugin that provides multiple widget types.
///
/// This macro generates a single `#[stabby::export]` entry point (`smearor_plugin_create`)
/// that constructs one of the registered widget types based on the `widget` field in the
/// plugin configuration and returns a `PluginContainer` with a manual VTable.
///
/// # Requirements
///
/// Each widget type must:
/// - Have a `meta` field with `id`, `display_name`, and `icon_name` attributes
/// - Implement `PluginMetaGetter`
/// - Implement `Plugin` and `WidgetBuilder`
/// - Implement a `new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper>` constructor
///
/// # Example
///
/// ```rust
/// use smearor_swipe_launcher_plugin_api::widget_factory_plugin;
///
/// widget_factory_plugin! {
///     "cpu" => cpu_widget => CpuWidget,
///     "memory" => memory_widget => MemoryWidget,
/// };
/// ```
#[macro_export]
macro_rules! widget_factory_plugin {
    (
        $(
            $name:literal => $widget_ident:ident => $widget_type:ty
        ),+ $(,)?
    ) => {
        paste::paste! {
            $(
                unsafe extern "C" fn [<destroy_ $widget_ident>](instance: *mut core::ffi::c_void) {
                    if !instance.is_null() {
                        unsafe {
                            let _ = Box::from_raw(instance as *mut $widget_type);
                        }
                    }
                }

                unsafe extern "C" fn [<build_widget_ $widget_ident>](
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

                unsafe extern "C" fn [<on_message_ $widget_ident>](
                    instance: *mut core::ffi::c_void,
                    message: *mut core::ffi::c_void,
                ) {
                    if instance.is_null() {
                        return;
                    }
                    let widget = unsafe { &mut *(instance as *mut $widget_type) };
                    <$widget_type as $crate::WidgetPlugin>::on_message(widget, message);
                }

                unsafe extern "C" fn [<start_ $widget_ident>](instance: *mut core::ffi::c_void) {
                    if instance.is_null() {
                        return;
                    }
                    let widget = unsafe { &mut *(instance as *mut $widget_type) };
                    <$widget_type as $crate::WidgetPlugin>::start(widget);
                }

                static [<VTABLE_ $widget_ident>]: $crate::WidgetPluginVTable = $crate::WidgetPluginVTable {
                    destroy: [<destroy_ $widget_ident>],
                    build_widget: [<build_widget_ $widget_ident>],
                    on_message: [<on_message_ $widget_ident>],
                    start: [<start_ $widget_ident>],
                    render_graphic: None,
                    render_html: None,
                };
            )+

            #[stabby::export]
            pub extern "C" fn smearor_plugin_create(
                config_json: *const i8,
                config_len: usize,
                core_context: *mut core::ffi::c_void,
            ) -> stabby::result::Result<
                *mut core::ffi::c_void,
                $crate::PluginConstructionErrorWrapper,
            > {
                let subscriber = tracing_subscriber::FmtSubscriber::builder()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::from_default_env()
                            .add_directive(tracing::Level::DEBUG.into()),
                    )
                    .finish();
                let _ = tracing::subscriber::set_global_default(subscriber);

                let config = match $crate::PluginConfig::new(config_json, config_len) {
                    Ok(config) => config,
                    Err(e) => return stabby::result::Result::Err(e),
                };

                let ffi_context = if core_context.is_null() {
                    None
                } else {
                    Some(unsafe { *(core_context as *mut $crate::FfiCoreContext) })
                };

                let widget_name = config.config.get("widget")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();

                match widget_name {
                    $(
                        $name => {
                            match <$widget_type>::new(config.clone(), ffi_context.clone()) {
                                Ok(widget) => {
                                    let container = $crate::WidgetPluginContainer {
                                        instance: Box::into_raw(Box::new(widget)) as *mut core::ffi::c_void,
                                        vtable: & [<VTABLE_ $widget_ident>],
                                        vtable_version: $crate::PLUGIN_VTABLE_VERSION,
                                    };
                                    stabby::result::Result::Ok(
                                        Box::into_raw(Box::new(container)) as *mut core::ffi::c_void
                                    )
                                }
                                Err(e) => stabby::result::Result::Err(e),
                            }
                        }
                    )+
                    _ => stabby::result::Result::Err(
                        $crate::PluginConstructionErrorWrapper::new(
                            $crate::PluginConstructionError::FailedToParseWidgetConfig,
                            format!("unknown widget: {}", widget_name).into(),
                        )
                    ),
                }
            }
        }
    };
}
