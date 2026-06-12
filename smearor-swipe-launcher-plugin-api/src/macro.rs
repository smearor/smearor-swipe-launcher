/// Macro to generate FFI boilerplate for service plugins.
///
/// This macro generates all the necessary FFI functions and vtable for a service type,
/// including:
/// - `destroy_{service_type}`: Destroys the service instance
/// - `get_id_{service_type}`: Returns the service ID
/// - `get_display_name_{service_type}`: Returns the service display name
/// - `on_message_{service_type}`: Handles incoming messages
/// - `VTABLE_{service_type}`: Static vtable for the service
/// - `smearor_service_create`: Entry point for creating the service
///
/// # Requirements
///
/// The service type must:
/// - Have a `meta` field with `id` and `display_name` attributes
/// - Implement a `new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError>` constructor
/// - Implement a `handle_envelope_message(message: FfiEnvelope)` method
///
/// # Example
///
/// ```rust
/// use smearor_swipe_launcher_plugin_api::service_plugin;
///
/// service_plugin!(MyService);
/// ```
#[macro_export]
macro_rules! service_plugin {
    ($service_type:ty) => {
        paste::paste! {
            unsafe extern "C" fn [<destroy_ $service_type:snake>](service: *mut ()) {
                if !service.is_null() {
                    unsafe {
                        let _ = Box::from_raw(service as *mut $service_type);
                    }
                }
            }

            unsafe extern "C" fn [<get_id_ $service_type:snake>](service: *mut ()) -> abi_stable::std_types::RString {
                if service.is_null() {
                    return abi_stable::std_types::RString::from("");
                }
                let service = unsafe { &*(service as *const $service_type) };
                abi_stable::std_types::RString::from(service.meta.id.clone())
            }

            unsafe extern "C" fn [<get_display_name_ $service_type:snake>](service: *mut ()) -> abi_stable::std_types::RString {
                if service.is_null() {
                    return abi_stable::std_types::RString::from("");
                }
                let service = unsafe { &*(service as *const $service_type) };
                abi_stable::std_types::RString::from(service.meta.display_name.clone())
            }

            unsafe extern "C" fn [<on_message_ $service_type:snake>](service: *mut (), message: $crate::FfiEnvelope) {
                if service.is_null() {
                    return;
                }
                let service = unsafe { &*(service as *const $service_type) };
                smearor_swipe_launcher_plugin_api::MessageHandler::handle_envelope_message(service, message);
            }

            static [<VTABLE_ $service_type:snake>]: $crate::ServiceVTable = $crate::ServiceVTable {
                destroy: [<destroy_ $service_type:snake>],
                get_id: [<get_id_ $service_type:snake>],
                get_display_name: [<get_display_name_ $service_type:snake>],
                on_message: [<on_message_ $service_type:snake>],
            };

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn smearor_service_create(
                config_json: *const i8,
                config_len: usize,
                core_context: $crate::FfiCoreContext,
            ) -> abi_stable::std_types::RResult<$crate::LoadedService, $crate::PluginConstructionError> {
                let subscriber = tracing_subscriber::FmtSubscriber::builder()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::from_default_env()
                            .add_directive(tracing::Level::DEBUG.into()),
                    )
                    .finish();
                let _ = tracing::subscriber::set_global_default(subscriber);

                let config = match $crate::PluginConfig::new(config_json, config_len) {
                    Ok(config) => config,
                    Err(e) => {
                        return abi_stable::std_types::RResult::RErr(e);
                    }
                };

                let core_context = if core_context.core_obj.is_null() {
                    None
                } else {
                    Some(core_context)
                };

                match <$service_type>::new(config, core_context) {
                    Ok(service) => {
                        abi_stable::std_types::RResult::ROk($crate::LoadedService::new(
                            service,
                            abi_stable::RRef::new(&[<VTABLE_ $service_type:snake>]),
                        ))
                    }
                    Err(e) => abi_stable::std_types::RResult::RErr(e),
                }
            }
        }
    };
}

