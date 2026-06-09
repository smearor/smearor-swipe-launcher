use abi_stable::StableAbi;

#[repr(C)]
#[derive(StableAbi)]
#[sabi(unsafe_opaque_fields)]
pub struct FfiWidget {
    pub raw_widget: *mut gtk4::ffi::GtkWidget,
}
