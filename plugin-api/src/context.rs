use crate::FfiEnvelope;
use abi_stable::RRef;
use abi_stable::StableAbi;

#[repr(C)]
#[derive(StableAbi, Copy, Clone)]
pub struct CoreContextVTable {
    pub send_message: unsafe extern "C" fn(context: *mut (), message: FfiEnvelope),
}

#[repr(C)]
#[derive(StableAbi, Copy, Clone)]
pub struct FfiCoreContext {
    pub core_obj: *mut (),
    pub vtable: RRef<'static, CoreContextVTable>,
}

impl FfiCoreContext {
    pub fn send_message(&self, message: FfiEnvelope) {
        unsafe {
            (self.vtable.get().send_message)(self.core_obj, message);
        }
    }
}
