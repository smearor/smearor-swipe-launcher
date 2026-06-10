use abi_stable::StableAbi;
use abi_stable::std_types::RString;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct FfiEnvelope {
    pub sender_id: RString,
    pub topic: RString,
    pub payload: RString, // Serialized JSON payload
}
