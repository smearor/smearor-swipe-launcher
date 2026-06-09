use abi_stable::StableAbi;
use abi_stable::std_types::RString;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub enum CoreMessage {
    RequestClose,
    TriggerParentMenu,
    EmitNotification { title: RString, body: RString },
    ScrollToPosition { position: f64 },
}
