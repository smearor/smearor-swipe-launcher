/// Macro to generate FFI boilerplate for a factory plugin that provides multiple
/// widget types, all implementing `GraphicRenderer`, with optional `WebRenderer`.
///
/// This is like `widget_factory_plugin!` but every widget gets `render_graphic: Some(...)`.
/// Entries marked with `=> html` also get `render_html: Some(...)`.
///
/// # Requirements
///
/// Each widget type must:
/// - Implement `PluginMetaGetter`
/// - Implement `Plugin` and `WidgetBuilder`
/// - Implement `GraphicRenderer`
/// - Implement `WebRenderer` (only for entries marked `=> html`)
/// - Implement a `new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper>` constructor
///
/// # Example
///
/// ```rust
/// use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;
///
/// widget_factory_plugin_graphic! {
///     "weather" => weather_widget => WeatherWidget => html,
///     "weather_today" => weather_today_widget => WeatherTodayWidget,
///     "weather_tomorrow" => weather_tomorrow_widget => WeatherTomorrowWidget,
/// }
/// ```
#[macro_export]
macro_rules! widget_factory_plugin_graphic {
    (
        $(
            $name:literal => $widget_ident:ident => $widget_type:ty $(=> $html:tt)?
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

                unsafe extern "C" fn [<render_graphic_ $widget_ident>](
                    instance: *mut core::ffi::c_void,
                    width: u32,
                    height: u32,
                ) -> $crate::FfiGraphic {
                    if instance.is_null() {
                        return $crate::FfiGraphic::null();
                    }
                    let result = std::panic::catch_unwind(|| {
                        let widget = unsafe { &*(instance as *mut $widget_type) };
                        <$widget_type as $crate::GraphicRenderer>::render_graphic(widget, width, height)
                    });
                    result.unwrap_or($crate::FfiGraphic::null())
                }

                $crate::widget_factory_plugin_graphic!(@html_fn $widget_ident $widget_type $($html)?);

                static [<VTABLE_ $widget_ident>]: $crate::WidgetPluginVTable = $crate::WidgetPluginVTable {
                    destroy: [<destroy_ $widget_ident>],
                    build_widget: [<build_widget_ $widget_ident>],
                    on_message: [<on_message_ $widget_ident>],
                    start: [<start_ $widget_ident>],
                    render_graphic: Some([<render_graphic_ $widget_ident>]),
                    render_html: $crate::widget_factory_plugin_graphic!(@html_vtable $widget_ident $($html)?),
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

                let widget_name = if widget_name.is_empty() {
                    $crate::widget_factory_plugin_graphic!(@first_name $($name),+)
                } else {
                    widget_name
                };

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

    (@html_fn $widget_ident:ident $widget_type:tt html) => {
        paste::paste! {
            unsafe extern "C" fn [<render_html_ $widget_ident>](
                instance: *mut core::ffi::c_void,
                instance_id: *const u8,
                instance_id_len: usize,
                plugin_id: *const u8,
                plugin_id_len: usize,
            ) -> $crate::FfiHtmlString {
                if instance.is_null() {
                    return $crate::FfiHtmlString::null();
                }
                let result = std::panic::catch_unwind(|| {
                    let widget = unsafe { &*(instance as *mut $widget_type) };
                    let instance_id = if instance_id.is_null() || instance_id_len == 0 {
                        String::new()
                    } else {
                        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(instance_id, instance_id_len)) }.to_string()
                    };
                    let plugin_id = if plugin_id.is_null() || plugin_id_len == 0 {
                        String::new()
                    } else {
                        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(plugin_id, plugin_id_len)) }.to_string()
                    };
                    let html = <$widget_type as $crate::WebRenderer>::render_html(widget, &instance_id, &plugin_id);
                    $crate::FfiHtmlString::from_string(html)
                });
                result.unwrap_or($crate::FfiHtmlString::null())
            }
        }
    };

    (@html_fn $widget_ident:ident $widget_type:tt) => {};

    (@html_vtable $widget_ident:ident html) => {
        paste::paste! { Some([<render_html_ $widget_ident>]) }
    };

    (@html_vtable $widget_ident:ident) => { None };

    (@first_name $first:literal $(, $rest:literal)*) => { $first };
}
