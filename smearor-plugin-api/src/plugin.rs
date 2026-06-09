use crate::FfiCoreContext;
use crate::FfiWidget;
use abi_stable::RRef;
use abi_stable::StableAbi;
use abi_stable::derive_macro_reexports::ROption;
use abi_stable::derive_macro_reexports::RResult;
use abi_stable::std_types::RString;

#[repr(C)]
#[derive(StableAbi)]
pub struct PluginVTable {
    pub destroy: unsafe extern "C" fn(plugin: *mut ()),
    pub get_id: unsafe extern "C" fn(plugin: *mut ()) -> RString,
    pub get_display_name: unsafe extern "C" fn(plugin: *mut ()) -> RString,
    pub get_icon_name: unsafe extern "C" fn(plugin: *mut ()) -> ROption<RString>,
    pub build_widget: unsafe extern "C" fn(plugin: *mut ()) -> FfiWidget,
    pub on_primary_action: unsafe extern "C" fn(plugin: *mut (), rotation: u32) -> i32,
    pub on_secondary_action: unsafe extern "C" fn(plugin: *mut (), rotation: u32) -> i32,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct LoadedPlugin {
    pub plugin_instance: *mut (),
    pub vtable: RRef<'static, PluginVTable>,
}

pub type PluginConstructor = unsafe extern "C" fn(config_json: *const i8, config_len: usize, core_context: FfiCoreContext) -> RResult<LoadedPlugin, RString>;
