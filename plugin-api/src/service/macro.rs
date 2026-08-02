/// Macro to generate FFI boilerplate for service plugins using stabby.
///
/// This macro generates a `#[stabby::export]` entry point (`smearor_service_create`)
/// that constructs the service and returns a `ServiceContainer` with a manual VTable.
///
/// # Requirements
///
/// The service type must:
/// - Have a `meta` field with `id` and `display_name` attributes
/// - Implement `PluginMetaGetter`
/// - Implement `Service`
/// - Implement a `new(config: PluginConfig, executor: PluginExecutor, broker: MessageBrokerHandle) -> Result<Self, PluginConstructionErrorWrapper>` constructor
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
            unsafe extern "C" fn [<destroy_ $service_type:snake>](instance: *mut core::ffi::c_void) {
                if !instance.is_null() {
                    unsafe {
                        let _ = Box::from_raw(instance as *mut $service_type);
                    }
                }
            }

            unsafe extern "C" fn [<on_message_ $service_type:snake>](
                instance: *mut core::ffi::c_void,
                message: *mut core::ffi::c_void,
            ) {
                if instance.is_null() {
                    return;
                }
                let service = unsafe { &mut *(instance as *mut $service_type) };
                smearor_swipe_launcher_plugin_api::ServicePlugin::on_message(service, message);
            }

            unsafe extern "C" fn [<start_ $service_type:snake>](instance: *mut core::ffi::c_void) {
                if instance.is_null() {
                    return;
                }
                let service = unsafe { &mut *(instance as *mut $service_type) };
                smearor_swipe_launcher_plugin_api::ServicePlugin::start(service);
            }

            static [<VTABLE_ $service_type:snake>]: $crate::ServicePluginVTable = $crate::ServicePluginVTable {
                destroy: [<destroy_ $service_type:snake>],
                on_message: [<on_message_ $service_type:snake>],
                start: [<start_ $service_type:snake>],
            };

            #[stabby::export]
            pub extern "C" fn smearor_service_create(
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

                match <$service_type>::new(config, ffi_context) {
                    Ok(service) => {
                        let container = $crate::ServicePluginContainer {
                            instance: Box::into_raw(Box::new(service)) as *mut core::ffi::c_void,
                            vtable: & [<VTABLE_ $service_type:snake>],
                            vtable_version: $crate::SERVICE_VTABLE_VERSION,
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
