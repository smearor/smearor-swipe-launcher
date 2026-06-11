use crate::service::AppLauncherService;
use abi_stable::RRef;
use abi_stable::std_types::RResult;
use abi_stable::std_types::RString;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::LoadedService;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::ServiceVTable;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

pub(crate) mod service;

unsafe extern "C" fn destroy_service(service: *mut ()) {
    if !service.is_null() {
        unsafe {
            let _ = Box::from_raw(service as *mut AppLauncherService);
        }
    }
}

unsafe extern "C" fn get_id(service: *mut ()) -> RString {
    if service.is_null() {
        return RString::from("");
    }
    let service = unsafe { &*(service as *const AppLauncherService) };
    RString::from(service.meta.id.clone())
}

unsafe extern "C" fn get_display_name(service: *mut ()) -> RString {
    if service.is_null() {
        return RString::from("");
    }
    let service = unsafe { &*(service as *const AppLauncherService) };
    RString::from(service.meta.display_name.clone())
}

unsafe extern "C" fn on_message(service: *mut (), message: FfiEnvelope) {
    if service.is_null() {
        return;
    }
    let service = unsafe { &*(service as *const AppLauncherService) };
    service.handle_envelope_message(message);
}

static VTABLE: ServiceVTable = ServiceVTable {
    destroy: destroy_service,
    get_id,
    get_display_name,
    on_message,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn smearor_service_create(
    config_json: *const i8,
    config_len: usize,
    core_context: FfiCoreContext,
) -> RResult<LoadedService, PluginConstructionError> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::DEBUG.into()))
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    let config = match PluginConfig::new(config_json, config_len) {
        Ok(config) => config,
        Err(e) => {
            return RResult::RErr(e);
        }
    };
    let core_context = if core_context.core_obj.is_null() { None } else { Some(core_context) };
    let app_launcher_service = match AppLauncherService::new(config, core_context) {
        Ok(app_launcher_service) => app_launcher_service,
        Err(e) => {
            return RResult::RErr(e);
        }
    };
    RResult::ROk(LoadedService {
        service_instance: Box::into_raw(Box::new(app_launcher_service)) as *mut (),
        vtable: RRef::new(&VTABLE),
    })
}
