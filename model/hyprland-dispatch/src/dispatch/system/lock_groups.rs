use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandLockType;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Locks, unlocks, or toggles locking of window groups.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LockGroupsDispatchMessage {
    pub lock_type: HyprlandLockType,
}

impl TypedMessage for LockGroupsDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::LockGroupsDispatchMessage");
}

impl MessageTopic for LockGroupsDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for LockGroupsDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
