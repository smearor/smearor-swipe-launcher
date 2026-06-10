use crate::FfiCoreContext;
use crate::FfiEnvelope;
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

pub type ServiceConstructor = unsafe extern "C" fn(config_json: *const i8, config_len: usize, core_context: FfiCoreContext) -> RResult<LoadedService, RString>;