/// Macro to generate FFI boilerplate for widget plugins.
///
/// This macro generates all the necessary FFI functions and vtable for a widget type,
/// including:
/// - `destroy_{widget_type}`: Destroys the widget instance
/// - `get_id_{widget_type}`: Returns the widget ID
/// - `get_display_name_{widget_type}`: Returns the widget display name
/// - `get_icon_name_{widget_type}`: Returns the widget icon name
/// - `build_widget_{widget_type}`: Builds the FFI widget
/// - `on_message_{widget_type}`: Handles incoming messages
/// - `VTABLE_{widget_type}`: Static vtable for the widget
/// - `smearor_plugin_create`: Entry point for creating the widget
///
/// # Requirements
///
/// The widget type must:
/// - Have a `meta` field with `id`, `display_name`, and `icon_name` attributes
/// - Implement a `new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError>` constructor
/// - Implement a `build_ffi_widget(plugin: *mut ()) -> FfiWidget` method
/// - Implement a `handle_envelope_message(message: FfiEnvelope)` method
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
            unsafe extern "C" fn [<destroy_ $widget_type:snake>](plugin: *mut ()) {
                if !plugin.is_null() {
                    unsafe {
                        let _ = Box::from_raw(plugin as *mut $widget_type);
                    }
                }
            }

            unsafe extern "C" fn [<get_id_ $widget_type:snake>](plugin: *mut ()) -> abi_stable::std_types::RString {
                if plugin.is_null() {
                    return abi_stable::std_types::RString::from("");
                }
                let widget = unsafe { &*(plugin as *const $widget_type) };
                widget.meta.id.clone()
            }

            unsafe extern "C" fn [<get_display_name_ $widget_type:snake>](plugin: *mut ()) -> abi_stable::std_types::RString {
                if plugin.is_null() {
                    return abi_stable::std_types::RString::from("");
                }
                let widget = unsafe { &*(plugin as *const $widget_type) };
                widget.meta.display_name.clone()
            }

            unsafe extern "C" fn [<get_icon_name_ $widget_type:snake>](plugin: *mut ()) -> abi_stable::derive_macro_reexports::ROption<abi_stable::std_types::RString> {
                if plugin.is_null() {
                    return abi_stable::derive_macro_reexports::ROption::RNone;
                }
                let widget = unsafe { &*(plugin as *const $widget_type) };
                widget.meta.icon_name.clone()
            }

            unsafe extern "C" fn [<build_widget_ $widget_type:snake>](plugin: *mut ()) -> $crate::FfiWidget {
                <$widget_type as $crate::FfiWidgetBuilder>::build_ffi_widget(plugin)
            }

            unsafe extern "C" fn [<on_message_ $widget_type:snake>](plugin: *mut (), message: $crate::FfiEnvelope) {
                if plugin.is_null() {
                    return;
                }
                let widget = unsafe { &*(plugin as *const $widget_type) };
                smearor_swipe_launcher_plugin_api::MessageHandler::handle_envelope_message(widget, message);
            }

            static [<VTABLE_ $widget_type:snake>]: $crate::PluginVTable = $crate::PluginVTable {
                destroy: [<destroy_ $widget_type:snake>],
                get_id: [<get_id_ $widget_type:snake>],
                get_display_name: [<get_display_name_ $widget_type:snake>],
                get_icon_name: [<get_icon_name_ $widget_type:snake>],
                build_widget: [<build_widget_ $widget_type:snake>],
                on_message: [<on_message_ $widget_type:snake>],
            };

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn smearor_plugin_create(
                config_json: *const i8,
                config_len: usize,
                core_context: $crate::FfiCoreContext,
            ) -> abi_stable::std_types::RResult<$crate::LoadedPlugin, $crate::PluginConstructionError> {
                let subscriber = tracing_subscriber::FmtSubscriber::builder()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::from_default_env()
                            .add_directive(tracing::Level::DEBUG.into()),
                    )
                    .finish();
                let _ = tracing::subscriber::set_global_default(subscriber);

                let config = match $crate::PluginConfig::new(config_json, config_len) {
                    Ok(config) => config,
                    Err(e) => {
                        return abi_stable::std_types::RResult::RErr(e);
                    }
                };

                let core_context = if core_context.core_obj.is_null() {
                    None
                } else {
                    Some(core_context)
                };

                match <$widget_type>::new(config, core_context) {
                    Ok(widget) => {
                        abi_stable::std_types::RResult::ROk($crate::LoadedPlugin::new(
                            widget,
                            abi_stable::RRef::new(&[<VTABLE_ $widget_type:snake>]),
                        ))
                    }
                    Err(e) => abi_stable::std_types::RResult::RErr(e),
                }
            }
        }
    };
}
