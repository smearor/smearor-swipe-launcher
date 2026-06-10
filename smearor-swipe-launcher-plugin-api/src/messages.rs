use abi_stable::StableAbi;
use abi_stable::std_types::RString;
use std::fmt::Display;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct FfiEnvelope {
    pub sender_id: RString,
    pub topic: RString,
    pub payload: RString, // Serialized JSON payload
}

impl Display for FfiEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FfiEnvelope {{ sender_id: {}, topic: {}, payload: {} }}", self.sender_id, self.topic, self.payload)
    }
}
