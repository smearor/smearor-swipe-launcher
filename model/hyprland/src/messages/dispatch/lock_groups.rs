use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandLockType;

use super::workspace::TOPIC_DISPATCH;

/// Locks, unlocks, or toggles locking of window groups.
#[derive(Clone, Debug, Default)]
pub struct LockGroupsDispatchMessage {
    pub lock_type: HyprlandLockType,
}

/// ABI-stable version of `LockGroupsDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct LockGroupsDispatchMessageStabby {
    pub lock_type: HyprlandLockType,
}

impl From<LockGroupsDispatchMessage> for LockGroupsDispatchMessageStabby {
    fn from(value: LockGroupsDispatchMessage) -> Self {
        Self { lock_type: value.lock_type }
    }
}

impl From<LockGroupsDispatchMessageStabby> for LockGroupsDispatchMessage {
    fn from(value: LockGroupsDispatchMessageStabby) -> Self {
        Self { lock_type: value.lock_type }
    }
}

impl TypedMessage for LockGroupsDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::LockGroupsDispatchMessage");
}

impl TypedMessage for LockGroupsDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::LockGroupsDispatchMessageStabby");
}

impl MessageTopic for LockGroupsDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for LockGroupsDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for LockGroupsDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
