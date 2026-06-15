use crate::FfiCoreContext;
use crate::FfiEnvelope;
use crate::PluginConstructionErrorWrapper;
use abi_stable::RRef;
use abi_stable::StableAbi;
use abi_stable::derive_macro_reexports::RResult;
use abi_stable::std_types::RString;

#[repr(C)]
#[derive(StableAbi)]
pub struct ServiceVTable {
    pub destroy: unsafe extern "C" fn(service: *mut ()),
    pub get_id: unsafe extern "C" fn(service: *mut ()) -> RString,
    pub get_display_name: unsafe extern "C" fn(service: *mut ()) -> RString,
    pub on_message: unsafe extern "C" fn(service: *mut (), message: FfiEnvelope),
}

#[repr(C)]
#[derive(StableAbi)]
pub struct LoadedService {
    pub service_instance: *mut (),
    pub vtable: RRef<'static, ServiceVTable>,
}

impl LoadedService {
    pub fn new<T>(service_instance: T, vtable: RRef<'static, ServiceVTable>) -> Self {
        LoadedService {
            service_instance: Box::into_raw(Box::new(service_instance)) as *mut (),
            vtable,
        }
    }
}

pub type ServiceConstructor =
    unsafe extern "C" fn(config_json: *const i8, config_len: usize, core_context: FfiCoreContext) -> RResult<LoadedService, PluginConstructionErrorWrapper>;
